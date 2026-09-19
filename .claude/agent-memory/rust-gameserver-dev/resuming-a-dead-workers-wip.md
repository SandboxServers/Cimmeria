# Resuming a dead worker's WIP commit

A `wip(...): session-N worker output, unbuilt, unverified` commit is a
*draft*, not a starting point you can trust. Every failure below came from
one real handover (Harset H06), and all of them were invisible until the
first build and the first live-DB run.

**It may not compile, and the gap can be a missing function.** H06's WIP
called `persist_arrival::addresses_learned_by_travelling(...)`, which was
never written — a leftover from an earlier two-helper design the final SQL
had made unnecessary. `cargo check` found it in one pass. Build before
reading the diff closely; the compiler tells you where the draft stopped.

**Its tests encode the design it started from, not the one it shipped.** Two
H06 live-DB fixtures asserted exact `known_stargates` contents after an
arrival, but the merged single-statement UPDATE also learns the gates of the
world named by the row's *pre-update* `world_location`. The origin world's
own gate turned up in both assertions. That was correct behaviour and a
useless test; the fix was to depart a gateless world, not to change the SQL.

**Its worknote's forward-looking sections are stale.** H06's integration
request told Castle CA10 which three lines to carry across a split that had
not happened yet. CA10 landed first, so the request had to be inverted into
a record of what the port actually did. Read the worknote for *decisions*
(check placement, refusal shape, ordering constraints, rejected
alternatives) and re-derive everything it says about other branches.

**Its "not run" list is the work.** A `Validation` table of `TBD` rows and a
`Regression proof per test: TBD` mean nothing has been proven. Budget for
the full proof, and expect it to find things: H06's proof exposed a vacuous
cancel guard that passed with the feature deleted, because the test dialled
a same-world gate whose own reject branch did the cancelling.

**Port hunks by hand; never resolve a modify/delete conflict by taking a
side.** When upstream turned `foo.rs` into `foo/`, `git merge` leaves the old
file in the tree as a modify/delete conflict. Taking "ours" resurrects a
dead file; taking "theirs" drops the packet. Re-read the new structure, place
each hunk where it now belongs, `git rm` the old path.

Related: [[revert-verification-loses-uncommitted-fmt]],
[[vacuous-guard-and-sentinel-collision-review]],
[[stacked-branch-rebase-traps]].
