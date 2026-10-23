# Install immediately after selecting a verified archive

> Type: Explanation · Audience: launcher contributors · Date: 2026-10-23
> Companions: [Archive contract](../../../crates/launcher/desktop/docs/seed-archive.md#explicit-selection-starts-eligible-installation), [player steps](../../client/launcher-guide.md#use-a-game-archive-you-already-downloaded), [ledger and acceptance](README.md#archive-install-handoff-follow-up-2026-10-23)

The owner requests that selecting an existing archive for installation starts the
installation without another Install press. The existing choose/accept workflow
already waits until native verification settles and persists a verified seed.
Reuse that result to invoke the existing install action, including its fresh
native status/revision check and no-replay behavior. Apply this to explicit
archive selection and accepting a Downloads offer from the main page or Settings.
A playable/owned installation or blocked/recovery state must not trigger a new
installation; the existing native admission remains authoritative.

A cancelled picker currently returns the old chosen archive as Inspect status.
That is indistinguishable from reselecting the same verified file, so add an
optional native picker_cancelled=true status marker on that response only.
The normal status omits it. The shared workflow preserves that response without
polling it away. A successful verified selection requires chosen, no verifying,
no last_error and no cancellation marker before invoking the install callback.
No callback runs for Inspect, restore, reopen, failed verification, cancellation,
lost replies or a disposed view. No unverified archive can start installation.

Keep callbacks at the explicit action boundary. The main page reuses its install
handler; Settings receives the same callback from composition through view.
Install still re-reads native revisions and ownership before mutation. Never
synthesize a click, infer trust from a filename, replay a mutation or add Plan
fields. Show busy on the initial archive press, verification progress, then the
existing install status/progress. The launcher never changes the source RAR.

Tests cover verification-in-progress followed by success before one Install
request, a rejected archive, both cancellation with a previous selection and
empty cancellation, duplicate presses, a lost reply, reopen with a remembered
archive, accepting an offer and Settings wiring. Native tests pin the optional
cancel marker without persisting it. Extend the archive JS logic UAT to exercise
the real install/selection views and state transitions. Prove removal of the
handoff fails a regression. Run frontend tests/build and affected native checks,
perform fixture visual verification, and include this in the same PR as PhysX
and development-key diagnosis. No actual game/install or lab is required.
