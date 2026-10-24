# OIDC login: client-side research

Date: 2026-10-19. Read-only research by the game-archaeology-specialist. No code changed, no lab used, no Ghidra or x64dbg session run for this report.

Question: what does it take to replace the SGW client's username and password login with a browser-based OIDC flow (Discord or another provider), given that the owner accepts shipping a client patch?

Short answer: **probably no binary patch at all.** The login screen is plain-text Lua on disk, and the launcher already rewrites one of those files (`LoginInternal.lua`) before every Play. The launcher can run the OIDC flow in the system browser, then hand the client a short-lived token that the client sends through its normal login path. The client SHA-1s whatever is in the password box, so the server receives `SHA1(token)` as a 40-hex "legacy hash" and checks it against a token table before the account password. Nothing in the wire format or the session-key path changes. Three behaviours still need a lab experiment before this can be called verified (see [§ 4](#4-unknowns-and-the-smallest-experiment-for-each)).

Evidence labels used below:

- **Verified (code)**: read in Cimmeria source.
- **Verified (client files)**: read in the shipped client's Lua, layout or TOC files, or found by a string scan of `SGW.exe` / `lua51.dll`.
- **Verified (prior RE)**: stated in a findings doc or project memory with an address, not re-checked today.
- **Inferred**: follows from the above, not observed.
- **Unknown**: needs an experiment.

## 1. The current client login flow

### 1.1 Sources

| Source | What it gave |
|---|---|
| `docs/protocol/login-handshake.md` | Wire format of Phases 1–3 |
| `docs/reverse-engineering/findings/auth-and-crypto-modernization-targets.md` | Function addresses, libcurl/OpenSSL versions, SHA-1 site |
| `docs/architecture/encryption-modernization.md` | Status: server halves of Phases 0–3 done; every client-side patch still pending; nothing exercised against a live 2009 client |
| `.claude/agent-memory/.../project_434_encryption_re_findings.md` | #434 RE summary (libcurl login, SHA-1 site, Mercury encrypter hooks) |
| `crates/auth/src/auth/credentials.rs`, `session_kind.rs` | How the server classifies and checks the credential |
| `crates/launcher-core/src/client_setup/login_servers.rs` | How the launcher writes `LoginInternal.lua` |
| `crates/client-launch/src/` | Suspended launch + telemetry DLL injection |
| Client tree `Working\SGWGame\Content\UI\Startup\` (`Login\Login.lua`, `LoginInternal.lua`, `Login.toc`, `Login.layout`, `ServerSelect\ServerSelect.lua`, `CharacterSelect\CharacterSelect.lua`, `EULA\EULA.lua`) | The UI side of the flow |
| String scans of `Working\Binaries\SGW.exe` and `lua51.dll` | Lua bindings present, Lua libraries present and absent |

### 1.2 UI layer (Verified, client files)

The login screen is **CEGUI with Lua scripts on disk**, not Flash (Scaleform is used for minigames, not for the front end).

- `Login.toc` loads `LoginInternal.lua`, then `Login.lua`, then `Login.layout`. It declares two MachineVariables, `GSavedVar_AccountName` and `GSavedVar_ShouldSaveAccount`. The account name can persist; the password never does.
- Startup order: `EULA.lua` (lines 45 and 97) calls `LoginMod.startup()`. That fills the account name from the saved variable and calls `LoginMod.loadServerSystems()`. `loadServerSystems` is defined by `LoginInternal.lua`; `Login.lua` defines a fallback only if it is nil. The widgets already exist when the scripts run (`Login.lua` calls `Login_VersionText:setText` at top level).
- `LoginMod.onLoginClicked`:
  1. reads `Login_AccountEdit:getText()` and `Login_PasswordEdit:getText()`;
  2. looks up the server URL in `LoginMod.servers[<combo box text>]`;
  3. returns silently if either field is empty;
  4. clears the password box;
  5. calls the native binding **`accountLogin(server, username, password)`**.
- Failure: `LoginWin` subscribes `Events.LoginFailed` to `LoginMod.onLoginFailure(this, window, errorStr, errorNum)`, which shows `PromptMod.showPrompt("<errorNum> - <errorStr>")`. **The text shown comes from the server.**
- Success: `Events.LoginSuccess` → `ServerSelectMod.onLoginSuccess`, which builds the shard list from `getShardCount()` / `getShardInfo(i)`. `selectServer(serverName)` starts Phase 2.
- Character select: **Back** calls `disconnect()` and shows `LoginWin` again. **Change Server** calls the native **`relogin()`**. `docs/known-issues.md` says `relogin()` sends `logOff (0xC2)` and "re-initiates SOAP login". Whether it re-sends the cached password hash is not verified (see [§ 4](#4-unknowns-and-the-smallest-experiment-for-each), U1).
- The password box is `Editbox_2` with `MaskText=True`. The layout sets no `MaxTextLength`, so the CEGUI default applies (very large). A native clamp further down is unknown (U5).
- Other native bindings found as UTF-16 strings in `SGW.exe` (around file offsets 0x154EFC2 and 0x154F408): `accountLogin`, `relogin`, `selectServer`, `getVersionInfo`, `openWebsite`, `openManageAccountWebsite`.

### 1.3 Native layer (Verified, prior RE; addresses are SGW.exe VAs with the image at its preferred base)

| Step | Address | Notes |
|---|---|---|
| `accountLogin` binding → `ServerConnection::logOnBegin` | `0x00DDF580` | Log string `"ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s"` at `0x019cf1f8`; a second variant at `0x019cf248` |
| SHA-1 of the password | inline in `logOnBegin`, SHA-1 ctor `FUN_0040D270` | CryptoPP, then CryptoPP HexEncoder, **uppercase**, 40 chars, stored at `ServerConnection+0x130` |
| Plaintext password kept | `ServerConnection+0x3C` (`std::string`) | Still held after hashing |
| `LoginReplyHandler` ctor | `0x00DDED60` | `curl_easy_init` thunk `0x013A96A0`, `curl_multi_init` `0x013A7970`, `curl_easy_setopt` `0x013A96E0` (×9), `curl_multi_add_handle` `0x013A9040` |
| Pump | `0x00DE10B0` | `curl_multi_perform` `0x013A9210` |
| CURLOPTs set | | `URL` 0x2712, `POSTFIELDS` 0x271F, `HTTPHEADER` 0x2751, `POST` 0x4E, `WRITEFUNCTION` 0x4E2B, `TIMEOUT` 0x0D |
| Response dispatch | | type 0x12 = AccountLogin, 0x14 = ServerSelect (V5 worker-4 checkpoint) |
| Success event registration | `register_NetIn_AccountLoginSuccess` `0x00de04c0` | Fires `Events.LoginSuccess` to Lua |

Libraries: libcurl **7.17.0** linked statically, OpenSSL **0.9.8g** (no TLS 1.2, no SNI), gSOAP 2.7 (XML only).

### 1.4 Wire (Verified, `docs/protocol/login-handshake.md` and server code)

1. **Phase 1**: plain HTTP `POST /SGWLogin/UserAuth` to the auth server (port 8081) with
   `<sgwLogin:SGWLoginRequest SKU="SGW_BETA" AccountName="…" Password="<40 hex SHA1>" ProtocolDigest="<32 hex MD5>"/>`.
   Reply: `SGWLoginSuccess` (AccountId, shard list) plus `Set-Cookie: SID=<40 chars>`; or `SGWLoginError` with an error number and string.
2. **Phase 2**: `POST /SGWLogin/ServerSelection` with `Cookie: SID=…`. Reply: `ServerLocation SessionKey=<64 hex AES-256 key> Port IP BWMailBox` plus `TICKET Ticket=<20 hex>`.
3. **Phase 3**: Mercury `baseAppLogin` = `uint32 accountId` + `uint8 len (20)` + ticket. Ticket TTL 30 s, single use.

**The password never feeds any key.** The Mercury AES-256 key is the server-chosen `SessionKey` from Phase 2. Changing what goes in the password box changes nothing in Phases 2–3.

### 1.5 How the server reads the credential (Verified, code)

`crates/auth/src/auth/credentials.rs`, `classify_credential`:

- A 40-character hex string is a `LegacySha1Hex` and is accepted on any transport.
- Anything else is treated as plaintext and accepted **only over TLS**, 1–128 bytes (`MAX_PLAINTEXT_LEN`).
- AccountName must be 3–20 chars of `[0-9a-zA-Z_-]` (a server rule, not a client one).
- The stored legacy hash is uppercase `SHA1(password bytes)`, unsalted (`crates/wireclient/src/auth.rs` `password_sha1_hex`, and the `session_kind.rs` tests).

### 1.6 A discrepancy to raise (Verified, inference is strong)

The Mercury wire-format draft (`docs/drafts/spec/mercury-wire-format.md`, R16, around lines 1913–1934 and 2477; figure mercury-41) and two memory files (`mercury-section-2-live-capture-findings.md`, `mercury-section-2-track-b-evidence.md`) say `ServerConnection+0x130` holds "a 40-char SHA-1 dispatch-table hash, internal only".

The value read live at that offset was `A94A8FE5CCB19BA61C4C0873D391E987982FBBD3`. That is `SHA1("test")`, the test account's password hash (the same constant appears in `crates/wireclient/src/auth.rs`). So `+0x130` is the hashed password, as the auth-crypto finding §3 says, and the R16 reading is wrong. Recommended: file an issue with `disputed_by` against the R16 claim and correct the two memory files. Not done in this pass (read-only brief).

## 2. Patch strategies, cheapest first

### 2.1 Facts that decide the ranking

- **The launcher already owns `LoginInternal.lua`.** It rewrites the file before every Play (`login_servers.rs`: CRLF, ASCII, rejects quotes and backslashes, entries ≤200 chars). Verified (code).
- **Signed Lua client patches already ship** through the manifest (`data/client-patches/`, for example `005-login-delay` on `eula.lua`; the rule "no CME bytes"; deltas). Verified (code).
- **Filling the login fields from Lua already works.** The lab's `lab_login` sets `Login_AccountEdit` / `Login_PasswordEdit` and clicks `Login_LoginButton`. Verified in practice, with one gotcha: on a slow boot (#1343) the password box read back empty.
- **The client's Lua is sandboxed.** `lua51.dll` is a custom wide-char Lua 5.1. Its base library has `loadstring` and `pcall` but no `dofile`, `loadfile`, `require` or `package`. There is no `io` library. `os` has only `time`, `difftime` and `date` (no `getenv`, `execute`, `clock`, `tmpname`). Verified (string scan); that the functions are truly absent at runtime is Inferred (U7). **Client Lua cannot read the command line, environment variables or arbitrary files.** `SGW.exe` imports `GetCommandLineW`, but no Lua binding exposes the arguments.
- **No anti-debug or anti-tamper** in the client (auth-crypto finding §4). Verified (prior RE).
- **A DLL-injection path exists**: `client-launch` does `CreateProcessW(SUSPENDED)` + `LoadLibraryW` for the opt-in `cimmeria-client-telemetry` DLL, which already has inline, IAT and vtable hook support and a lab `lua_eval`. Verified (code).

### 2.2 Rank 1 — launcher-mediated token, no binary patch (strategy a)

1. The player presses Play. The launcher runs OIDC / OAuth 2.0 authorization code + PKCE in the **system browser** with a loopback redirect (RFC 8252). Discord supports this; any OIDC provider does.
2. The launcher exchanges the code, or (better) posts it to the Cimmeria auth server, which talks to the provider and returns a **Cimmeria login token**: random ASCII, ≥128 bits, short TTL, bound to the account.
3. The launcher writes the account name and token into the launcher-owned `LoginInternal.lua`, inside `loadServerSystems` (or a function it calls), for example:
   - set `Login_AccountEdit` and `Login_PasswordEdit`, then call `LoginMod.onLoginClicked()` or `accountLogin(server, name, token)` directly; or
   - replace `LoginMod.onLoginClicked` so the Login button sends the token. CEGUI Lua subscriptions are by function-name string, so a later redefinition should take effect (Inferred, U6).
4. The client hashes the token like any password and sends `SHA1(token)` as the 40-hex `Password`.
5. The server, on `LegacySha1Hex`, checks a token table for that hash **before** the account's password hash; on a hit it marks the token used, logs the account in, and continues to Phase 2 as today.

Why it is cheapest: the launcher already writes this file; no binary bytes change; the wire is unchanged; the token's length does not matter on the wire (always 40 hex), so the only limits are Lua string handling and the launcher's ≤200-char / no-quote rule.

Costs and caveats:

- **Token on disk.** It sits in `LoginInternal.lua` in the install directory until the launcher rewrites it. Mitigations: short TTL (for example 2–5 min), single redemption, and have the launcher scrub the file once the client has started (whether the client reads it once at startup is U2).
- **`relogin()` and Back.** If the token is single-use and `relogin()` re-sends the cached hash (U1), Change Server fails. Options: accept the same hash for a short session window, or keep the token in a Lua global and re-submit. Back → login screen with an empty password box needs the same treatment.
- **Plain HTTP.** `SHA1(token)` crosses the network in clear and is replayable within its TTL. That is the same exposure class as the existing `SessionKey`; the real fix is the pending TLS Phase 1 client patch, not something OIDC changes. A one-time, short-lived token is still a large improvement on a reusable unsalted password hash.
- **Account name field.** The server rule is 3–20 `[0-9a-zA-Z_-]`. A provider identity (for example a Discord snowflake, 17–19 digits) fits, but a linked Cimmeria account name is friendlier; the server can ignore AccountName for token logins or require it to match.
- **Players without the launcher** keep the normal password path; nothing breaks.

### 2.3 Rank 2 — in-client device-style flow in Lua, no launcher (strategy c, variant)

For players who start `SGW.exe` without the launcher. Shipped as a signed Lua patch (or in `LoginInternal.lua`):

1. Lua sends `accountLogin(server, "<sentinel or account>", "start")`.
2. The server answers `SGWLoginError` whose `ErrorStr` carries a short code and URL (the client already displays server text).
3. Lua overrides `LoginMod.onLoginFailure` to recognise this reply and calls `openWebsite(url)` to open the browser (U3: does it open an arbitrary https URL in the default browser?).
4. The player signs in, comes back and presses Login ("I've signed in"); Lua retries with the code; the server accepts once the browser leg has completed.

Do not use Lua `math.random` for anything secret (time-seeded); let the server mint the code.

### 2.4 Rank 3 — in-process hooks via the injected DLL (strategy b)

Hook `curl_easy_setopt` (`0x013A96E0`) or the `LoginReplyHandler` ctor (`0x00DDED60`) to swap the request body, or `logOnBegin` (`0x00DDF580`) to skip the SHA-1. **OIDC itself does not need this.** It is only worth it for the separate goals of TLS (OpenSSL 0.9.8g cannot do modern TLS) or sending plaintext credentials. The telemetry DLL's hook foundation is the place to do it.

### 2.5 Not viable without a DLL — command line or environment (strategy a, as briefed)

Passing the token on the command line or in an environment variable needs native code to read it, because client Lua has no `os.getenv`, `io` or argument binding (§ 2.1). With a DLL it becomes easy, but then Rank 1 is still simpler.

### 2.6 What is free of a client binary patch

| Approach | Binary patch? | Delivery |
|---|---|---|
| Rank 1 (launcher token) | No | Launcher-written `LoginInternal.lua` + server change |
| Rank 2 (Lua device flow) | No | Signed Lua patch or `LoginInternal.lua` + server change |
| Rank 3 (hooks) | No bytes on disk; code injected at runtime | Telemetry or a new DLL |
| Command line / env | Needs a DLL | — |

## 3. Constraints

| Constraint | Value | Status |
|---|---|---|
| What reaches the server | `SHA1(password box)`, uppercase 40 hex | Verified (prior RE + server code) |
| Token length on the wire | Fixed 40 hex, whatever the token length | Verified |
| Token alphabet | Keep to ASCII `[A-Za-z0-9_-]`: launcher file rule (no quotes, no backslashes, ASCII); wide→narrow conversion of non-ASCII before hashing is unknown | Verified (launcher) / Unknown (client, U5) |
| Token length limit | CEGUI default is large; launcher entry ≤200 chars; native clamp unknown | U5 |
| AccountName | 3–20 `[0-9a-zA-Z_-]` (server); client limit unknown | Verified (server) |
| Does any key derive from the password? | No. The AES-256 Mercury key is the server's `SessionKey` | Verified |
| Server error text shown to the player | `ErrorStr` shown verbatim with `errorNum`; maximum length unknown | Verified (Lua) / Unknown (length) |
| Plaintext password still in memory | `ServerConnection+0x3C` | Verified (prior RE) |
| Transport | Plain HTTP; client OpenSSL 0.9.8g cannot do TLS 1.2 | Verified (prior RE) |

## 4. Unknowns and the smallest experiment for each

All lab items need the user's go-ahead first.

| # | Unknown | Smallest experiment |
|---|---|---|
| U1 | Does `relogin()` re-send `UserAuth` with the cached hash (`+0x130`) or the plaintext (`+0x3C`)? | Lab: log in, enter character select, press Change Server, read the server log for a second `UserAuth` and its `Password` value. Alternative: x64dbg non-freezing BP on `0x00DDF580`. |
| U2 | When does the client read the Login Lua files (startup only, or again on return to the login screen)? | Lab: edit `LoginInternal.lua` after launch, press Back, see whether the change appears; or log `CreateFileW` with the telemetry DLL. |
| U3 | Does `openWebsite(url)` open an arbitrary https URL in the default browser? | One lab `lua_eval`: `openWebsite("https://example.org")`. Ghidra: decompile the binding for a URL allow-list. |
| U4 | Can `accountLogin` be called from `loadServerSystems` at startup, or is it too early? | One lab run with a patched `LoginInternal.lua` that calls it directly; compare with a call after the EULA. |
| U5 | Native length or character checks on username and password; wide→narrow conversion of non-ASCII; maximum `ErrorStr` shown | Ghidra decompile of the `accountLogin` binding and `logOnBegin`; lab test with a 200-char token and a long error string. |
| U6 | Does redefining `LoginMod.onLoginClicked` after subscription take effect (lazy name lookup)? | One lab `lua_eval` that redefines it, then a click. |
| U7 | Are `os` and `math.random` opened at runtime? | One lab `lua_eval`: `return type(os), type(os.getenv), type(math.random), type(io)`. |
| U8 | The second `logOnBegin` log variant (`0x019cf248`): a separate SHA-1 path? | Ghidra: xrefs to the string. |

U1, U2, U4 and U7 can share one lab session.

## 5. Server-side work this implies (for the server design)

- A login-token table: hash (`SHA1(token)`, uppercase hex), account id, provider identity, expiry, used flag; checked before the account password on `LegacySha1Hex`.
- An account ↔ provider-identity link (Discord user id or OIDC `sub` + issuer) and a policy for account names.
- An issuance endpoint the launcher calls after the browser flow (or a server-side redirect handler), and the device-code endpoints if Rank 2 is built.
- A `relogin()` story (window reuse or re-issue), decided after U1.

See the sibling `server-research.md` for the server design.

## 6. Docs to update when this is built

- `docs/protocol/login-handshake.md`: token login as an accepted Phase 1 credential.
- `docs/architecture/encryption-modernization.md`: how OIDC sits beside the TLS / plaintext phases.
- `docs/client/launcher-guide.md`: the browser sign-in step.
- The R16 dispute in `docs/drafts/spec/mercury-wire-format.md` and the two memory files (§ 1.6).
