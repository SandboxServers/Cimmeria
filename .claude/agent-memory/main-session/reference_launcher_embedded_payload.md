# Embedded Windows launcher payload

> **Type:** Reference · **Audience:** Resuming launcher contributors · **Updated:** 2026-10-23
> **Companions:** [Design](../../../docs/analysis/launcher-uat-fixes/design-embedded-payload.md), [release contract](../../../crates/launcher/desktop/docs/release.md#what-the-windows-job-builds)

Owner authorizes one Windows launcher EXE embedding the independently pinned
launch worker, client-patches DLL and player telemetry DLL. Keep the existing
public ZIP asset name; do not switch browser downloads to a bare EXE. Startup
verifies/repairs the app-local payloads/content-digest/windows cache. Shared
leases protect active versions; cleanup preserves unknown entries and old loose
sidecars. Build-stage pins and headless --verify-embedded-payload prove packaging.

macOS already distributes a single Cimmeria Launcher.app bundle containing its
helpers/DLLs/graphics inside a ZIP; do not change that resource layout. WebView2,
game content and the downloaded Mac Wine runtime remain separate. The owner gave
GO to merge the embedded-payload PR and release the launcher from main after
merge, retaining the Windows ZIP. Packaged/game acceptance remains pending;
this GO does not authorize a live game or lab session.
