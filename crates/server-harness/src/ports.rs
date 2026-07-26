//! Ephemeral port allocation for concurrently-running server instances.
//!
//! The server binds six ports (auth, logon, base, cell, admin, minigame). Any
//! of them hard-coded means two harness instances collide, which is the
//! "port collisions" half of the wireclient ADR's process-lifecycle risk.
//!
//! # Approach
//!
//! Ask the OS for a free port by binding to port 0, read the assignment back,
//! then close the socket and hand the number to the server.
//!
//! This has a well-known race: between our close and the server's bind, some
//! other process could take the port. It is nonetheless the right trade here —
//! the alternative (passing an inherited listening socket) would require
//! changing the server's bind path purely to serve tests, and the OS does not
//! immediately recycle just-released ephemeral ports. Two mitigations narrow
//! the window that actually bites in practice:
//!
//! 1. **Same bind address as the server.** Probes bind `0.0.0.0`, matching the
//!    server's default `base_host`/`auth_host`. Probing `127.0.0.1` would
//!    "prove" a port free that a `0.0.0.0` bind then fails on.
//! 2. **Same protocol as the server.** UDP ports are probed with a UDP socket
//!    and TCP ports with a TCP listener. TCP and UDP port spaces are
//!    independent, so probing the wrong one proves nothing.
//! 3. **A process-wide reservation set** ([`RESERVED`]) so two harnesses in
//!    one test binary can never be handed the same number, even if the OS
//!    would happily recycle it between the close and the next probe.

use std::collections::HashSet;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, UdpSocket};
use std::sync::{Mutex, OnceLock};

/// Ports handed out by this process, so concurrent harnesses in one test
/// binary never receive the same number twice.
///
/// Never shrinks: a test binary allocates a handful of ports per harness and
/// the set is bounded by test count.
static RESERVED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();

fn reserved() -> &'static Mutex<HashSet<u16>> {
    RESERVED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// How many times to re-probe when the OS hands back a port this process has
/// already reserved. Generous: exhausting this means the ephemeral range is
/// genuinely saturated, which is a real failure worth surfacing.
const MAX_PROBE_ATTEMPTS: usize = 64;

/// Transport a port will be used with. TCP and UDP port spaces are
/// independent, so the probe has to match the eventual bind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortKind {
    Tcp,
    Udp,
}

/// Reserve a free port of the given kind on `0.0.0.0`.
///
/// The port is recorded in this process's reservation set and never handed
/// out again by [`reserve_port`], but nothing stops an unrelated process from
/// taking it — see the module docs.
pub fn reserve_port(kind: PortKind) -> io::Result<u16> {
    for _ in 0..MAX_PROBE_ATTEMPTS {
        let port = probe_free_port(kind)?;

        let mut set = reserved()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if set.insert(port) {
            return Ok(port);
        }
        // Already handed out in this process — the OS recycled it after an
        // earlier probe closed. Probe again; the just-closed socket for this
        // attempt is dropped at the end of the iteration, so the next probe
        // is free to return something else.
    }

    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        format!(
            "could not find a free {kind:?} port after {MAX_PROBE_ATTEMPTS} attempts \
             (ephemeral port range saturated?)"
        ),
    ))
}

/// Bind port 0, read the OS assignment, release.
fn probe_free_port(kind: PortKind) -> io::Result<u16> {
    // 0.0.0.0 matches the server's default bind address. A port free on
    // 127.0.0.1 is not necessarily free on 0.0.0.0.
    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0));
    match kind {
        PortKind::Tcp => TcpListener::bind(addr)?.local_addr().map(|a| a.port()),
        PortKind::Udp => UdpSocket::bind(addr)?.local_addr().map(|a| a.port()),
    }
}

/// The full set of ports one server instance needs.
///
/// Every port the server binds is represented. Leaving one out means it keeps
/// its compiled-in default and collides with a second instance — which is
/// exactly how `minigame_port` (previously not overridable by environment)
/// made concurrent servers impossible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortSet {
    /// Auth service port for BaseApp connections (TCP).
    pub auth: u16,
    /// Auth HTTP port for SOAP client login (TCP).
    pub logon: u16,
    /// BaseApp Mercury listener (UDP).
    pub base: u16,
    /// CellApp listener (UDP).
    pub cell: u16,
    /// Admin REST API — also the readiness probe target (TCP).
    pub admin: u16,
    /// Minigame SmartFoxServer listener (TCP).
    pub minigame: u16,
}

impl PortSet {
    /// Reserve a fresh, non-colliding port for every service.
    pub fn reserve() -> io::Result<Self> {
        Ok(Self {
            auth: reserve_port(PortKind::Tcp)?,
            logon: reserve_port(PortKind::Tcp)?,
            base: reserve_port(PortKind::Udp)?,
            cell: reserve_port(PortKind::Udp)?,
            admin: reserve_port(PortKind::Tcp)?,
            minigame: reserve_port(PortKind::Tcp)?,
        })
    }

    /// All TCP ports, for uniqueness assertions.
    #[cfg(test)]
    fn tcp_ports(&self) -> [u16; 4] {
        [self.auth, self.logon, self.admin, self.minigame]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_ports_are_nonzero() {
        // Port 0 means "let the OS choose" — handing it to the server would
        // bind an arbitrary port the harness then cannot find.
        assert_ne!(reserve_port(PortKind::Tcp).unwrap(), 0);
        assert_ne!(reserve_port(PortKind::Udp).unwrap(), 0);
    }

    /// The core anti-collision property: repeated reservations in one process
    /// never repeat. Removing the `RESERVED` set makes this fail as soon as
    /// the OS recycles a just-closed probe port.
    #[test]
    fn repeated_reservations_never_repeat_within_a_process() {
        let mut seen = HashSet::new();
        for _ in 0..40 {
            let port = reserve_port(PortKind::Tcp).unwrap();
            assert!(
                seen.insert(port),
                "port {port} was handed out twice by reserve_port; the \
                 process-wide reservation set is not holding"
            );
        }
    }

    /// A full set must be internally unique per protocol, or the server binds
    /// the same TCP port twice and the second bind fails.
    #[test]
    fn port_set_tcp_ports_are_mutually_unique() {
        let ports = PortSet::reserve().unwrap();
        let tcp = ports.tcp_ports();
        let unique: HashSet<u16> = tcp.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tcp.len(),
            "TCP ports must be mutually unique, got {tcp:?}"
        );
        // UDP ports share no space with TCP, but must differ from each other.
        assert_ne!(ports.base, ports.cell, "base and cell both bind UDP");
    }

    /// Two independently reserved sets must not overlap — this is the
    /// "concurrent harness runs" case the ADR calls out.
    #[test]
    fn two_port_sets_do_not_overlap() {
        let a = PortSet::reserve().unwrap();
        let b = PortSet::reserve().unwrap();

        let a_tcp: HashSet<u16> = a.tcp_ports().into_iter().collect();
        let b_tcp: HashSet<u16> = b.tcp_ports().into_iter().collect();
        assert!(
            a_tcp.is_disjoint(&b_tcp),
            "two harness instances were handed overlapping TCP ports: {a:?} vs {b:?}"
        );

        let a_udp: HashSet<u16> = [a.base, a.cell].into_iter().collect();
        let b_udp: HashSet<u16> = [b.base, b.cell].into_iter().collect();
        assert!(
            a_udp.is_disjoint(&b_udp),
            "two harness instances were handed overlapping UDP ports: {a:?} vs {b:?}"
        );
    }

    /// A reserved port must actually be bindable — proves the probe releases
    /// the socket rather than holding it, which would make every spawned
    /// server fail to bind.
    #[test]
    fn a_reserved_tcp_port_is_bindable_afterwards() {
        let port = reserve_port(PortKind::Tcp).unwrap();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
        TcpListener::bind(addr).expect(
            "a reserved port must be free for the server to bind; if this fails \
             the probe is leaking its listener",
        );
    }
}
