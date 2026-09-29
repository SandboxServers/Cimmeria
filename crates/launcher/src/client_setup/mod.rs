//! Client setup the launcher does on the player's own files, at the end of
//! every Install / Update and before every launch. Every step is
//! idempotent and touches nothing that is already right.
//!
//! - [`stock_case`]: puts back the stock spelling of the files the patch
//!   sets write (`EULA.lua`, which an earlier launcher renamed to
//!   `eula.lua`, leaving the game with no login screen).
//! - [`login_servers`]: the client finds its login server through
//!   `LoginInternal.lua` (`LoginMod.loadServerSystems`), not through
//!   anything in `SGW.exe`. The launcher writes that file from
//!   [`crate::config::LauncherConfig::login_servers`].
//! - [`aslr`]: the client-patches DLL and the RE addresses assume
//!   `SGW.exe` loads at its image base `0x00400000`, so ASLR is switched
//!   off in its PE header, the same one-byte change "Fix ASLR" makes.
//!
//! This replaces the old `.rdata` "hostname patch": the only ASCII
//! `www.stargateworlds.com` in `SGW.exe` is inside the SOAP namespace
//! `http://www.stargateworlds.com/xml/sgwlogin`, so that patch redirected
//! nothing and corrupted the namespace the auth server expects.

pub mod aslr;
pub mod login_servers;
pub mod stock_case;

use std::path::Path;

pub use aslr::AslrOutcome;
pub use login_servers::LoginServer;
pub use stock_case::Restored;

use crate::install_layout;

/// What [`prepare`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    /// Files renamed back to their stock spelling.
    pub restored_names: Vec<Restored>,
    /// True when `LoginInternal.lua` was (re)written.
    pub login_servers_written: bool,
    pub aslr: AslrOutcome,
}

/// Restore stock file names, write the login-server list and switch ASLR
/// off for the install at `install_dir`.
pub fn prepare(install_dir: &Path, servers: &[LoginServer]) -> std::io::Result<SetupReport> {
    let restored_names = stock_case::restore(install_dir)?;
    let login_servers_written = login_servers::write(install_dir, servers)?;
    let aslr = aslr::disable(&install_layout::sgw_exe(install_dir))?;
    Ok(SetupReport {
        restored_names,
        login_servers_written,
        aslr,
    })
}
