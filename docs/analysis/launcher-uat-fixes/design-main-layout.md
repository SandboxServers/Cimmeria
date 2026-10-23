# Main-page layout and Cimmeria branding

> Design before implementation, 2026-10-23. Owner requested this follow-up after
> adopting and reopening a working client. This is a direct fix, in one PR.

The main page will contain a compact Cimmeria hero, launcher update action, Play,
three preference controls and the native Changes to your client summary. The full
change list expands in place. Settings keeps maintenance, folders, servers and
recovery. Remove the Patch Notes tab and its automatic UI fetch; preserve any
native API used by other consumers. Put the entire main page in the existing
scrollable content area so small windows can reach every control.

Owner addition before PR: the adoption dialog selects SGW.exe, rather than an
ambiguous folder. Use a native executable-file filter, validate the selected
basename and ordinary-file identity, and resolve its Working/Binaries parent
through the existing native game-root resolver before fetching the catalog.
Reject another executable, a missing file, links and unsupported flat layouts.
Cancellation starts nothing. Keep the existing path-free IPC and confirmation
review; the chosen executable is a target locator, never a relocation request.

Move the existing controls, rather than duplicating them: client patches,
launcher setup diagnostics and game diagnostics retain their native persistence,
consent defaults, busy/error feedback and adopted-install binding rules. Their
status feedback remains adjacent to each control. Detailed privacy explanations
remain available under an expandable explanation. All presses show feedback.

Game readiness comes from the existing native launch inspection: a non-null
installation_id, available launch resources, no required state reopen and no
open setup operation. A known running launch retains completed setup; an unknown
launch or reconciliation keeps inspection visible. No file-path heuristic or new Plan field is introduced. The
known launcher-update gate still blocks Play independently. Share this observed
readiness with the installer view to hide the archive and Recheck Play status
buttons after setup. Errors and recovery remain visible; transient game launches
must not resurrect installation prompts. Settings retains archive and inspection
access. Preserve the adoption button's existing ownership predicate.

The misleading compatibility message comes from install-view.ts interpreting any
successful saved operation as an unfinished install. Completed adoption is not
an incomplete compatibility operation. Suppress that false message for adoption;
when the native launch check confirms readiness, suppress idle setup text while
retaining real errors, progress and recovery instructions.

Use Cimmeria for launcher-visible branding, Cimmeria Launcher for the window and
about text, and “Your journey through the gate begins here.” Keep on-disk legacy
directory names, executable names, bundle identifiers, updater asset names and
stock archive filenames compatible. Those identities are data contracts, not
headings; renaming an adopted installation would violate the in-place rule.

Validation: component/unit tests for navigation, placement, native readiness,
successful-adoption startup, and all three saved controls across remount;
mutation checks must fail with the corresponding fix removed. Run frontend
tests/typecheck/build, native shell tests/build and required lane checks. Exercise
the mounted UI in a JS REPL with persisted native state where supported, and
inspect browser layouts at normal and small window sizes. Record fixture versus
packaged/live-client limits explicitly. Walk the desktop-launcher and tester
behavior rows in docs/agents/doc-update-map.md. Ship with ship-pr, fresh review,
CI/merge queue and same-day retirement. A further release needs owner GO.
