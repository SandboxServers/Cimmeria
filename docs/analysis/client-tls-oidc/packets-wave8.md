# client-tls-oidc wave 8 and the tail: CI, compose forwarding, OIDC rules, ledger (PR #2031 review)

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract C18.3](contract.md#c183-oidc-token-endpoint-authentication-and-claims-ct-83-ct-84-d-ct24), [wave 6](packets-wave6.md), [wave 7](packets-wave7.md), [wave 9](packets-wave9.md), [wave 10](packets-wave10.md), [ledger](README.md), [review ruling](worknotes/pr2031-ruling.md)

Wave 8 closes the relay's CI gap (F18), forwards every provider secret the examples name (owner item 5), makes the OIDC token-endpoint method configurable (F15) and pins the ID-token claim policy (owner item 2, D-CT24). The tail commits the campaign record into the PR (F16), keeps one deferred UI item, and holds the packets the owner's 2026-10-26 calls released (CT-94, CT-101, CT-102; see [wave 10](packets-wave10.md)).

**Ships as:** every packet merges into `client-tls-oidc/campaign` (`cto-camp`) with `merge-noff.ps1`; still **one PR, #2031**. Wave 8 touches no file of waves 6, 7 or 9 and runs **in parallel with wave 6**, branching at the campaign head current at dispatch.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-85 Relay tests in CI; every provider secret forwarded (F18, item 5) | packet-coder | none | - |
| CT-83 OIDC token-endpoint method (F15) | packet-coder | none | - |
| CT-84 ID-token claim policy (D-CT24) | packet-coder | none | CT-83 merged |

**Merge order:** CT-85, CT-83, CT-84. Dispatch CT-85 and CT-83 at once; CT-84 branches at the campaign head after CT-83 merged (both edit `oidc.rs`).

**After the wave:** the wave review loop on `cto-camp`. No sentinel ids (no live-DB test).

## CT-85 Relay tests in CI; every provider secret forwarded (F18, item 5)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-85-ci-relay-compose` · **Worktree:** `cto-85`
- **Commit:** `ci(launcher): run the relay's tests; compose forwards every provider secret (CT-85)`
- **Rules:**
  - R1: PowerShell only, never the Bash tool; start every command with `Set-Location <repo>\.claude\worktrees\cto-85;`; absolute paths for every file write.
  - R2: every cargo call through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground; never cargo directly; never `--target-dir` in a local command (the workflow's `--target-dir target/desktop` is CI-only and stays); read the lane log under `logs/cto-85/` before rerunning; at most 5 fix rounds per check.
  - R3: edit only the files below; `git add` explicit paths; commit with the subject above and the Haiku attribution lines from work-packets.md; no `git stash`, rebase or push.
  - R5: `docs/**/*.md` is CRLF. `docker/*.yml`, `docker/.env.example`, `.github/workflows/*.yml` and `config/*.example` keep their current endings (check before and after with `([IO.File]::ReadAllText('<abs>') -match "\r\n")`).
  - R7: no real secret, host or address; example values stay empty.
  - Do not edit `crates/server/src/main.rs` (another campaign edits it).
- **The defects.** F18: `launcher-desktop.yml` tests and lints the engine (l.111, l.113) and the shell (l.136, l.138) but never `cimmeria-launcher-relay`, a separate package whose 14 unit tests (`crates/launcher/desktop/relay/src/tests.rs`) run nowhere. Item 5: `docker/compose.yml` forwards `CIMMERIA_IDP_DISCORD_SECRET` and `CIMMERIA_IDP_GOOGLE_SECRET` (l.118-119); the opt-in overlay `compose.auth-proxy.yml` adds `CIMMERIA_IDP_OIDC_SECRET` (l.50-51) and repeats Discord; any other name in `client_secret_env` never reaches the container, and the provider is skipped as `secret_missing`.

Files:

1. `.github/workflows/launcher-desktop.yml`, after the engine test step (l.112-113), two steps in the same form:

   ```yaml
         - name: Check launcher relay
           run: bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-relay --target-dir target/desktop --all-targets -- -D warnings
         - name: Test launcher relay
           run: bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-relay --target-dir target/desktop
   ```

   Check the other desktop workflows that run engine tests (`launcher-desktop-macos.yml`, `launcher-desktop-linux.yml`, `launcher-desktop-windows-arm64.yml`, `launcher-release.yml`): where one runs `-p cimmeria-launcher-engine` tests, add the relay test step there too; where none does, leave it.
2. `docker/compose.yml` (l.118-119): forward all three, `CIMMERIA_IDP_DISCORD_SECRET`, `CIMMERIA_IDP_GOOGLE_SECRET`, `CIMMERIA_IDP_OIDC_SECRET`, each `"${NAME:-}"`, with a comment: a provider with another `client_secret_env` name needs that name added here or in a `compose.override.yml`.
3. `docker/compose.auth-proxy.yml` (l.50-51): remove the two secret lines (the base file forwards them now) and say so in the header comment (l.28-30).
4. `docker/.env.example` (l.53-59 and l.74-81): one commented line per name in one block, `# CIMMERIA_IDP_OIDC_SECRET=` included; drop the duplicated Discord line of the overlay block.
5. `crates/identity/src/deploy_env_tests.rs` (new, `#[cfg(test)] mod deploy_env_tests;` in `crates/identity/src/lib.rs`): a unit test that reads `include_str!("../../../docker/compose.yml")`, `include_str!("../../../docker/.env.example")` and `include_str!("../../../config/auth-providers.toml.example")`, collects every `client_secret_env = "<NAME>"` in the example and every `CIMMERIA_IDP_<X>_SECRET` in `.env.example`, and asserts each name appears in `compose.yml` as `<NAME>: "${<NAME>:-}"`.
6. Docs: `docs/operations/auth-https.md`, where it lists the provider secrets: one sentence, a provider whose `client_secret_env` is not one of the three needs the name forwarded in `docker/compose.yml` or an override file. CRLF.

Tests:

| Test | Type | Fails on revert because |
|---|---|---|
| `every_example_provider_secret_reaches_the_container` | unit | Today `CIMMERIA_IDP_OIDC_SECRET` (in `.env.example`) is missing from `compose.yml`. |
| CI: the relay's 14 tests run | CI | Locally: run the relay check below and confirm its log lists 14 passed tests; quote the count in the report. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; D1; D2 `cimmeria-launcher-relay`; D3 `cimmeria-launcher-relay` (no filter; the count must be 14 or more). **Docs rows:** `docs/operations/auth-https.md` (item 6).

## CT-83 OIDC token-endpoint method (F15)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-83-oidc-auth-method` · **Worktree:** `cto-83`
- **Commit:** `fix(identity): OIDC sends client_secret_basic by default, post when configured (CT-83)`
- **Rules:** R1 (worktree `cto-83`), R2 (`logs/cto-83/`), R3 as in CT-85. R4: `config.rs` is 646 lines: first move its inline `mod tests` (l.394 to the end) to a new `crates/identity/src/config_tests.rs` with `#[cfg(test)] #[path = "config_tests.rs"] mod tests;`, unchanged, as your first edit, then make the change; `oidc.rs` (301) stays under 400; `oidc_tests.rs` is 433: put new tests in a new file. R5: docs CRLF. R6: unit tests with `wiremock`, like `oidc_tests.rs`. R7: test secrets are the existing test constants; never log the secret or the header. R8: names from the excerpt.
- **Contract excerpt (C18.3, first two bullets):** provider setting, `kind = "oidc"` only: `token_endpoint_auth_method = "client_secret_basic" | "client_secret_post"`, default `client_secret_basic`. Any other value is a config error at startup. `basic` sends `Authorization: Basic base64(form_urlencode(client_id) ":" form_urlencode(client_secret))` (RFC 6749 2.3.1) and keeps `client_id` but no secret in the form; `post` sends `client_id` and `client_secret` in the form, as today. Discord and dev-mock ignore the key. Discovery reads `token_endpoint_auth_methods_supported` (optional); when present and lacking the configured method, `prepare` (before the browser opens) fails with `ProviderError::BadResponse("auth_method_unsupported")` and a WARN naming the provider id and the method. Absent metadata is not refused.

Files:

1. `crates/identity/src/config.rs`:
   - `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)] pub enum TokenAuthMethod { #[default] ClientSecretBasic, ClientSecretPost }` with `pub fn as_str(self) -> &'static str` (`"client_secret_basic"` / `"client_secret_post"`).
   - `RawProvider` gains `token_endpoint_auth_method: Option<String>`; `ProviderConfig` gains `pub token_auth: TokenAuthMethod` (doc line).
   - In `load`, for `ProviderKind::Oidc`: `None` or `"client_secret_basic"` is Basic, `"client_secret_post"` is Post, anything else `ConfigError::UnknownAuthMethod(raw_id)` (`#[error("provider {0}: token_endpoint_auth_method must be client_secret_basic or client_secret_post")]`). Other kinds: Basic, the key ignored.
2. `crates/identity/src/providers/oidc_keys.rs` `Discovery` (l.25-43): add `pub auth_methods: Option<Vec<String>>`, read from `token_endpoint_auth_methods_supported` (`#[serde(default)]` on the raw struct).
3. `crates/identity/src/providers/oidc.rs`:
   - Field `token_auth: TokenAuthMethod` from the config; fix the `client_secret` field's doc (l.36).
   - `fn check_auth_method(&self, doc: &Discovery) -> Result<(), ProviderError>`: when `doc.auth_methods` is `Some(list)` and `list` lacks `self.token_auth.as_str()`, `tracing::warn!(provider = %self.id.as_str(), method = self.token_auth.as_str(), reason = "auth_method_unsupported", "oidc issuer does not list the configured token endpoint auth method")` and `Err(ProviderError::BadResponse("auth_method_unsupported"))`.
   - `prepare` (l.223): `let doc = self.discovery(Instant::now()).await?; self.check_auth_method(&doc)`. `exchange`: call `check_auth_method(&doc)` after its `discovery` too (the cache may have refreshed).
   - `exchange` form (l.265-272): `client_id` always; `client_secret` only for Post. For Basic, `.header(reqwest::header::AUTHORIZATION, basic_credentials(&self.client_id, self.client_secret.expose()))` with

     ```rust
     /// RFC 6749 2.3.1: each part form-urlencoded, then `id:secret` in standard base64.
     fn basic_credentials(id: &str, secret: &str) -> String {
         let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
         format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", enc(id), enc(secret))))
     }
     ```
4. `crates/identity/src/providers/oidc_tests.rs`: the existing tests keep passing with the Basic default (their `/token` mock matches only method and path); make the helpers the new file needs `pub(super)` (`start_issuer`, `serve_jwks`, `oidc_provider`, `good_claims`, `serve_token`, `TestKey`) and give `oidc_provider` a sibling `oidc_provider_with(issuer, extra_toml: &str)` that appends lines to the provider table.
5. `crates/identity/src/providers/oidc_auth_tests.rs` (new), `#[cfg(test)] #[path = "oidc_auth_tests.rs"] mod auth_tests;` in `oidc.rs`; it uses `super::tests::*`.
6. `config/auth-providers.toml.example`, the Google block (l.44-52): a commented `# token_endpoint_auth_method = "client_secret_basic"   # or "client_secret_post"; oidc only`.
7. `docs/technical/launcher-sign-in.md`, the OIDC provider paragraph: one sentence on the setting, its default and the startup refusal. CRLF.

Tests (unit):

| Test | Fails on revert because |
|---|---|
| `basic_default_sends_the_secret_in_the_authorization_header` (the `/token` mock matches `header("authorization", <expected Basic value>)` and a closure matcher that the body has no `client_secret=`; exchange succeeds) | Today the secret goes in the body and no header is sent, so the mock answers 404 and exchange fails. |
| `post_sends_the_secret_in_the_form` (`token_endpoint_auth_method = "client_secret_post"`; body has `client_secret=`, no `authorization` header) | Guards the explicit post path. |
| `basic_credentials_form_encode_each_part` (`basic_credentials("a:b", "s p&")` equals `Basic ` + base64 of `a%3Ab:s+p%26`) | Guards RFC 6749 encoding. |
| `prepare_refuses_a_method_the_issuer_does_not_list` (discovery lists only `client_secret_post`, config default; `prepare` is `BadResponse("auth_method_unsupported")`; the WARN row carries the reason) | Today discovery ignores the field and `prepare` succeeds. |
| `absent_metadata_is_not_refused` (no `token_endpoint_auth_methods_supported`, `post` configured; `prepare` and exchange succeed) | Guards against refusing issuers that omit the field. |
| `unknown_auth_method_is_a_config_error` and `discord_ignores_the_auth_method_key` (in `config_tests.rs`) | The key is new. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`. **Docs rows:** `docs/technical/launcher-sign-in.md` and `config/auth-providers.toml.example` (items 6, 7).

## CT-84 ID-token claim policy (D-CT24)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-84-oidc-claims` · **Worktree:** `cto-84` (branch at the campaign head after CT-83 merged)
- **Commit:** `fix(identity): OIDC requires iat and checks azp (CT-84)`
- **Rules:** R1 (worktree `cto-84`), R2 (`logs/cto-84/`), R3 as in CT-85. R4: `oidc.rs` stays under 400; new tests in a new file. R5: docs CRLF. R6: unit tests signing tokens with the existing `TestKey`. R7: never log the token or a claim value; the reason string is the only field.
- **Contract excerpt (C18.3, third bullet, D-CT24):** `iat` is required and refused when more than 60 s after now (`ProviderError::Token("iat_future")`); a missing `iat` is `Token("missing_claim")`. `azp` is required when `aud` has more than one value (`Token("azp_missing")`); `azp`, when present, must equal the client id (`Token("azp_mismatch")`).

Files:

1. `crates/identity/src/providers/oidc.rs` `Claims` (l.59-73): add `iat: Option<u64>`, `azp: Option<String>`, `aud: serde_json::Value` (a string or an array; `jsonwebtoken` has already checked membership).
2. `verify` (l.123-173). **Amendment (consolidated ruling, C09 / review A-L3):** in the `Validation` block (l.138-142) add `validation.validate_nbf = true;` with a comment that the 60 s leeway applies; without it the `ImmatureSignature` to `not_yet_valid` mapping (l.191) is dead code. Then, after `decode` and before the nonce check:

   ```rust
   let iat = claims.iat.ok_or(ProviderError::Token("missing_claim"))?;
   if iat > jsonwebtoken::get_current_timestamp() + LEEWAY_SECS {
       return Err(ProviderError::Token("iat_future"));
   }
   let audiences = match &claims.aud { serde_json::Value::Array(list) => list.len(), _ => 1 };
   match claims.azp.as_deref() {
       None if audiences > 1 => return Err(ProviderError::Token("azp_missing")),
       Some(azp) if azp != self.client_id => return Err(ProviderError::Token("azp_mismatch")),
       _ => {}
   }
   ```

   Leave `set_required_spec_claims` as it is: `jsonwebtoken` checks only the names it knows there, so the `Option` plus the explicit check is what makes `iat` required. A comment says so and cites OIDC Core 3.1.3.7.
3. `crates/identity/src/providers/oidc_tests.rs` `good_claims` (l.133): add `"iat": <now as u64>` (`jsonwebtoken::get_current_timestamp()`), so the existing tests stay valid.
4. `crates/identity/src/providers/oidc_claims_tests.rs` (new), `#[cfg(test)] #[path = "oidc_claims_tests.rs"] mod claims_tests;` in `oidc.rs`; it uses `super::tests::*` (CT-83 made the helpers `pub(super)`).
5. `docs/technical/launcher-sign-in.md`, the OIDC paragraph: one sentence with the claim policy (iat required, 60 s skew; nbf checked with the same leeway; azp required with several audiences and equal to the client id). CRLF.

Tests (unit; each signs `good_claims` with one change and runs exchange or `verify`):

| Test | Fails on revert because |
|---|---|
| `a_token_without_iat_is_refused` (`Token("missing_claim")`) | Today `iat` is not read. |
| `an_iat_five_minutes_ahead_is_refused` (`Token("iat_future")`) | Same. |
| `an_iat_thirty_seconds_ahead_is_accepted` | Guards the skew allowance. |
| `a_token_with_nbf_five_minutes_ahead_is_refused` (`Token("not_yet_valid")`; amendment) | Today `nbf` is never checked. |
| `a_token_with_nbf_thirty_seconds_ahead_is_accepted` (amendment) | Guards the leeway; it also settles whether the pinned `jsonwebtoken` applies `leeway` to `nbf`. |
| `two_audiences_without_azp_are_refused` (`aud: [client_id, "other"]`) | Today `azp` is not read. |
| `two_audiences_with_azp_equal_to_the_client_id_are_accepted` | Guards the normal multi-audience case. |
| `an_azp_naming_another_client_is_refused` (single `aud`, `azp: "other"`) | Today `azp` is ignored. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`. **Docs rows:** `docs/technical/launcher-sign-in.md` (item 5).

## Tail

These run after waves 6 to 9 have merged into `cto-camp`, before the campaign review.

### CT-92 Campaign record in the PR (F16)

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `client-tls-oidc/CT-92-ledger` · **Worktree:** `cto-92` (branch at the campaign head after the last wave 9 packet merged)
- **Commit:** `docs(client-tls-oidc): commit the campaign ledger and contract; fix the guide's links (CT-92)`
- **Rules:** R1 (worktree `cto-92`), R3 as in CT-85; no cargo. R5: every `docs/**/*.md` CRLF; check each file you write. R7: no IPs, hosts, account names or secrets in any committed ledger file; grep the copied files for them before committing. R9: never hand-edit a `<!-- gen:NAME -->` block.
- **What:** the ledger exists only in the main checkout, untracked. Copy `<repo>\docs\analysis\client-tls-oidc\` (README, contract, work-packets, packets-wave*.md, RESUME, worknotes) into the worktree's `docs/analysis/client-tls-oidc/`, keeping the two worknotes already tracked there. Then:
  1. `docs/technical/launcher-sign-in.md` l.6-7: the links to README, contract and work-packets resolve in the tree; fix any anchor that does not.
  2. In the same guide: the revocation model in one short section (what `revoke_account` reaches: refresh family, access tokens, launch tickets and the handoffs they issued; that an established game connection is not cut), the `storage_warning` values and their player texts (CT-88, CT-91), the `resume` command (CT-89), and which server sign-in uses (CT-90).
  3. Reconcile the contract's stale lines against the merged code (names only; no new design): every `C<n>` the source cites must exist.
  4. A check: `git ls-files docs/analysis/client-tls-oidc` lists README, contract, work-packets and RESUME; quote the output in the report.
  5. **Amendment (consolidated ruling, all CRLF).** In the revocation-model section of `docs/technical/launcher-sign-in.md`, add:
     - the kill switch blocks new sign-ins, while a live access token can still mint a ticket for up to 15 min; account disable is enforced at each use (C11);
     - reuse revokes the reused family and the account's ticket, and other devices stay signed in (C03);
     - `/link/existing` limits, the 15 min lockout and the link-code cap (C07);
     - the OIDC age gate does not apply until D-CT29 (C10);
     - the stored-email behavior as CT-94 left it (C01) and the ticket lifetime as CT-102 left it (C05);
     - pending-store eviction (C14).
     In `docs/operations/auth-https.md`: until the list is narrowed (D-CT22 on the colo; the default is loopback only after CT-101), any container on a trusted network can forge `X-Forwarded-*`. Note that the SOAP disabled answer (error 5) is legacy parity (C08). The full list is in the [consolidated ruling](packets-wave10.md).
- **Docs rows:** `docs/technical/launcher-sign-in.md`, `docs/operations/auth-https.md`, the campaign folder.

### CT-93 Reauth provider chooser (Deferred)

`account-view.ts` `signAgainPress` (l.173-174) signs in again with the most recently used provider; with none known it now shows a status text (CT-91) but offers no choice. A chooser listing the account's linked providers is a follow-up, not a review fix. Status **Deferred**; not dispatched in this campaign.

### CT-94 Stored email authority (D-CT23 = (a), decided 2026-10-26)

Owner item 1. The owner chose option (a): on every successful sign-in, refresh or clear the stored `email` and `email_verified_at`, and have `email_candidates` accept only identities whose provider is `trusted_email` in the current config. The packet spec, sentinels (`0x7000_EBC0..0x7000_EBFF`) and tests are in [wave 10](packets-wave10.md#ct-94-stored-email-authority-c01-d-ct23--a). Status **Ready**; it merges before CT-92.

### CT-100, CT-101, CT-102

CT-100 (Deferred), CT-101 (D-CT26 = (b)) and CT-102 (D-CT27 = (b)) are specified in [wave 10](packets-wave10.md). CT-101 and CT-102 merge before CT-92.