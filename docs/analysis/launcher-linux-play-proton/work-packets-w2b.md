# Work packets: launcher-linux-play-proton (wave 2 additions from the PP-S1 ruling)

Ledger: [README.md](README.md). Rules R1-R9, the contract C-1..C-6 and
§ Dispatch: [work-packets.md](work-packets.md); R10 and C-7:
[work-packets-w1b.md](work-packets-w1b.md). Ruling: [worknotes/PP-S1-ruling.md](worknotes/PP-S1-ruling.md).
**Host: linux** throughout. `D/`, `E/`, `S/`, `LC/` as defined there.

Wave 2 merge order (whole wave): PP-05, PP-06, PP-07, PP-15, PP-16, PP-08,
PP-09. PP-S2 runs beside it (no code). PP-08W is released by the PP-S2
ruling and merges after PP-08.

## C-8 Native DirectX (PP-15, PP-16)

```rust
// LC/unpack/mod.rs, #[cfg(any(not(windows), test))]
/// One named member of one single-cabinet file, MSZIP, stored or LZX. Never
/// used by the install path, which stays MSZIP/stored-only (D-PP21).
pub fn extract_cab_member(cab: &Path, member: &str, max_bytes: u64) -> Result<Vec<u8>, UnpackError>;

// E/linux_proton/directx.rs
pub struct Pinned { pub cab: &'static str, pub dll: &'static str, pub bytes: u64, pub sha256: &'static str }
pub const DLLS: [Pinned; 2] = [
    Pinned { cab: "Nov2008_d3dx9_40_x86.cab", dll: "d3dx9_40.dll", bytes: 4_379_984,
             sha256: "16fd909aeb68d0d1aca8529dc7f78880b97d6649d70ce8d03a2c858bc28e216b" },
    Pinned { cab: "APR2007_xinput_x86.cab", dll: "xinput1_3.dll", bytes: 81_768,
             sha256: "8d540d484ea41e374fd0107d55d253f87ded4ce780d515d8fd59bbe8c98970a7" },
];
pub const OVERRIDES: &str = "d3dx9_40=n;xinput1_3=n";
pub fn ensure(prefix: &Prefix, game_root: &Path) -> Result<(), ProtonError>;
```

### PP-S2: Second Deck spike (hands-on, owner present)

**Agent:** `claude --agent hands-on` on the Deck, Desktop Mode, owner present
for steps B, C and F. **Host:** linux. **Depends:** none (runs beside wave 2).
**Output:** `<pp-integration>/.claude/reports/launcher-linux-play-proton/PP-S2-findings.md`;
the orchestrator copies it to `worknotes/PP-S2.md`. No repo source change.
Mark every row **tested** or **untested**, record each command, exit code and
the evidence file.

Reuse PP-S1's setup: `~/pp-spike/bin` (the same four pieces and SHA-256s),
the helper scripts `s1.sh`, `s3.sh`, `s7.sh`, the launcher-installed game
copy, `C=~/.local/share/Steam`, `STEAM_COMPAT_CLIENT_INSTALL_PATH=$C`. "DX
changes" = copy `d3dx9_40.dll` and `xinput1_3.dll` (7z from the two cabs,
pins in C-8) into `<compat>/pfx/drive_c/windows/syswow64/` and set
`WINEDLLOVERRIDES="d3dx9_40,xinput1_3=n"`. "Profile" = the prefix's
`pfx/drive_c/users/steamuser/Documents/My Games/Firesky/SGWGame/`.

- **A. Why the PhysX probe fails.** Fresh prefix `compat-s2a` (GE11, `s1.sh`),
  then `s7.sh` with `WINEDEBUG=+relay,+loaddll` (stderr to a file). Record the
  argument and return value of `GetFinalPathNameByHandleW`, `SetDefaultDllDirectories`,
  `AddDllDirectory` / `LdrAddDllDirectory` and the `LoadLibraryExW` of
  `PhysXLoader.dll` (grep the relay log). The worker's probe code is
  `D/runtime-probe/src/windows.rs:81-118`; its four failure points are
  85 (canonicalize), 86 (not a dir / no `SGW.exe`), 92-96, 97-100. Name which
  one fires and with which path string (is it `\\?\Z:\...`?). Run `s7.sh`
  a second time in the same prefix and record the result.
- **B. First Play with no profile.** `compat-s2b` = `cp -a compat-ge11-dx`;
  move its Profile folder aside. B1: run the step-5 request (both DLLs);
  record whether the window draws without a touch, then the files the game
  wrote into Profile (names, sizes, the `windowedMode` line, the
  `[WinDrv.WindowsClient]` block). B2: move Profile aside again; create only
  `SavedSystemOptions.xml`, reduced from B1's file to its root element plus the
  single `groupName="video"` `windowedMode` line with `valueBool="true"`;
  run again; record draw/no draw and what the game wrote back. B3: restore
  B1's files, set `windowedMode` true and `StartupResolutionX/Y` 1024/768
  (the launcher default); run; record draw/no draw and the X geometry.
- **C. Other Protons with the DX changes.** For Proton Experimental, Proton
  11.0 and Proton 9.0: fresh prefix, `s1.sh`, PhysX via `s7.sh`, the DX changes,
  B1's profile with `windowedMode` true, then steps 3-5 of PP-S1. Pass/fail each.
- **D. Optional, xinput.** In `compat-s2b`, set `WINEDLLOVERRIDES="d3dx9_40=n"`
  and delete the native `xinput1_3.dll`; does the login screen render?
- **F. Fullscreen.** In `compat-s2b` with B1's profile: `windowedMode` false and
  `StartupFullscreen=True`, args without `-windowed`. Does the game draw
  without a touch, and does Alt+Tab bring the launcher back?

Rulings the findings must state: the failing probe call (A); whether a
seeded partial profile works (B2) and whether 1024x768 draws (B3); a pass
table per Proton (C); xinput needed or not (D); fullscreen usable (F).
Then the architect writes `worknotes/PP-S2-ruling.md`, which specs PP-08W and
fills PP-17 step 3.

### PP-15: Single-member cabinet extract with LZX

**Agent:** rust-gameserver-dev. **Host:** linux. **Depends:** PP-20 and PP-25
merged (they touch `LC/unpack/`). **Branch:** `launcher-linux-play-proton/pp-15`, worktree `pp-15`.
**Commit:** `feat(launcher): read one member out of an LZX or MSZIP cabinet (PP-15)`.

Files: `crates/launcher-core/Cargo.toml` (add `lzxd = "0.2"` under
`[dependencies]`), `Cargo.lock` **and** `crates/launcher/desktop/Cargo.lock`
(both workspaces hold `cimmeria-launcher-core`), `LC/unpack/mod.rs` (the
C-8 wrapper only, under 500 lines), `LC/unpack/cab_portable/mod.rs` (one
`pub(super) mod single;` line), `LC/unpack/cab_portable/format.rs` (keep the
raw `typeCompress` on `Folder`), new `LC/unpack/cab_portable/single.rs`,
new `LC/unpack/cab_portable/single_tests.rs`.

1. Lockfiles: after editing `Cargo.toml`, run once **without** `--locked`:
   `bash tools/build-lane/lane.sh cargo check --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-core`
   and `bash tools/build-lane/lane.sh cargo check -p cimmeria-launcher-core`
   (the root workspace). Commit both lockfiles. Every later check uses `--locked`.
   Read the fetched crate's `src/lib.rs` (`~/.cargo/registry/src/*/lzxd-*/`)
   for the exact API; it is expected to be
   `Lzxd::new(WindowSize::..)` and `decompress_next(&mut self, chunk: &[u8], output_len: usize) -> Result<&[u8], DecodeFailed>`.
2. `format.rs`: `Folder` gains `pub(super) raw_type: u16`, set where
   `Compression::from_type` is called. Nothing else changes; the install
   path's LZX/Quantum refusal and its test
   `lzx_and_quantum_folders_are_refused_by_name` stay exactly as they are.
3. `single.rs` `pub(super) fn extract_member(cab: &Path, member: &str, max_bytes: u64) -> Result<Vec<u8>, UnpackError>`:
   open the file, `format::parse`; find the one non-continuation file whose
   name equals `member` ignoring ASCII case (zero or two matches → `UnpackError::Cab`);
   refuse a folder that continues to or from another cabinet; refuse a file
   larger than `max_bytes` before decoding. Decode that folder's blocks in
   order from its first block (read headers with the existing
   `format::read_block_header`): stored → copy; MSZIP → the existing
   `mszip::Mszip`; LZX → one `lzxd::Lzxd` per folder, window from
   `(raw_type >> 8) & 0x1F` (15..=21 → `WindowSize::KB32`..`MB2`, anything
   else → `UnpackError::Cab`), `decompress_next(block, uncompressed_len)`;
   Quantum or unknown → `UnpackError::Cab`. Stop once the member's
   `offset + size` is decoded; return exactly its bytes.
4. `unpack/mod.rs`: the C-8 `pub fn extract_cab_member` that calls it.

Tests (`single_tests.rs`, unit, using the MSZIP and stored builders in
`fixture.rs`): the second of three files in one MSZIP folder comes out
byte-identical; a stored folder works; the name match ignores case; a
missing member, a file over `max_bytes`, a spanning folder and a Quantum
folder are each refused; an LZX folder with garbage data fails with an
`lzx` error, not the "unsupported compression" refusal (fails if LZX is
not routed to `lzxd`). Plus one `#[ignore]` test `real_directx_cabs` that
reads `CIMMERIA_DX9_DIR` (skip when unset), extracts both DLLs of C-8 and
asserts their sizes and SHA-256 pins; run it on the Deck with
`CIMMERIA_DX9_DIR="<game>/.cimmeria-prerequisites/DX9.0c"` (the path from
`worknotes/PP-S1.md` § Game copy) and paste the output. This is the revert
guard for the LZX path.

Lane (R1, R10): core fmt, clippy, `test --lib cab_portable`, the ignored test,
`test --lib unpack`. Docs: none (PP-13).

### PP-16: Native DirectX in the Proton prefix

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-07 and PP-15 merged.
**Branch:** `pp-16`. **Commit:** `feat(launcher): Proton Play uses the game's own d3dx9_40 and xinput1_3 (PP-16)`.

Files: `E/linux_proton/mod.rs` (add `pub mod directx;`), new
`E/linux_proton/directx.rs`, new `E/linux_proton/directx_tests.rs`,
`E/linux_proton/environment.rs` (`game()` only), `E/storage/launch/proton.rs`
(one call), `E/linux_proton/environment_tests.rs`.

1. `directx.rs` per C-8. `ensure` calls an inner
   `ensure_with(prefix, game_root, pins: &[Pinned], extract: impl Fn(&Path, &str, u64) -> Result<Vec<u8>, UnpackError>)`;
   production passes `DLLS` and `cimmeria_launcher_core::unpack::extract_cab_member`.
   For each pin: `target = prefix.pfx/drive_c/windows/syswow64/<dll>`. The
   `syswow64` folder must exist (setup made it), else `ProtonError::Setup("directx")`.
   If `target` is a regular file (`symlink_metadata`, not a symlink) whose
   length and SHA-256 match the pin, skip it. Else
   `cab = cimmeria_patchset::resolve_existing_case(game_root, Path::new(".cimmeria-prerequisites/DX9.0c").join(pin.cab))`
   (D-PP12), `bytes = extract(&cab, pin.dll, pin.bytes)`; size or hash
   mismatch → `Setup("directx")`; write `<dll>.cimmeria-tmp` in `syswow64`,
   `sync_all`, `rename` over `target`. Any I/O error → `Setup("directx")`.
   Log `proton_directx` (`installed` / `present`, the dll name; no path, R8).
2. `environment.rs` `game()`: `WINEDLLOVERRIDES` becomes
   `format!("{};{}", directx::OVERRIDES, <the C-4 value>)`. Proton appends its own.
3. `proton.rs` `prepare`: call `directx::ensure(&prefix, game_root)` right after
   `setup::ensure` succeeds and before PhysX; an error →
   `proton_setup_failed` (the same refusal path `setup` uses). It runs on
   every Play, because a Proton prefix upgrade may rewrite `syswow64`.

Tests (unit, TempDir; the fake extractor counts calls and returns fixed
bytes; test pins are the size and SHA-256 of those bytes): missing target →
written and matches; matching target → extractor not called; wrong target
→ replaced; extractor returns wrong bytes → `Setup("directx")` and the old
target is untouched; a lower-case `dx9.0c` folder on disk is still found
(fails on a literal join); no `syswow64` → error; `game()` starts with
`d3dx9_40=n;xinput1_3=n;`. Name the reverting line for each.

Lane (R1): engine fmt, clippy, `test --lib linux_proton`, `test --lib launch`.

### PP-08W: Prerequisite worker probe under Proton

**Status:** BlockedDependency (`worknotes/PP-S2-ruling.md`). The ruling writes
this packet from PP-S2 step A. Expected shape: a change in
`D/runtime-probe/src/windows.rs` (i686 Windows code: this host cannot compile
it, so `ops` runs the `launcher runtime probe` workflow on the packet branch
as its compile check), plus a closed detail code on `Failure::Probe` if the
ruling wants one. **Agent:** rust-gameserver-dev. **Host:** linux, CI for the Windows build.
</content>
</invoke>
