# Fix worknote: `onMailHeaderInfo` sentTime showed Unix epoch 0

> Type: reference. Audience: the social-systems coordinator (cimmeria-23) and PR reviewers.
> Companions: [README.md](../README.md), [ss-m1.md](ss-m1.md), [ss-m4.md](ss-m4.md), [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md) M-Q3.

## Contract

- **Task:** owner playtest bug (2026-09-28, screenshot): a freshly sent mail's Read Message window showed **Sent: "Wed Dec 31st, 1969 @ 7:0 pm"** (Unix epoch 0 in US Eastern) and the inbox showed **Expires: "Soon"**. Find the client's actual `sentTime` semantics, fix the server, and add regression guards.
- **Base:** `origin/main` `cf89f314a`. Branch `fix/mail-sent-time`, worktree `.claude/worktrees/mail-time`.
- **Edited:**
  - `crates/base-methods/src/base/world_entry/methods/mail/headers.rs`: `to_wire` now sends `sent_time_age_secs(now, r.sent_time)` (age, not epoch); new `sent_time_age_secs` helper with the full RE citation; a `sent_time_in_future` warn if a row's `sent_time` is ahead of `now` (clock skew).
  - `crates/base-methods/src/base/world_entry/methods/mail/tests/{mod.rs,sent_time_wire.rs,packets.rs}`: new test file, registered; the wire decoder now captures `sentTime` as `f32` instead of discarding it.
  - `crates/wire/src/cell/mail/mod.rs`: doc comments on `MailHeader::sent_time`/`read_time` and the `onMailHeaderInfo` wire-format doc, citing the disassembly.
  - Docs: `docs/reverse-engineering/findings/mail-wire-formats.md` (M-Q3, MEDIUM → HIGH, corrected), `docs/gameplay/mail-system.md` (the Expiry section and the `MessageHeader` wire table), `docs/reverse-engineering/findings/struct-field-layouts.md` (the `sentTime`/`readTime` row types, which were wrong — `int64`, not `FLOAT`).
- **Read set:** `mail-wire-formats.md` M-Q3/M-Q7 (as it stood before this fix), `crates/wire/src/mercury/game_clock/mod.rs` (ruled out — a different clock domain, ticks since server start, not used by mail), `docs/analysis/crafting/worknotes/cr-02.md` (the game clock's own worknote, confirming mail does not share it), `headers.rs`, `claim.rs` (`unix_now`), the mail `tests/` tree (`packets.rs`, `mod.rs`, `read_scoping.rs` as a pattern reference).

## Root cause (RE)

**Not a game-clock problem.** The CR-02 game clock (`crates/wire/src/mercury/game_clock/`) is a separate BigWorld domain — ticks since server start, used for ability cooldowns and effect timers via `BigWorldTimeComplete`. Mail's date display uses the client's own real system clock (`GetSystemTime`), not that domain at all; the two systems never touch.

Traced the client's header-record constructor with headless Ghidra (`tools/re/ghidra-headless/Probe.java` against the analyzed `SGW.exe` project — no GUI, no MCP bridge; see `.claude/agent-memory/game-archaeology-specialist/headless-ghidra-decompile-workaround.md`). `FUN_00eb5ab0` (`ghidra://SGW.exe@0x00eb5ab0`) is the constructor `Detail__unknown_00e15450` (the `onMailHeaderInfo` wire decoder) calls per header row. Byte-exact disassembly around the date/TTL computation:

```asm
00eb5bc6: FLD float ptr [ESP + 0x3c]      ; param_7 = sentTime
00eb5bd6: CALL 0x012379f6                 ; round(ST0) -> EAX:EDX (64-bit int)
00eb5bea: CALL 0x00eb5a10                 ; FUN_00eb5a10(this, low, high, &this+0xa6)
00eb5bef: MOVSS XMM0,dword ptr [ESP + 0x40]   ; param_8 = readTime (unrelated path)
00eb5bf5: COMISS XMM0,dword ptr [0x017f94b8]  ; readTime >= threshold -> HasBeenRead
00eb5c0d: CALL 0x01237e00                 ; __aulldiv(low, high, 3600, 0)
00eb5c19: MOV dword ptr [ESI + 0xb8],ECX  ; expiresHours = 720 - hours
```

Stack-offset tracing (counting every push/pop from function entry) confirms `[ESP+0x3c]` at `0x00eb5bc6` is the constructor's 7th argument, and the call site in the wire decoder passes the decoded `"sentTime"` field there — not `readTime` (the 8th argument, which only reaches the `HasBeenRead` compare at `0x00eb5bef`).

`FUN_00eb5a10` (`ghidra://SGW.exe@0x00eb5a10`), decompiled in full:

```c
void FUN_00eb5a10(uint param_1, int param_2, LPSYSTEMTIME param_3) {
    GetTimeZoneInformation(&local_ac);
    GetSystemTime(&_Stack_cc);
    SystemTimeToTzSpecificLocalTime(&local_ac, &_Stack_cc, &_Stack_bc);
    SystemTimeToFileTime(&_Stack_bc, &_Stack_d4);           // client's own "now", local time
    lVar2 = __allmul(param_1, param_2, 10000000, 0);        // sentTime * 100ns ticks/sec
    _Stack_d4 -= lVar2;                                     // now MINUS sentTime-as-ticks
    FileTimeToSystemTime(&_Stack_d4, param_3);               // -> "Sent: <date>"
}
```

**`sentTime` is seconds elapsed since the mail was sent — an age — not a Unix epoch timestamp.** The client never receives an absolute send time on the wire; it takes its own current local time and subtracts the wire float (converted to FILETIME ticks). `Mercury__unknown_012379f6` is a misnamed shared MSVC helper (`(unsigned __int64)(float)x` rounding via the x87 stack), called from 60+ unrelated Mercury/GFx/CEGUI sites — not a `Mercury`-specific clock.

Both playtest symptoms trace to the same cause: the pre-fix server sent the raw `sgw_gate_mail.sent_time` (Unix epoch seconds, ~1.7-1.8 billion in 2026) cast to `f32`. The client read that as an age of ~56 years: `now - 56yr` lands within seconds of Unix epoch 0 ("Dec 31 1969"), and the same value divided by 3600 makes `ExpiresHours = 720 - hours` underflow to a large negative number, which `Content/UI/Core/GateMail/GateMail.lua:138` (`elseif msgInfo.ExpiresHours < 2 then ... "Soon"`) renders as "Soon" no matter how fresh the mail is.

`readTime` has no equivalent bug and needed no change: `FUN_00eb5ab0` only ever compares it (`COMISS`/`JC`) against a small float threshold to set `HasBeenRead`; it never reaches `FUN_00eb5a10` or the age/TTL math, so the raw epoch value (0 = unread, non-zero = read since it is trivially `>=` any small threshold) was already correct.

**Why the existing test suite never caught this.** `mail/tests/mod.rs::insert_mail` always stamps `sent_time = 0`. Under both the buggy code (raw epoch) and the fix (age), a stored `0` produces the wire value `0`, which looks fresh either way — the two interpretations only diverge for a non-zero `sent_time`, which none of the ~130 existing mail tests exercised on the wire (they assert on SQL side effects or discard `sentTime`'s bytes when decoding).

Updated `docs/reverse-engineering/findings/mail-wire-formats.md` M-Q3 from MEDIUM to HIGH confidence with the full disassembly. Also corrected `docs/reverse-engineering/findings/struct-field-layouts.md`'s `MailHeader` table, which had `sentTime`/`readTime` typed `int64` (wrong; both are `FLOAT` per `entities/defs/alias.xml:97-98` and the wire decoder) and undocumented semantics — that doc predates the M-Q3 investigation.

## Fix

`headers.rs::to_wire` reads `now = unix_now()` once per header batch and calls a new pure helper:

```rust
pub(super) fn sent_time_age_secs(now: i32, sent_time_unix: i32) -> f32 {
    (now - sent_time_unix).max(0) as f32
}
```

`sent_time: sent_time_age_secs(now, r.sent_time)` replaces the old `r.sent_time as f32`. A row whose stored `sent_time` is ahead of `now` (clock skew) logs `reason="sent_time_in_future"` (WARN, `target: "mail"`, with `entity_id`/`player_id`/`account_id`/`mail_id`/`now`/`db_sent_time`) and clamps the age to 0 rather than sending a negative value. `read_time` is unchanged (still the raw epoch, per the RE above). This is the single site that builds every `mail::MailHeader` from `sgw_gate_mail` (the header list, the one-header refresh after an attachment op, and the new-mail notification all route through it), so the fix covers every mail source (player send, system mail, GM `.mail`, Black Market settlement mail) uniformly.

The server-side `sgw_gate_mail.expires_at` column and its 30-day sweep (`expiry/mod.rs`) are unaffected: that column is server-only, never sent on the wire, and was already computed correctly as `sent_time + 30 days` from the Unix-epoch DB column.

## Tests

| Test | Type | What it guards |
|---|---|---|
| `sent_time_wire::sent_time_age_secs_is_now_minus_sent_time_clamped_at_zero` | unit | the conversion formula and the future-clock clamp |
| `sent_time_wire::header_wire_bytes_carry_the_age_not_the_epoch` | wire-format (byte-exact) | the `sentTime` field's LE bytes at its wire offset equal the age's bytes, not the epoch's |
| `sent_time_wire::fresh_mail_expires_hours_matches_the_client_formula` | unit | models the client's recovered `ExpiresHours = 720 - age_secs/3600` formula (cites `SGW.exe@0x00eb5c06`-`0x00eb5c19`); a fresh mail (age 0) yields 720, a 100-hour-old mail yields 620 |
| `sent_time_wire::request_headers_sends_the_age_not_the_epoch` | live-DB / fan-out byte | inserts a mail with `sent_time` stamped 100 s in the past, requests headers over the real router + wire decoder, asserts the decoded `sentTime` float lands in `90.0..=130.0` (a small age), not billions (the raw epoch) |

The wire decoder (`tests/packets.rs`) previously discarded `sentTime`'s bytes as `let _sent = r.i32();`; it now captures them as `f32` on `Received::HeaderInfo::sent_time: Vec<f32>`, which every other mail test's `Received::HeaderInfo { .. }` pattern already tolerates (all use `..`).

## Regression proof

Reverted the fix in place (`sed` back to `sent_time: r.sent_time as f32,`), ran the live-DB filter, restored:

| Command | Result |
|---|---|
| `bash tools/build-lane/live-db-test.sh sent_time_wire` (fix reverted) | FAIL: `request_headers_sends_the_age_not_the_epoch` — "sentTime must be a small age in seconds, got 1790569500" |
| `bash tools/build-lane/live-db-test.sh sent_time_wire` (fix restored) | 4 passed, 0 failed |
| `bash tools/build-lane/live-db-test.sh mail::` (fix restored, full mail suite) | 170 passed, 0 failed, 5363 skipped (unrelated crates) |

## Commands run

| Command | Exit | Result |
|---|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base-methods -p cimmeria-wire --all-targets` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-base-methods sent_time_wire` | 0 | 4 passed (live-DB test self-skips without `DATABASE_URL`) |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire mail` | 0 | 18 passed |
| `bash tools/build-lane/live-db-test.sh sent_time_wire` | 0 | 4 passed |
| `bash tools/build-lane/live-db-test.sh mail::` | 0 | 170 passed, 5363 skipped |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-base-methods -p cimmeria-wire --all-targets -- -D warnings` | 0 | clean |

No dependency changes; no `cargo hakari` run needed.

## Known gaps

- `PTR_017f94b8` (the `HasBeenRead` float threshold) was not read out of the binary's data section this session (`DATAAT` on it returned no defined data at the probed address); the mechanism (a `COMISS`/`JC` threshold compare) is confirmed, only its exact constant is not. Immaterial to this fix since `read_time` needed no change.
- The "Never" branch for archived mail (`GateMail.lua:137`, a distinct client-side sentinel) is not modeled in the new tests; unrelated to this bug (archived mail's `expires_at` is cleared server-side, a pre-existing and unaffected path per `mail-system.md`'s Expiry section).
- Client Lua source for `GateMail.lua`'s exact `dateTime` format string was not fully re-derived (not needed to confirm the fix; the screenshot's format already matches a `SYSTEMTIME`-derived string built by `FileTimeToSystemTime`).

## Integration edits for the coordinator

- None outside this branch's own commits — this is a self-contained fix packet, no ledger ownership change.
