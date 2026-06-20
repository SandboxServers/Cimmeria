# cimmeria-client-telemetry

Injected DLL that observes the original 2009 `SGW.exe` (CME `EventSignal`
subscription, function hooks, log tees) and ships events to the
`cimmeria-server` telemetry endpoint. Built as a `cdylib` for the **32-bit
client process** plus an `rlib` so the non-`unsafe` layers (queue, uploader,
events) can be unit-tested on the host.

## ⚠️ Build/lint against the i686 target, not the host

Most of this crate — every hook, vtable swap, and ABI shim — lives behind
`#[cfg(all(target_os = "windows", target_arch = "x86"))]`. **Your host x64
toolchain never compiles that code**, and CI's *workspace* clippy job
explicitly excludes this crate, so x64-only checks pass while x86-gated lints
and build errors sail straight through. The single `i686-pc-windows-msvc`
CI job is the only thing that catches them.

Before pushing changes to this crate, run the same checks locally:

```sh
rustup target add i686-pc-windows-msvc   # one-time

cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc \
  --all-targets -- -D warnings
cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc
cargo fmt -p cimmeria-client-telemetry --check
```

A plain `cargo clippy -p cimmeria-client-telemetry` on the host **will not**
exercise the gated code and will give you a false green.

## Layout

- `boot.rs` — `DllMain` + bootstrap-thread plumbing (re-entrancy-safe init).
- `cme.rs` — CME `EventSignal` subscription + the `FakeVtable` static-subscriber shim.
- `hooks/` — the hook techniques (CME subscribers, inline JMP, IAT replace, vtable swap).
- `queue.rs` / `uploader.rs` — the bounded event queue + batched uploader (host-testable).
- `events.rs` — the typed event taxonomy shipped to the server.

See [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../../docs/reverse-engineering/findings/client-instrumentation-hookpoints.md)
and [`client-instrumentation-entry-points.md`](../../docs/reverse-engineering/findings/client-instrumentation-entry-points.md)
for the per-hook address catalog.
