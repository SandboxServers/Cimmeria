# Work packet LI-09: game telemetry never over plain http

Ledger: [README.md](README.md) (F-11 to F-14, D-LI8, D-LI11, D-LI14 to D-LI17).
The other packets are in [work-packets.md](work-packets.md). This packet runs in
wave 2, after LI-06 merges to `main`. It ships as its own PR, behind ship
gate G-LI9. `Host: linux`.

## Rules this packet carries (R1-R9, copied from work-packets.md)

- **R1. Builds go through the lane.** Never run `cargo check`, `build`, `test`,
  `nextest` or `clippy` directly; a hook refuses them. From the worktree root:
  - root crate: `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-launcher-core --all-targets --features test-fixtures -- -D warnings`, then
    `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-launcher-core --features test-fixtures`
  - desktop workspace: `bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --target-dir target/desktop --all-targets -- -D warnings`, then
    `bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --target-dir target/desktop`
  - `cargo fmt` may run directly: `cargo fmt -p cimmeria-launcher-core`, or
    `cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml`.
  - The order is fmt, then clippy, then tests. A lane failure that is not your
    code (exit 28, disk, lock): stop and report `blocked`.
- **R2. Test type** ([TESTING.md](../../../TESTING.md)): unit tests in a
  `#[cfg(test)] mod tests` in the file, or in a sibling `tests.rs` when the file
  would pass 500 lines. Every behavior change needs a test that **fails when
  the change is reverted**. Say in the commit body which test that is.
- **R3. File caps**: 500 lines soft, 700 hard. Split along the seams this file names.
- **R4. Line endings**: files under `docs/` are CRLF. For any `.md`, keep its
  existing endings (check with `file <path>`), and never rewrite a whole file.
  Rust files are LF.
- **R5. Git**: commit locally with an explicit path list. No `git stash`, no
  push, no rebase, no PR. `ops` does those.
- **R6. Scope**: edit only the files your packet names. If a needed file is not
  listed, stop and report `blocked`, naming it.
- **R7. Telemetry**: forwarded launcher targets (`LAUNCHER_EVENT_TARGETS` in
  `engine/src/storage/game_telemetry/launcher_event.rs`) carry closed codes
  only. They carry no paths, error text or URLs.
- **R8. Do not add or upgrade any crate.** No dependency edits and no
  `external/` changes.
- **R9. Docs**: update only the doc rows your packet lists, and name them in
  the commit body.

## LI-09: game telemetry never over plain http

- Host: linux. Branch `launcher-linux-install-telemetry/li-09` from `main`,
  **after LI-06 has merged**; worktree `li-09`.
- Commit: `fix(launcher): send game telemetry over https only, never plain http (LI-09)`
- Rules: R1-R9.
- Lane:
  - root: `-p cimmeria-launcher-core --features test-fixtures`.
  - desktop: `-p cimmeria-launcher-engine`, then `-p cimmeria-launcher-desktop`.
- Why: D-LI11 (owner): "never plain http" covers game telemetry. D-LI14 keeps
  loopback. D-LI15: one upgrade rule, shared with summaries. D-LI16: there is
  no fallback.

**Contract**

```rust
// crates/launcher-core/src/telemetry/endpoint.rs (re-exported as cimmeria_launcher_core::telemetry_endpoint)
/// D-LI8: an `http://` login server on a public DNS name, as the https origin of the
/// same name on 443 ("https://play.example"). `None` for https and other schemes,
/// IP literals, `localhost`, `*.localhost`, single-label names, a trailing dot and
/// credentials. Never an http URL.
pub fn https_upgrade_origin(login_server: &str) -> Option<String>;
impl EndpointPolicy {
    pub fn from_login_servers<'a>(login_server_urls: impl IntoIterator<Item = &'a str>) -> Self; // signature unchanged
    pub fn check(&self, url: &str) -> Result<(), EndpointError>;
}
```

| URL given to `check` | Before LI-09 | After LI-09 |
|---|---|---|
| `https://…` | Ok | Ok |
| `http://localhost…`, `http://127.x.x.x…`, `http://[::1]…` | Ok | Ok (D-LI14) |
| `http://` to the host and port of an http login server | Ok | `InsecureRemote` |
| any other `http://` | `InsecureRemote` | `InsecureRemote` |

**Files**

1. `crates/launcher-core/src/telemetry/endpoint.rs` (218 lines):
   - Add `https_upgrade_origin`. Its body is the check in LI-06's
     `upgraded_from_http_login_server` (read it in
     `crates/launcher/desktop/engine/src/storage/launcher_summary/endpoint.rs`
     on `main`), moved here:
     - `Url::parse`. The scheme must be `http`, with no username and no
       password. Take `host_str()`.
     - Return `None` for `name.parse::<Ipv4Addr>().is_ok()`, for
       `name.starts_with('[')` and for `!upgradable(name)`.
     - Otherwise return `Some(format!("https://{name}"))`.
     - Move `upgradable` here as a private fn, unchanged.
   - `EndpointPolicy` becomes a unit struct:
     `#[derive(Debug, Clone, Default, PartialEq, Eq)] pub struct EndpointPolicy;`.
     - Delete `http_login_origins`, `is_http_login_server` and `origin`.
     - Do not add `Copy`.
   - `from_login_servers` keeps its signature, so its five call sites do not
     change. Its parameter becomes `_login_server_urls`, and it returns `Self`.
     - Its doc comment: "Login servers no longer widen the rule (D-LI11): an
       http login server is reached through [`https_upgrade_origin`], never
       over plain http. Every session still builds its policy here."
   - `check`: delete the `"http" if self.is_http_login_server(..)` arm.
     - If clippy flags `unused_self`, add
       `#[allow(clippy::unused_self)] // the policy is a value sessions carry`.
   - The `InsecureRemote` text becomes: `"telemetry needs an https:// server
     address; {0} is plain http to another machine, so nothing was sent"`.
   - Module docs (lines 1-19):
     - The rule is "`https://` anywhere, plain `http://` only to this machine".
     - Replace the paragraph "Why a login server may take plain http" with
       this: the owner withdrew the 2026-09-29 exception on 2026-10-24
       (launcher-linux-install-telemetry D-LI11). Game telemetry reaches an
       http login server on a DNS name over https on 443, under the same name
       (`https_upgrade_origin`). An IP or single-word login server gets no
       game telemetry; a dotted LAN name is tried over https.
2. `crates/launcher/desktop/engine/src/storage/launcher_summary/endpoint.rs`:
   - The body of `upgraded_from_http_login_server` becomes
     `Self::parse(&format!("{}/api/", https_upgrade_origin(login_server)?))`.
     Import it from `crate::telemetry_endpoint`.
   - Delete the local `upgradable`, and any import it leaves unused.
   - **Do not change its tests.** LI-06's tests must pass unchanged, and that
     proves summaries and game telemetry share one rule.
3. `crates/launcher/desktop/engine/src/storage/game_telemetry/session.rs` (508
   lines; it may grow by **at most 15 lines**):
   - Add, above `auth_base_url`:
     ```rust
     /// The origin game telemetry talks to for `login_server` (D-LI11, D-LI15): an http
     /// login server on a public DNS name becomes `https://<name>` on 443, the rule
     /// summaries use. Anything else is kept as written, and the policy refuses it
     /// unless it is https or loopback. There is no http fallback.
     fn telemetry_origin(login_server: &str) -> String {
         https_upgrade_origin(login_server)
             .unwrap_or_else(|| login_server.trim_end_matches('/').to_owned())
     }
     ```
   - `auth_base_url` returns `format!("{}/api", telemetry_origin(login_server))`.
     `mint_url` becomes `pub(super)`, unchanged otherwise. `transport/mod.rs`
     picks the change up through `auth_base_url`, so refresh moves to https too.
   - `upload_for`: pass `&telemetry_origin(login_server)` to
     `effective_upload_endpoint`, in place of `login_server`.
     - A loopback upload URL advertised to an http login server on a DNS name
       is then replaced with `https://<name>/api/telemetry`.
     - An IP login server still gets `no_https_endpoint`.
   - The comment in `start_kind` (about lines 162-164) becomes: "https
     anywhere, plain http only to this machine; an http login server on a DNS
     name is asked over https (`telemetry_origin`)."
   - Test `a_loopback_upload_from_a_plain_http_login_has_no_usable_address`
     (about line 381): rename it to
     `a_loopback_upload_from_a_plain_http_login_is_replaced_with_its_https_origin`.
     - The result is `Ok("https://play.example/api/telemetry")`.
     - There is exactly one `upload_endpoint_overridden` event, with kind `game`.
4. `…/game_telemetry/transport/mod.rs`: the module doc only, the
   "**Addresses.**" bullet (lines 33-36): "https anywhere; plain http only to
   this machine ([`EndpointPolicy`](…)). An http login server on a DNS name is
   reached over https on 443." No code change.
5. New `…/game_telemetry/session_https_tests.rs`. Declare it in
   `game_telemetry/mod.rs` beside `mod launcher_session_tests;`, with the same
   `#[cfg(test)]`. That declaration is the only edit to `mod.rs`.
6. Some tests may fail because they expect http to a login server to pass. If
   such a test is in `game_telemetry/tests.rs`, `launcher_session_tests.rs`,
   `launcher_session.rs`, or `launcher-core/src/telemetry/{mod.rs,runner.rs}`,
   flip its assertion to the new rule and list the file in the commit body. If
   it is anywhere else, stop and report `blocked`.

**Tests**

In `launcher-core/src/telemetry/endpoint.rs` `mod tests`:

- Replace `plain_http_to_a_login_server_host_and_port_is_accepted` with
  `a_login_server_never_vouches_for_plain_http`. This is **the revert test for
  the policy**.
  - The policy is `with_login_servers(&["http://play.cimmeria.app:8081", "http://lan-server"])`.
  - Each of these gives `InsecureRemote`:
    - `http://play.cimmeria.app:8081/api`
    - `http://PLAY.cimmeria.app:8081/api/telemetry`
    - `http://play.cimmeria.app:8081/api/telemetry/upload-chunk`
    - `http://lan-server/api`
    - `http://lan-server:80/api`
  - `http://localhost:8443/api` and `https://play.cimmeria.app/api` give `Ok`.
  - The test fails when reverted, because the old policy accepts the first
    five URLs.
- Delete `a_login_server_without_a_port_means_port_80`; its URLs are in the
  test above. Keep the other tests as they are.
- `https_upgrade_origin_keeps_the_name_and_drops_port_and_path`: each of
  `http://play.example:8081`, `http://Play.Example:8081/login` and
  `http://play.example` gives `Some("https://play.example")`.
- `https_upgrade_origin_refuses_ips_local_names_and_credentials`: each of these
  gives `None`:
  - `http://127.0.0.1:8081`, `http://192.168.1.10:8081` and `http://[2001:db8::1]:8081`
  - `http://localhost:8081` and `http://game.localhost:8081`
  - `http://sgwserver:8081` and `http://play.example.:8081`
  - `http://user@play.example:8081` and `http://u:p@play.example`
  - `https://play.example` and `ftp://play.example`
- `an_upgraded_origin_passes_the_policy`: for each origin from the first
  upgrade test, `EndpointPolicy::default().check(&format!("{o}/api/auth/dev-session"))`
  is `Ok`.

In the new `session_https_tests.rs`:

- `a_plain_http_login_server_on_a_dns_name_is_asked_over_https`. This is **the
  revert test for the derivation**:
  - `session::mint_url("http://play.example:8081")` is
    `"https://play.example/api/auth/dev-session"`.
  - `session::auth_base_url("http://play.example:8081")` is `"https://play.example/api"`.
- `other_login_servers_keep_their_address`. `auth_base_url` gives:
  - `https://play.example:9443/api` for `https://play.example:9443`
  - `http://localhost:8081/api` for `http://localhost:8081`
  - `http://192.168.1.10:8081/api` for `http://192.168.1.10:8081`
- `an_ip_or_lan_login_server_is_refused_not_sent_in_plaintext`: for
  `http://192.168.1.10:8081` and for `http://sgwserver:8081`,
  `EndpointPolicy::from_login_servers([s]).check(&session::mint_url(s))` is
  `Err(InsecureRemote(_))`. This fails if the policy is reverted.
- `a_plain_http_upload_url_from_the_server_is_refused`: this call returns
  `Err("endpoint_refused")`, and fails when reverted:
  ```rust
  upload_for(&EndpointPolicy::from_login_servers(["http://play.example:8081"]),
      "http://play.example:8081", "http://play.example:8081/api/telemetry", "game")
  ```
- `an_ip_login_server_with_a_loopback_upload_has_no_usable_address`: with the
  login server `http://192.168.1.10:8081` and the advertised URL
  `http://localhost:8443/api/telemetry`, `upload_for` returns
  `Err("no_https_endpoint")`.
- `no_derived_mint_url_is_plain_http_off_this_machine`: build the policy from
  each login server `s` in the `None` list above, plus
  `http://play.example:8081` and `https://play.example`. Then either
  `mint_url(s)` starts with `https://`, or its host is loopback, or the policy
  refuses it.

The commit body names the revert tests:
`a_login_server_never_vouches_for_plain_http`,
`a_plain_http_login_server_on_a_dns_name_is_asked_over_https` and
`a_plain_http_upload_url_from_the_server_is_refused`.

**Docs (rows 34 and 57)**. R4 applies: `docs/` files are CRLF; check the
others with `file`.

- `docs/architecture/dev-session-telemetry.md`, the address list (about lines
  196-211, which starts with "`https://` anywhere;"):
  - Keep two cases: https anywhere, and plain http to loopback.
  - Replace the text from "The third case exists for the login-port mount" to
    "tightened in launcher-telemetry wave 2." with these four points:
    - An http login server on a DNS name is reached as `https://<name>` on
      443, by the rule summaries use (`https_upgrade_origin`).
    - An IP or single-word http login server gets no game telemetry; a dotted LAN name is tried over https.
    - A plain-http upload URL that the server hands back is refused, never
      rewritten.
    - There is no http fallback.
  - Keep the sentence about redirects.
- `docs/operations/telemetry-upload-endpoint.md`, the paragraph "Plain
  `http://play.cimmeria.app:8081` still serves the same five routes…" (about
  line 104). Add: "Launchers from LI-09 on never use it for telemetry. They
  mint, refresh and upload through the https route, so that route, and the
  Caddyfile it depends on (OPS-LT1), carries all game telemetry from those
  launchers."
- `docs/operations/telemetry.md`: find the launcher-side lines that say
  telemetry goes to the login port over plain http (about lines 105-120 and
  165-175). Change them to this rule: the launcher reaches the login port's
  routes only as `https://<name>` on 443, and plain http goes only to this
  machine.
- `crates/launcher/desktop/docs/privacy-inventory.md`, the "Game diagnostics"
  row (line 17). Add: "Sent only over https. An http login server's name is
  reached as `https://<name>` on 443, plain http goes only to this machine,
  and an IP or single-word login server gets no game telemetry; a dotted LAN name is tried over https."
- `crates/launcher/desktop/docs/launcher-telemetry.md` and
  `docs/client/launcher-guide.md`: search each for `plain http`, `login port`
  and `8081`. Where a sentence says game telemetry travels over plain http,
  change it to the rule above. If there is no such sentence, make no edit.
- `docs/analysis/launcher-telemetry/work-packets-coverage.md`, under LT-41,
  directly after LI-06's amendment line, add one line: "Amended by
  launcher-linux-install-telemetry LI-09: `session::upload_for` upgrades an
  http login server on a DNS name before `effective_upload_endpoint`, so
  LT-41's tests reach `DEFAULT_UPLOAD_ENDPOINT` only from an IP or
  single-label http server."
