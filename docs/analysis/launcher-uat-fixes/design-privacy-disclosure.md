# Complete diagnostics and privacy disclosure

> Type: Explanation · Audience: launcher contributors · Date: 2026-10-23

The owner requests a complete in-launcher list of launcher/client telemetry and
important verified exclusions. Audit actual collection, local storage, transport,
consent and receiver behavior before making claims. The existing two paragraphs
omit identifying game data, hostname-derived machine identity, raw session-log
bundles and the separate manual-upload permission.

Replace the existing About diagnostics and privacy body with nested, offline
HTML disclosures covering every current collection path and data category, and
all fixed-schema payload keys. Distinguish local launcher/game/patch logs, the
Launcher diagnostics summary opt-in, Game diagnostics and detailed launcher
replay, the injected player DLL, automatic session log chunks/bundles, and the
explicit Upload debug logs action. Include identifiers, gameplay/action/world
and camera data, network/protocol data, performance, memory/stack samples,
errors/free-form messages, patch/install/adoption file metadata, destination,
redaction limits, retention and changing consent while the game runs. Arbitrary
game/log fields cannot be truthfully described as a finite allowlist of text
values; explicitly disclose that free-form game output is part of the data.

The Documentation Writer owns a source-backed reference inventory. The UI must
contain the complete disclosure itself, with a reference link as supplementary
source detail. Keep controls outside the long collapsed disclosure so the main
page remains usable. Render static source HTML; reading privacy requires no
network request, IPC call, consent change or logging. Preserve the existing
anchor used by game-diagnostics status. Never expose local tokens, paths,
settings values or machine/account identifiers merely to render the disclosure.
No collection, consent defaults or upload behavior changes in this follow-up.

Verify absence claims against the published player build and its excluded lab
bridge, and limit them to diagnostics scope. Do not promise all data anonymous,
instant removal of an already injected DLL, perfect redaction, no IP visibility
at the receiving server, or a server retention/deletion guarantee unsupported
by code/config. Tell users what normal game networking still does independently
of optional diagnostics. Do not add private keys or a static secret.

Tests compare the disclosed field lists with actual Rust wire structs so a new
summary/session/event/bundle field requires a disclosure update. Add DOM checks
for consent boundaries, identifying/free-form data, verified exclusions and the
collapsed layout. Prove an omitted collected field fails the guard. Run frontend
tests/build and a JS REPL disclosure pass on the real DOM, then visually inspect
the expanded section. Native collection is unchanged; existing native checks
remain applicable unless review requires code changes. Review the new inventory
and UI before updating PR #1999, merging through the queue and releasing.
