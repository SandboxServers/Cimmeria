# Launcher CI speed validation

> Type: Reference · Audience: Launcher maintainers · Updated: 2026-10-23
> Design: [CI speed](../design-ci-speed.md)

## Implemented and measured locally

- Hosted native Rust caching, saved only on main; superseded PR cancellation.
- Native CI dev/test debug information disabled; tests, all-target clippy,
  executable builds, frontend tests and persistence UAT remain.
- Feed and overlay tools use release-tools, with the desktop dependency graph.
  Player and helper executables retain their release optimization.
- Actual tool builds via the Windows PowerShell lane passed: feed example
  111.682s (`20261023-071821-66960`), overlay CLI 34.516s
  (`20261023-072015-402452`). These local results have different caches/hardware
  from hosted baseline timings; they do not quantify a workflow improvement.
- Actual release-tools feed CLI signed an inert root-executable zip using a
  throwaway encrypted key, then passed production signature, feed and zip
  verification including the CLI's tampered-copy rejection. Only local files
  were used; the allowlisted GitHub URL was fixture metadata, never fetched.
- Actual overlay CLI packed one Lua fixture twice. Bytes and manifest entries
  matched; zip path/content, manifest size and SHA-256 were independently
  verified. The release profile and default profile were not compared bytewise.
- The feed example's unit test passed under release-tools in 1.884s
  (`20261023-072608-146156`). Core cannot run dev-dependency tests from the
  desktop workspace because it is a dependency, not a workspace member;
  the actual core binary did build and passed the fixture checks above.
- All 17 Python release tests passed (12 existing release identity tests,
  five new workflow-policy guards). Reverting cache-write policy, PR
  cancellation, engine test command, signing environment or tool optimization
  separately made its guard fail; every mutation was restored.
- New shell-stub cases execute the actual feed-tool and overlay dispatch for
  PROFILE=release/dev and a target directory containing spaces. Linux CI runs
  these and shellcheck; they were not run through Bash on this Windows host.
- One initial local fixture used an unapproved host and was correctly refused
  by the release verifier; the corrected allowlisted fixture passed.
- Existing release-cfg unused SigningKey import warning is unchanged.

## Boundaries and outstanding evidence

No production secrets, release dispatch, publication, player executable startup,
launcher replacement, game, Wine or lab run occurred. No signing cache or
third-party action was added; the key remains scoped to the signing step in
GitHub's launcher-release environment, after the tool compilation step.

Native Windows/macOS checks, script stubs/shellcheck and package/feed dry runs
must pass on the PR. A warm-cache main/PR comparison and an owner-authorized
real release are still required to establish production time savings.

Doc-update-map rows: build profile/build system, CI checklist/gating, desktop
packaging/release and tool catalog. Updated desktop README/release contract,
build-system explanation, pre-PR reference, tools prose, design index and
committed project memory. No generated blocks or player procedure changed.
