# T-00 signed evidence manifest

> **Title**: T-00 signed evidence manifest
> **Type**: Reference
> **Audience**: Evidence reviewers and campaign coordinators
> **Last updated**: 2026-10-14
> **Companions**: [Ledger](README.md), [T-00 worknote](worknotes/T-00.md), [wave resume](RESUME.md)

Signed by: `game-archaeology-specialist` (evidence worker), 2026-10-14.
This is an authored attestation, not a cryptographic identity signature or a
mission/lore advisor approval. The documentation review and evidence integration pass;
mission advisor approval remains pending.

| Artifact | Identity |
|---|---|
| Seed snapshot | Git `d3efb7d41aad0b6008397f112d224ce7430d7acf` |
| Source ZIP | `SGW_FINAL_DEVELOPER_HANDOFF.zip` |
| ZIP SHA-256 | `0317e0d4445f1c851bb60ee910fc8ea77b5661bfd589d315015dfcadc1cfb28a` |
| Mission member | `WORLDS/RECOVERED/Tollana/02_MISSIONS.csv` |
| Matrix | [tollana-source-matrix.csv](tollana-source-matrix.csv) |
| Matrix SHA-256 (canonical Git blob, UTF-8 LF) | `7ae4bf5cd425fac359fd238d6744831fdc91b5842112dee4bd419240efa15ed6` |
| Scope | 51 unique census IDs; conditional 1364/1366 documented outside the matrix |
| Dispositions | 51 verified hold; separate owner intent: 23 restore, seven cross-zone, 21 conditional/lineage-held |
| Dispatch authorization | None; every row BlockedEvidence |

I attest that the matrix was compared to the named ZIP and seed snapshot,
that missing per-mission build lineage was not inferred from map builds, and
that no recipient was invented for blank objectives 2438 or 4734. I attest
only to the documented evidence and bounded checks, not to playable content,
client compatibility or missing SQLite joins. The matrix's provenance column
identifies candidate recovered text; no reconstructed encounter is approved.

Mission advisor approval: **pending**. Documentation review: **passed, two rounds**.
Coordinator integration: **Integrated, evidence packet only**. Client UAT: **pending, not run**.
All 51 chronology dispositions remain hold/BlockedEvidence; review grants no runtime dispatch.

## Canonical matrix verification

Hash the committed matrix blob's raw bytes, using LF line endings, without a
PowerShell text pipeline or checkout-byte hash. In PowerShell 7, pass the reviewed
commit as `<reviewed-commit>` and run this from the repository worktree:

```powershell
$blob = 'docs/analysis/tollana-continuation/tollana-source-matrix.csv'
$start = [Diagnostics.ProcessStartInfo]::new('git')
$start.ArgumentList.Add('-C')
$start.ArgumentList.Add((Get-Location).Path)
$start.ArgumentList.Add('cat-file')
$start.ArgumentList.Add('blob')
$start.ArgumentList.Add("<reviewed-commit>:$blob")
$start.RedirectStandardOutput = $true
$start.UseShellExecute = $false
$process = [Diagnostics.Process]::Start($start)
$bytes = [IO.MemoryStream]::new()
$process.StandardOutput.BaseStream.CopyTo($bytes)
$process.WaitForExit()
if ($process.ExitCode -ne 0) { throw 'git cat-file failed' }
[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes.ToArray())).ToLowerInvariant()
$process.Dispose()
$bytes.Dispose()
```

Git normalizes this text file to LF; a Windows CRLF checkout has different bytes.
The listed SHA-256 identifies the canonical blob, independent of checkout EOL.
Review fix attestation, 2026-10-14: chronology holds, separate restoration intent,
and per-ID district/dependency evidence were checked against T-00 and journal
references. This supplements the evidence worker's original attestation;
mission advisor approval and client UAT remain pending. The wave-close review
record is in the companion T-00 worknote.
