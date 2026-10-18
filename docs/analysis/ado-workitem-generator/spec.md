# ADO work item generator: specification

Status: draft spec, 2026-10-16. No code exists. Author: architect pass.

`adogen` is a standalone tool. It creates a seeded, reproducible synthetic data set of about 10,000 work items in an Azure DevOps (ADO) project. It also writes a ground-truth manifest, so the output of the work item **migration** code can be diffed against what was really created. It works on projects built from any of the four system processes: Agile, Scrum, Basic and CMMI.

## 0. Assumptions and provenance

| # | Assumption / finding | Basis |
|---|---|---|
| A1 | **No earlier ADO migration notes exist in the repo.** I grepped `docs/`, `.claude/` and the user auto-memory for `Azure DevOps`, `ADO`, `dev.azure.com`, `work item`, `workitem` and `bypassRules`. The only hit is `docs/analysis/issue-triage-2026-09-25/findings/tooling.md`. It is about an old *Azure DevOps pipeline* idea for launcher patch distribution, since superseded by GitHub Releases, and has nothing to do with work items. The earlier conversation about the migration is lost, so this spec starts from scratch. | grep, 2026-10-16 |
| A2 | The migration under test is **ADO to ADO** (Services to Services, possibly across orgs or processes). The manifest is kept tool-neutral, so an ADO-to-other migration can still diff against it. | assumed; D-AG8 |
| A3 | The target is **Azure DevOps Services** (`dev.azure.com`). Azure DevOps Server 2022 is a stretch goal (Section 13). | assumed; D-AG3 |
| A4 | The generator owns a **sandbox org** (or sandbox projects) where it can create inherited processes, projects, repos, pipelines and test plans. Its caller holds Project Collection Administrator rights, or at least *Bypass rules on work item updates* plus process-edit rights. | assumed; D-AG4 |
| A5 | There is an **identity pool** of at least 8 real org users (Stakeholder licence is enough) for assignment and `ChangedBy` impersonation. | assumed; D-AG4 |
| A6 | Custom fields are only possible on **inherited** processes. The generator therefore always creates `<Prefix><System>` inherited from the chosen system process and never edits the system process. "Works against any of the four default processes" means a project whose process inherits from Agile, Scrum, Basic or CMMI. | K |

**Confidence tags used throughout:**

- **K**: documented ADO behaviour that I know with high confidence.
- **R**: recalled from documentation with medium confidence.
- **U**: unverified or guessed.

**No claim in this spec was re-checked against live docs or a live org in this session**, because there was no network access. Milestone M0 (Section 11) runs `adogen probe` against the sandbox and turns every R or U item into a measured value in `discovery.json`.

## 1. Goals and non-goals

| Goals | Non-goals |
|---|---|
| G1: About 10,000 items (configurable) on any of the four processes. | Performance-testing ADO itself. |
| G2: Byte-for-byte reproducible **plan** from (seed, config, discovery, generator version). | Reproducing server-assigned IDs or timestamps. Those are *observed* and recorded. |
| G3: Coverage of every WIT, every field type, every link type, attachments, comments, revisions, deletes and backdating. | Boards, queries, dashboards, wiki and test *results*/runs (stretch: Section 13). |
| G4: Resumable after a crash or kill, with no duplicate items and no double-applied revisions. | Driving the ADO web UI. |
| G5: A ground-truth manifest plus a `diff` command for migration output. | Being a migration tool. |
| G6: Refuses non-sandbox targets and never logs secrets. | |

## 2. Process coverage

### 2.1 Work item types per process

The type set comes from `GET _apis/work/processes/{id}/workitemtypes` at probe time. This table is the planning baseline.

| WIT (reference name) | Agile | Scrum | Basic | CMMI | Created via | Notes |
|---|---|---|---|---|---|---|
| Epic | Y | Y | Y | Y | WIT | |
| Feature | Y | Y | - | Y | WIT | |
| User Story | Y | - | - | - | WIT | |
| Product Backlog Item | - | Y | - | - | WIT | |
| Requirement | - | - | - | Y | WIT | |
| Issue | Y | - | Y | Y | WIT | Basic's Issue is its backlog item |
| Task | Y | Y | Y | Y | WIT | |
| Bug | Y | Y | - | Y | WIT | Basic has no Bug (K) |
| Impediment | - | Y | - | - | WIT | |
| Change Request | - | - | - | Y | WIT | |
| Review | - | - | - | Y | WIT | |
| Risk | - | - | - | Y | WIT | |
| Test Case | Y | Y | Y | Y | WIT | `Microsoft.VSTS.TCM.Steps` XML |
| Shared Steps | Y | Y | Y | Y | WIT | |
| Shared Parameter | Y | Y | Y | Y | WIT | |
| Test Plan | Y | Y | Y | Y | **Test Plan API** (R: the WIT API refuses to create it) | live-dated |
| Test Suite | Y | Y | Y | Y | **Test Plan API** | static, requirement-based and query-based |
| Code Review Request/Response, Feedback Request/Response | hidden | hidden | hidden | hidden | WIT (U) | weight 0 by default; `include_hidden_types` |

### 2.2 Default distribution, count = 10,000

| WIT | Agile | Scrum | Basic | CMMI |
|---|---|---|---|---|
| Epic | 100 | 100 | 300 | 100 |
| Feature | 400 | 400 | - | 400 |
| User Story / PBI / Requirement | 2,500 | 2,500 | - | 2,170 |
| Issue | 300 | - | 3,500 | 300 |
| Task | 4,000 | 4,000 | 5,000 | 3,800 |
| Bug | 1,870 | 1,870 | - | 1,500 |
| Impediment | - | 300 | - | - |
| Change Request | - | - | - | 400 |
| Review | - | - | - | 200 |
| Risk | - | - | - | 300 |
| Test Case | 600 | 600 | 700 | 600 |
| Shared Steps | 60 | 60 | 180 | 60 |
| Shared Parameter | 30 | 30 | 90 | 30 |
| Test Plan | 20 | 20 | 30 | 20 |
| Test Suite | 120 | 120 | 200 | 120 |
| **Total** | **10,000** | **10,000** | **10,000** | **10,000** |

The weights are stored as integers, and any `count` is apportioned by the **largest-remainder method**, with ties broken by WIT reference name. Every enabled type gets at least 1 item when `count >= number of enabled types`. The weights can be overridden per process in config.

### 2.3 Workflow baseline

States and reasons are always read at probe time from `GET _apis/work/processes/{id}/workItemTypes/{wit}/states`, and the planner walks that discovered graph. In the inherited model any state can move to any state (R). The planner still prefers the "natural" path below and adds backward moves deliberately.

| Process | WIT | States (natural order) | Backward / reopen moves generated |
|---|---|---|---|
| Agile | Epic, Feature, User Story, Bug | New, Active, Resolved, Closed (+Removed) | Resolved to Active; Closed to Active (reopen); any to Removed and back |
| Agile | Task | New, Active, Closed (+Removed) | Closed to Active |
| Agile | Issue | Active, Closed | Closed to Active |
| Scrum | Epic, Feature | New, In Progress, Done (+Removed) | Done to In Progress |
| Scrum | PBI, Bug | New, Approved, Committed, Done (+Removed) | Committed to Approved; Done to Committed |
| Scrum | Task | To Do, In Progress, Done (+Removed) | Done to To Do |
| Scrum | Impediment | Open, Closed | Closed to Open |
| Basic | Epic, Issue, Task | To Do, Doing, Done | Done to Doing; Done to To Do |
| CMMI | Epic, Feature, Requirement, Bug, Task, Change Request, Issue, Risk | Proposed, Active, Resolved, Closed | Closed to Active; Resolved to Proposed |
| CMMI | Review | Active, Closed | Closed to Active |
| all | Test Case | Design, Ready, Closed | Closed to Design |
| all | Test Plan | Active, Inactive | Inactive to Active |
| all | Test Suite | In Planning, In Progress, Completed | Completed to In Progress |

On each transition `System.Reason` is set to the discovered default reason for the target state. 15% of transitions pick a non-default allowed reason instead. With `bypassRules`, 1% use an **invalid reason or state** (edge case EC-WF-03), which tests how the migration handles data that no rule would allow.

## 3. Field coverage

### 3.1 Field types

The ADO `FieldType` enum is string, integer, dateTime, plainText, html, treePath, history, double, guid, boolean, identity, picklistString, picklistInteger and picklistDouble (K).

| Type | System fields exercised | Custom field created | Notes |
|---|---|---|---|
| string (single line, 255 max, K) | System.Title, Microsoft.VSTS.Build.FoundIn | Custom.StressString | |
| plainText (long) | Microsoft.VSTS.TCM.LocalDataSource | Custom.StressPlain | |
| html (long) | System.Description, Microsoft.VSTS.Common.AcceptanceCriteria, Microsoft.VSTS.TCM.ReproSteps, Microsoft.VSTS.TCM.SystemInfo | Custom.StressHtml | |
| html in **Markdown** format | System.Description and Custom.StressHtml switched per item | (same field) | `/multilineFieldsFormat/<field>` patch path (R; probe). One-way switch from HTML to Markdown (R). |
| integer | Microsoft.VSTS.Common.Priority (picklist int), System.Rev (read-only) | Custom.StressInt | values: int32 min/max, 0, negatives |
| double | Microsoft.VSTS.Scheduling.StoryPoints, Effort, RemainingWork, OriginalEstimate, CompletedWork, Microsoft.VSTS.Common.StackRank / BacklogPriority | Custom.StressDouble | 0.1+0.2 class values, 1e-300, 1e300, -0.0 |
| dateTime | Microsoft.VSTS.Scheduling.StartDate, TargetDate, DueDate, FinishDate; ActivatedDate, ResolvedDate, ClosedDate (rule-set) | Custom.StressDate | 1753-01-01 (SQL min, U), 9999-12-31, leap day, DST edges, ms precision |
| boolean | (few system fields) | Custom.StressBool | true, false and unset |
| identity | System.AssignedTo, Microsoft.VSTS.Common.ActivatedBy, ResolvedBy, ClosedBy, System.CreatedBy, System.ChangedBy | Custom.StressIdentity (+ `allowGroups` variant, U) | pool users, a group, a deactivated user (D-AG4) |
| picklistString | Microsoft.VSTS.Common.Severity, ValueArea, Risk, Microsoft.VSTS.CMMI.Blocked, Triage | Custom.StressPickStr (closed), Custom.StressPickStrOpen (`isSuggested` = free text allowed) | values removed from the list mid-run (EC-PL-02) |
| picklistInteger | Microsoft.VSTS.Common.Priority | Custom.StressPickInt | |
| picklistDouble | none | not created (U: probably not creatable through the process API) | probe only |
| treePath | System.AreaPath, System.IterationPath | not creatable (R) | deep, unicode and moved nodes |
| history | System.History | n/a | backdated comments (Section 6.3) |
| guid | none | not creatable (U) | probe only |
| tags | System.Tags (plainText, `;`-separated) | n/a | churn (Section 6.1) |
| bookkeeping | | **Custom.GenKey** (string), **Custom.GenRev** (integer) | used for idempotency and recovery. The migration can ignore them through the diff profile. |

Fields are created at org scope with `POST _apis/wit/fields` and added to each WIT with `POST _apis/work/processes/{pid}/workItemTypes/{wit}/fields`. They are placed on the layout with `.../layout/groups/{gid}/controls` (R). Fields that are on the WIT but **not on the layout** (EC-FL-01) are also generated, because migrations often skip them.

### 3.2 Edge values

Each edge case has an ID. The manifest tags every item with the IDs it exercises, so a migration report can show coverage per case.

| ID | Field kinds | Value | Rate |
|---|---|---|---|
| EC-ST-01 | string | empty / field removed (`op: remove`) | 5% of string writes |
| EC-ST-02 | string | exactly 255 chars, plus 255 chars where some are astral (surrogate pairs) | 0.5% |
| EC-ST-03 | string | 256 chars, expected to be **rejected** (goes to `negatives.jsonl`) | 5 items |
| EC-ST-04 | string, html | emoji, including ZWJ sequences, flags and skin tones | 3% |
| EC-ST-05 | string, html | RTL Arabic and Hebrew, mixed bidi, U+202E override | 1% |
| EC-ST-06 | string | combining marks, NFC vs NFD pairs, zero-width chars, U+FEFF | 1% |
| EC-ST-07 | string | leading and trailing whitespace, CRLF inside a single-line field (U: is it stripped?) | 0.5% |
| EC-ST-08 | string | SQL, WIQL and JSON metacharacters: `' " \ ; -- @ [ ] { }` | 1% |
| EC-TX-01 | html, plain | about 1 MB and the probed max length (U) | 10 items |
| EC-HT-01 | html | tables, nested lists, `<pre><code>` blocks | 10% of html writes |
| EC-HT-02 | html | inline image `<img src=".../_apis/wit/attachments/{guid}?fileName=x.png">` that is **not** an AttachedFile relation | 3% |
| EC-HT-03 | html | identity mention `<a href="#" data-vss-mention="version:2.0,{userId}">@Name</a>` (R markup) | 3% |
| EC-HT-04 | html | work item mention `#1234` / `<a href=".../_workitems/edit/1234" data-vss-mention="version:1.0">#1234</a>`, pointing to a sibling in the same run or a cross-project item | 3% |
| EC-HT-05 | html | `<script>`, `onerror=` and `<iframe>` (server sanitises, R; the observed result is recorded) | 20 items |
| EC-HT-06 | html | external image URL, data-URI image (U: is it accepted?) | 20 items |
| EC-MD-01 | markdown | fenced code with language tags, GFM tables, task lists, headings, nested quotes | 50% of markdown writes |
| EC-MD-02 | markdown | `@<userId>` mention and `#id` link in markdown (U syntax) | 5% |
| EC-NU-01 | int, double | min, max, 0, -1; doubles with 17 significant digits; -0.0 | 2% |
| EC-NU-02 | double | value in an integer-looking form (`3.0`) | 1% |
| EC-DT-01 | dateTime | min and max dates, Feb 29, a time on a DST transition, `Z` vs `+05:30` input offsets | 2% |
| EC-ID-01 | identity | unassign (empty), a group, a deactivated user, an identity string that does not resolve (bypassRules, U) | 2% |
| EC-PL-01 | picklist open | free-text value that is not in the list | 2% |
| EC-PL-02 | picklist closed | value later removed from the list (process churn, Section 6.6) | 20 items |
| EC-TP-01 | treePath | node at max depth 14 (R), unicode node names, names that need escaping | 2% |
| EC-TP-02 | treePath | node renamed or deleted in the live phase, with items reassigned | optional churn |
| EC-TG-01 | tags | 0, 1, 50 and 100+ tags; unicode tags; tags that differ only by case; tag of max length (U) | see Section 6.1 |
| EC-FL-01 | any custom | field set but not on the form layout | 5% |
| EC-WF-03 | state, reason | state or reason that is not in the workflow, set with bypassRules | 1% |

## 4. Relationships

| Link type (reference name) | Topology | Where it is generated | Volume (10k run) |
|---|---|---|---|
| System.LinkTypes.Hierarchy-Forward/-Reverse | tree; no cycles; one parent (K) | process-natural hierarchy, plus cross-category nesting (Task under Task, Story under Story) | ~85% of non-top items have a parent |
| System.LinkTypes.Related | network; cycles allowed (K) | random pairs, triangles and 5-cycles | ~3,000 |
| System.LinkTypes.Dependency-Forward/-Reverse | dependency; no cycles (K) | chains up to length 30, diamonds | ~1,500 |
| System.LinkTypes.Duplicate-Forward/-Reverse | tree (R) | Bug/Issue pairs; one item with 20 duplicates | ~300 |
| Microsoft.VSTS.Common.TestedBy-Forward/-Reverse | dependency (R) | backlog item to Test Case; requirement-based suites | ~800 |
| Microsoft.VSTS.Common.Affects-Forward/-Reverse | dependency (R) | **CMMI only**: Change Request affects Requirement | ~400 |
| Microsoft.VSTS.TestCase.SharedStepReferencedBy | dependency (R) | Test Case to Shared Steps (also a `<compref>` in the Steps XML) | ~300 |
| Microsoft.VSTS.TestCase.SharedParameterReferencedBy | dependency (R) | Test Case to Shared Parameter | ~150 |
| System.LinkTypes.Remote.Related / Remote.Dependency | cross-org (R) | only when `peer_org` is configured (same Entra tenant) | 0 by default |
| Hyperlink | n/a | http/https, very long (2,000+ chars), unicode IRI, `mailto:`, `file://` UNC | ~500 |
| ArtifactLink: commit | `vstfs:///Git/Commit/{projId}%2F{repoId}%2F{sha}`, name "Fixed in Commit" (R) | real commits in a provisioned repo | ~400 |
| ArtifactLink: pull request | `vstfs:///Git/PullRequestId/{projId}%2F{repoId}%2F{prId}`, name "Pull Request" (R) | real PRs | ~150 |
| ArtifactLink: branch | `vstfs:///Git/Ref/{projId}%2F{repoId}%2FGB{branch}`, name "Branch" (R) | real branches, including a unicode branch name | ~150 |
| ArtifactLink: build | `vstfs:///Build/Build/{buildId}`, names "Build", "Found in build", "Integrated in build" (R) | real pipeline runs | ~150 |
| ArtifactLink: dangling | a valid URI shape whose target was deleted or never existed (U: is it accepted?) | edge case | 20 |
| AttachedFile | n/a | Section 5 | |

Every relation also carries `attributes.comment` 30% of the time, using the edge-value text generators.

**Structural edge cases**

| ID | Case | Expected |
|---|---|---|
| EC-LK-01 | orphan items of every type: no links at all | ~8% |
| EC-LK-02 | **deep hierarchy**: a 60-level parent/child chain (Epic, Feature, then nested Stories and Tasks) | 2 chains |
| EC-LK-03 | **wide parent**: one Epic with 900 children | 1 |
| EC-LK-04 | **link hub**: one item with 995 links of mixed types, then 6 more attempted; the attempts past the limit are rejected (1,000 limit, K) | 1 |
| EC-LK-05 | hierarchy cycle attempt (A child of B, B child of A), expected to be **rejected** | 10, into `negatives.jsonl` |
| EC-LK-06 | dependency cycle attempt, rejected | 10 |
| EC-LK-07 | second parent attempt, rejected | 10 |
| EC-LK-08 | self-link of each type, rejected (R) | 5 |
| EC-LK-09 | related cycles (allowed): triangles, 5-cycles, a 50-cycle | 30 |
| EC-LK-10 | link added, removed, then re-added across revisions (same and different link types) | 3% of links |
| EC-LK-11 | reparent (parent changed 1 to 5 times) | 5% of children |
| EC-LK-12 | link to an item that is later deleted, restored or destroyed | 50 |
| EC-LK-13 | cross-project link to an item in the peer project | 200 |
| EC-LK-14 | parent and child of different types, including test types (U: are they allowed?) | 20 |

## 5. Attachments

Upload is `POST _apis/wit/attachments?fileName=&uploadType=Simple`. Bodies over the simple limit use `uploadType=Chunked`, then `PUT` with `Content-Range` (R: the simple limit is 130 MB). The attachment is then bound with a JSON Patch `add /relations/-` of `rel: AttachedFile`.

| ID | Case | Count |
|---|---|---|
| EC-AT-01 | 0-byte file (U: is it accepted?) | 10 |
| EC-AT-02 | sizes log-uniform from 1 B to 10 MB | ~1,500 |
| EC-AT-03 | near the limit: probed max minus 1 KB, and probed max plus 1 B (rejected) | 2 and 2 |
| EC-AT-04 | types: png, jpg, gif, svg, pdf, docx, zip, txt (UTF-8 and UTF-16 BOM), exe-like `.bin`, no extension | mixed |
| EC-AT-05 | duplicate names on one item, with different content and with the same content | 50 items |
| EC-AT-06 | unicode, RTL, emoji and very long (200+ char) file names, and names with `#%&+` | 100 |
| EC-AT-07 | added and later removed (`remove /relations/{idx}`); some re-added | 10% |
| EC-AT-08 | 100 attachments on one item (R limit), then a 101st (rejected) | 1 |
| EC-AT-09 | inline image in HTML that is also an AttachedFile, and one that is not | 3% |
| EC-AT-10 | the same blob (same attachment GUID) bound to two items (U: is it allowed?) | 10 |

Content is generated deterministically from (seed, att_key), so it is never stored by default. The manifest records sha256, size, name and content type. `--store-blobs` keeps copies for debugging. Total upload volume at defaults is about 2.5 GB, which a config cap limits.

## 6. History

### 6.1 Revisions per item

| Bucket | Share | Mean |
|---|---|---|
| 1 revision | 10% | 1 |
| 2-5 | 35% | 3.5 |
| 6-20 | 35% | 13 |
| 21-50 | 13% | 35 |
| 51-200 | 6% | 125 |
| 201-500 | 0.9% | 350 |
| "monster" | 2 items | 2,500 (tier `extreme`: one at the probed revision limit minus 5) |

That comes to about 215,000 planned revisions, before link side-effect revisions (Section 6.4).

Each revision applies 1 to 6 operations, drawn by weight: field edit 40, state transition 12, reassign 10, tag churn 8, History comment 10, link add/remove 8, attachment add/remove 4, area/iteration move 5, type change 0.3, markdown switch 0.2, other 2.5. Tag churn adds and removes 1 to 5 tags from a vocabulary of 3,000 that includes the EC-TG-01 tags. 5% of items reach 50+ tags.

### 6.2 Backdating and impersonation

Every phase-A write (Section 6.4) is `PATCH|POST ...?bypassRules=true&suppressNotifications=true`. Each one sets:

- `System.ChangedDate` and `System.ChangedBy`.
- On revision 1 only, `System.CreatedDate` and `System.CreatedBy`.

The rules (R; M0 confirms them):

- ChangedDate is strictly later than the previous revision's ChangedDate, and never in the future.
- CreatedDate is the same as ChangedDate on revision 1.

The planner gives each item its own timeline. Items are created uniformly across the window `[window_start, window_end]`. Revision gaps are exponential, with mean 2 days, clipped so that every revision lands before `window_end`, which is at least 1 hour before the run starts. Each `ChangedBy` is drawn from the identity pool with per-item affinity: 70% the item's "owner", 30% others.

### 6.3 Comments

| Mechanism | Dating | Used for |
|---|---|---|
| `System.History` in a bypassRules revision | backdated, impersonated | about 90% of comments. HTML, mentions, `#id` links. The newer comments API also lists these (R). |
| Comments API `POST .../workItems/{id}/comments?format=markdown|html` (7.1-preview, R) | live: now, as the caller | markdown comments |
| Comments API `PATCH .../comments/{cid}` | live | edits, 1 to 4 versions. `GET .../comments/{cid}/versions` captures the truth. |
| Comments API `DELETE .../comments/{cid}` | live | 5% of comments deleted |
| Reactions `PUT .../comments/{cid}/reactions/{type}` | live | 5% (U: does the migration care?) |

### 6.4 Phases and dating hazards

Some writes cannot be backdated: comments API calls, recycle-bin operations, test-plan API calls, and possibly the revision on the *other* end of a link. One of those can push an item's ChangedDate to "now", and after that no further backdated write to that item is possible. The run therefore has two phases:

| Phase | Contents | Clock |
|---|---|---|
| A: backdated | creates, all planned revisions, links, attachments, History comments, type changes | simulated; strictly ordered per item |
| B: live | comments API (add, edit, delete, react), recycle-bin delete, restore and destroy, test plan and suite membership, process churn | real time; after phase A for that item |

**Link side effects (U, the main dating risk).** Adding link A to B also writes a revision on B. If B's side-effect revision takes A's backdated ChangedDate, the plan stays valid as long as A's op precedes B's later ops in simulated time. The executor guarantees that by running phase A in **global simulated-time order** within a lookahead window, with per-item locks covering both ends of a link. If instead B's revision gets "now", M0 detects it and the planner switches to `link_strategy = "links_last"`: every link op moves to the end of phase A, dated in its own late slice per pair. The chosen strategy is recorded in `run.json`.

### 6.5 Deletes, restore, destroy

| Op | API | Share | Notes |
|---|---|---|---|
| Soft delete to recycle bin | `DELETE _apis/wit/workitems/{id}` | 3% | |
| Restore | `PATCH _apis/wit/recyclebin/{id}` body `{"IsDeleted": false}` (R) | half of the deleted | some get more phase B ops after restore |
| Delete, restore, delete | | 0.3% | |
| Permanent destroy from recycle bin | `DELETE _apis/wit/recyclebin/{id}` | 1% | the manifest keeps the record with `final_status: destroyed` and asserts the item is **absent** |
| Direct destroy | `DELETE _apis/wit/workitems/{id}?destroy=true` | 0.2% | |
| Test artifact delete | test artifacts skip the recycle bin and are deleted permanently (R) | 1% of Test Case | |

### 6.6 Type changes, moves, churn

- **Type change** `replace /fields/System.WorkItemType` (R: allowed in Services through PATCH): User Story and Bug both ways, Task to Bug, Issue to Task, and so on. Excluded to and from test types (R: blocked). The state is remapped to a valid state of the new type, or left invalid on 10% (bypassRules, EC-WF-03).
- **Area/iteration moves**: 5% of revisions, including moves to the deepest node and moves to nodes with unicode names.
- **Cross-project move** (EC-MV-01, optional): `System.TeamProject` plus Area plus Iteration to the peer project. 20 items. If the peer uses a different process, the type changes too.
- **Process churn** (phase B, opt-in, D-AG7): remove a picklist value already in use, rename an area node, delete an iteration node with reassignment, hide a field from the layout, add a new custom field late.

## 7. Mechanics

### 7.1 Endpoints (api-version pinned in config, default `7.1`; preview endpoints marked)

| Purpose | Endpoint |
|---|---|
| Create item | `POST {org}/{project}/_apis/wit/workitems/${type}?bypassRules=true&suppressNotifications=true&validateOnly={dry}` (JSON Patch, `application/json-patch+json`) |
| Update item | `PATCH {org}/_apis/wit/workitems/{id}?bypassRules=true&suppressNotifications=true` with a leading `{"op":"test","path":"/rev","value":N}` |
| Batch | `POST {org}/_apis/wit/$batch`: up to 200 sub-requests (R; the brief's "100" is unverified), **not transactional**, temp IDs `-1, -2...` usable in relation URLs within one batch (R) |
| Read revisions | `GET .../workitems/{id}/revisions?$expand=all&$top=200&$skip=` |
| Read many | `GET _apis/wit/workitemsbatch` (POST), at most 200 IDs (K) |
| Find by GenKey | WIQL `SELECT [System.Id] FROM WorkItems WHERE [Custom.GenKey] = 'wi-000123'`, 20,000 result cap (K) |
| Comments | `.../workItems/{id}/comments` (7.1-preview.4, R) |
| Attachments | `POST _apis/wit/attachments` (simple and chunked) |
| Recycle bin | `GET/PATCH/DELETE _apis/wit/recyclebin/{id}` |
| Process / fields / layout | `_apis/work/processes/...`, `_apis/wit/fields` |
| Classification nodes | `POST {project}/_apis/wit/classificationnodes/{Areas|Iterations}/{path}` |
| Test plans / suites | `_apis/testplan/plans`, `.../suites`, `.../suites/{id}/TestCase` |
| Git and builds for artifact links | `_apis/git/repositories/{repo}/pushes`, `/pullrequests`, `_apis/pipelines/{id}/runs` |

### 7.2 Throttling (TSTU)

ADO limits each identity to **200 TSTUs per 5-minute sliding window** (K), and delays requests before it rejects them. The response headers are `Retry-After`, `X-RateLimit-Resource`, `X-RateLimit-Delay`, `X-RateLimit-Limit`, `X-RateLimit-Remaining` and `X-RateLimit-Reset` (K).

| Signal | Reaction |
|---|---|
| `X-RateLimit-Remaining` below 20% of `Limit` | halve concurrency (AIMD; floor 1) |
| `X-RateLimit-Delay` present | sleep for that delay before the next dispatch on all workers |
| 429 or 503 with `Retry-After` | wait `Retry-After`, then retry the same op |
| 429 or 503 without it, 500/502/504, connect or read timeout | full-jitter exponential backoff: base 1 s, cap 120 s, 8 attempts, then `failed_retryable` in state |
| 400 / validation error | not retried. Recorded as a failure, or as **expected** if the op is a planned negative. |
| 409, or a failed `test /rev` | go to reconcile (Section 7.3) |
| 401 / 403 | stop the run; exit code 3 |
| No errors for 60 s | add 1 to concurrency, up to `max_concurrency` (default 4) |

**Throughput estimate (U).** About 230,000 writes at 8 to 15 writes/s comes to 4 to 8 hours on one identity. `extra_pats` (several service identities, each with bypassRules permission) spreads the calls. The impersonated `ChangedBy` does not depend on the caller.

### 7.3 Idempotency, resumability, state

- **Plan first.** `adogen plan` writes `plan.jsonl.zst`: every operation with `op_key = <gen_key>/<gen_rev>/<seq>`, its simulated time and its JSON Patch template. Server IDs appear in it only as symbolic `gen_key` references.
- **State store.** `state.sqlite` (WAL mode) holds `ops(op_key PK, status, attempts, last_error, server_rev, response_hash)`, `items(gen_key PK, id, project, last_gen_rev, phase)`, `attachments(att_key PK, url)`, `comments(comment_key PK, id, version)` and `run(meta)`. Each status change commits before the next op on that item is dispatched.
- **Create recovery.** If a create times out, WIQL-query `Custom.GenKey` before retrying. If one item is found, adopt its ID. If more than one is found, which is a bug, destroy the extras and log.
- **Update recovery.** Every update writes `Custom.GenRev = gen_rev` and starts with `test /rev`. On a test failure, read the latest revision: if `Custom.GenRev` there already equals this op's `gen_rev`, the op was applied, so mark it done. Otherwise reconcile from the revisions list, or fail the item.
- **Comments and attachments.** Comment text carries an invisible marker `<!--gk:c-4217-01-->` for HTML, or a trailing zero-width marker for markdown (U: does the server preserve it?). Attachments are keyed by att_key in state, so a re-upload is only an orphaned blob (harmless).
- `adogen run` on an existing run dir resumes. A config or plan hash mismatch aborts unless `--replan-remaining` is passed.

### 7.4 Dry run

- `--dry-run=offline` builds the plan and prints stats (counts per WIT, link type, edge case, API-call estimate and TSTU estimate) with no network.
- `--dry-run=remote` also sends phase A creates and updates for a 1% sample with `validateOnly=true`, which writes nothing (K for create/update).

### 7.5 Target safety

| Guard | Rule |
|---|---|
| Org allowlist | `safety.allowed_orgs` must contain the org. Anything else exits with code 4. |
| Project prefix | Target and peer project names must start with `safety.project_prefix` (default `adogen-`). |
| Non-empty project | Refuse if the project has work items without `Custom.GenKey`, unless `--allow-foreign-items`. |
| Override | `--i-understand-this-is-not-a-sandbox=<org>/<project>` is the only way past the first two guards. The value must match exactly. There is no interactive prompt. |
| Teardown scope | Only items whose `Custom.GenKey` belongs to this run's `run_id` (prefix `wi-<run8>-`). Project and process deletion need `--include-project --confirm=<org>/<project>`. |

### 7.6 Auth and secrets

- The PAT comes from `ADO_PAT`, or from Windows Credential Manager target `adogen:<org>`. Entra tokens come from `az account get-access-token --resource 499b84ac-1321-427f-aa17-267ca6975798` (K: the ADO resource ID).
- **Scopes:** Work Items (Read, write and manage), Project and Team (Read, write and manage) for provisioning, Code (Read and write), Build (Read and execute), Test Management (Read and write), and Process edit rights.
- **Never logged.** The `Authorization` header is redacted in traces, and URLs with a `?token=` query are scrubbed. Secrets never go into `run.json`, the plan or the manifest. A unit test greps every output artifact for the PAT value.

## 8. Ground-truth manifest

### 8.1 Layout (`<run-dir>/manifest/`)

| File | Content |
|---|---|
| `run.json` | header: schema_version, generator_version, run_id, seed, config_sha256, plan_sha256, org, projects (name, id, process name, parent process, process id), window, link_strategy, identities `{alias, uniqueName, descriptor, display}`, observed limits, counts, timings |
| `items.jsonl.zst` | one `Item` per line, sorted by gen_key |
| `negatives.jsonl` | planned-rejected ops: op_key, expected error class, observed HTTP status and message hash |
| `blobs/sha256/<2>/<64>` | long text bodies over 4 KB (content-addressed) and, with `--store-blobs`, attachment bytes |
| `manifest.schema.json` | JSON Schema 2020-12 for all of the above |
| `SHA256SUMS` | integrity |

The manifest has two layers:

- **planned** is written by `plan`.
- **observed** is written by `snapshot`, which reads back every item's revisions, relations, comments with versions, and the recycle-bin state.

`adogen verify` diffs planned against observed. Server-computed fields (System.Rev, System.Watermark, System.AuthorizedDate, System.RevisedDate, BoardColumn, CommentCount and rule-set dates) are filled from the observed layer only. **The migration diff runs against the observed layer.**

### 8.2 Item record (abridged sample)

```json
{
  "gen_key": "wi-3f9a21c0-004217",
  "id": 4217,
  "project": "adogen-agile",
  "type": "User Story",
  "type_history": [{"rev": 1, "type": "Bug"}, {"rev": 14, "type": "User Story"}],
  "final_status": "restored",
  "edge_cases": ["EC-ST-04", "EC-HT-02", "EC-LK-10", "EC-AT-07"],
  "revisions": [
    {
      "rev": 1, "gen_rev": 1, "source": "planned",
      "changed_date": "2023-02-14T09:31:07.123Z", "changed_by": "u03",
      "fields": {
        "System.Title": {"v": "Checkout fails for 🧑🏽‍🚀 users ‮ (bidi)"},
        "System.State": {"v": "New"}, "System.Reason": {"v": "New defect reported"},
        "System.AreaPath": {"v": "adogen-agile\\Ωmega\\L2"},
        "System.Description": {"blob": "sha256:9c1e…", "len": 18342, "format": "html"},
        "Custom.StressDouble": {"v": 0.30000000000000004, "repr": "0.30000000000000004"},
        "System.Tags": {"set": ["perf", "ünïcode"]}
      }
    },
    {
      "rev": 2, "gen_rev": 2, "source": "planned",
      "changed_date": "2023-02-16T17:02:44.000Z", "changed_by": "u07",
      "fields": {"System.AssignedTo": {"identity": "u07"}, "System.History": {"blob": "sha256:41ab…", "format": "html"}},
      "relations_added": [{"rel_key": "r-004217-01"}], "attachments_added": ["att-004217-01"]
    },
    {"rev": 3, "source": "link_side_effect", "from": "wi-3f9a21c0-000310", "changed_date": "2023-02-17T08:00:00.000Z"}
  ],
  "relations": [
    {"rel_key": "r-004217-01", "rel": "System.LinkTypes.Hierarchy-Reverse",
     "target": {"gen_key": "wi-3f9a21c0-000310"}, "attributes": {"comment": "مرتبط"},
     "added_rev": 2, "removed_rev": 9, "readded_rev": 12},
    {"rel_key": "r-004217-02", "rel": "ArtifactLink",
     "target": {"artifact": "commit", "repo": "adogen-repo", "sha": "a1b2c3…"},
     "attributes": {"name": "Fixed in Commit"}, "added_rev": 5}
  ],
  "attachments": [
    {"att_key": "att-004217-01", "file_name": "repro 截图.png", "size": 48213,
     "sha256": "e3b0…", "content_type": "image/png", "added_rev": 2, "removed_rev": 7,
     "inline_in": null}
  ],
  "comments": [
    {"comment_key": "c-004217-01", "id": 88123, "source": "comments_api", "format": "markdown",
     "versions": [{"v": 1, "sha256": "…", "date": "2026-10-20T11:02:00Z", "by": "runner"},
                  {"v": 2, "sha256": "…", "date": "2026-10-20T11:05:10Z", "by": "runner"}],
     "deleted": false, "mentions": ["u02", "#wi-3f9a21c0-000310"]}
  ],
  "lifecycle": [{"op": "delete", "at": "2026-10-20T12:00:00Z"}, {"op": "restore", "at": "2026-10-20T12:10:00Z"}],
  "final_fields": {"System.State": {"v": "Active"}, "System.AssignedTo": {"identity": "u07"}}
}
```

**Value encoding:**

- `v` holds a scalar.
- `blob` points to text in `blobs/`, by hash with length.
- `identity` is a pool alias; `run.json` maps it to a uniqueName.
- `set` is used for tags, compared case-insensitively (R: ADO tags are case-insensitive).
- Dates are ISO-8601 UTC with milliseconds.
- `"removed": true` marks an explicit clear.

### 8.3 Diff profile (`adogen diff --expected <manifest> --actual <org>/<project> --profile p.toml`)

| Setting | Default |
|---|---|
| `ignore_fields` | System.Id, System.Rev, System.Watermark, System.AuthorizedDate, System.RevisedDate, System.PersonId, System.BoardColumn*, Custom.GenRev |
| `id_map` | via `Custom.GenKey` if migrated, otherwise via a `ReflectedWorkItemId`-style field named in the profile |
| `html_compare` | `normalized`: parse, canonicalise attributes and whitespace, rewrite attachment URLs by sha256 |
| `date_tolerance_ms` | 0 for backdated fields |
| `revisions` | `full` (every revision, in order), `final` or `count` |
| `identity_map` | a file mapping source to target uniqueName |
| Output | `diff-report.json` plus a Markdown summary: per edge-case ID pass/fail, missing, extra and mismatched entries |

## 9. Config and CLI

### 9.1 Sample `adogen.toml`

```toml
[target]
org = "https://dev.azure.com/adogen-sandbox"
project = "adogen-agile"
peer_project = "adogen-agile-peer"   # cross-project links / moves; optional
process = "Agile"                     # Agile | Scrum | Basic | CMMI
inherited_process_name = "adogen-Agile"
api_version = "7.1"

[safety]
allowed_orgs = ["adogen-sandbox"]
project_prefix = "adogen-"

[auth]
pat_env = "ADO_PAT"
extra_pat_envs = []

[generation]
seed = 424242
count = 10000
window_start = "2019-01-01T00:00:00Z"
window_end = "2026-09-30T00:00:00Z"
tier = "standard"                     # smoke | standard | extreme
include_hidden_types = false
link_strategy = "auto"                # auto | interleaved | links_last
process_churn = false
attachment_budget_mb = 3000

[generation.weights.Agile]            # optional override of Section 2.2
"User Story" = 2500
Task = 4000

[identities]
pool = ["u01:alice@adogen.example", "u02:bob@adogen.example"]  # alias:uniqueName (sandbox users only)
deactivated = ["u08"]

[execution]
max_concurrency = 4
batch_size = 100                      # clamped to the probed $batch max
retry_max_attempts = 8
run_dir = "C:/adogen-runs/agile-424242"
```

### 9.2 CLI (`adogen <cmd>`; global flags `--config`, `--run-dir`, `--log-format json|pretty`)

| Command | Does | Writes |
|---|---|---|
| `init` | writes a sample config | adogen.toml |
| `probe` | discovers process, WITs, states, reasons, fields and identities. On a scratch item it measures limits (Section 10) and dating behaviour (Section 6.4), then destroys the item. | discovery.json |
| `provision` | creates the inherited process, custom fields, layout, project and peer project, area and iteration trees, repo, commits, branches, PRs, a pipeline and builds | provision.json |
| `plan` | builds the deterministic plan (offline) | plan.jsonl.zst, plan-stats.md |
| `run` | executes or resumes phase A then phase B; `--phase a|b`, `--dry-run=offline|remote` | state.sqlite, run log |
| `status` | progress, throttle stats, failures | stdout |
| `snapshot` | reads back everything into the observed manifest | manifest/ |
| `verify` | planned vs observed (self-check) | verify-report.json |
| `diff` | ground truth vs migration target | diff-report.json/.md |
| `teardown` | soft-delete, `--destroy`, `--include-project` | teardown log |

Exit codes: 0 ok, 1 completed with failed ops, 2 config/plan error, 3 auth, 4 safety refusal, 5 aborted by throttling budget.

## 10. ADO limits to respect

| Limit | Value | Conf. | Generator behaviour |
|---|---|---|---|
| Links per work item | 1,000 | K | hub item at 995, then planned rejects |
| Attachments per work item | 100 | R | EC-AT-08 |
| Attachment size (Services) | 60 MB default | R | probe; near-limit cases use the probed value |
| Simple vs chunked upload | 130 MB | R | chunked above |
| Revisions per work item | 10,000 | R | **The brief's "1,000?" does not match what I recall.** Probe by count only in tier `extreme`. |
| `$batch` sub-requests | 200 | R | the brief's "100" is unverified; config is clamped to the probed value |
| Work items per read batch | 200 | K | |
| WIQL results | 20,000 | K | paged queries by ID range |
| Single-line string | 255 chars | K | EC-ST-02/03 |
| Long text (html, plainText) | ~1 M chars | U | probe by binary search |
| Request body size | U | U | probe; long bodies are split across revisions |
| Tag length / tags per item | U | U | probe |
| Fields per org / per process | 1,024 | R | about 15 custom fields used |
| WITs per process | 64 | R | |
| Picklist items | 2,048 | R | |
| Classification nodes | 10,000 per project, depth 14 | R | trees about 300 nodes |
| TSTU | 200 per 5 min per identity | K | Section 7.2 |
| ChangedDate monotonic, not in the future | | R | Section 6.2 |
| Recycle bin retention | U | U | snapshot right after the run; migration should run soon after |
| Comments per item / comment length | U | U | probe |

## 11. Language, layout, milestones

### 11.1 Language: Rust (decision D-AG1, proposed)

| Criterion | Rust | C# / .NET | Python |
|---|---|---|---|
| Fits this repo's toolchain, lane, CI and agents | **yes** | no | partly |
| Deterministic portable RNG | `rand_chacha` ChaCha20, portable stream (K) | `System.Random(seed)` is stable but undocumented as such | stable `random` |
| Official ADO SDK | none (thin typed REST client, about 25 endpoints) | yes, but it lags preview APIs (comments 7.1-preview, markdown format) | the SDK is in maintenance |
| Bounded async concurrency | tokio + semaphore | good | asyncio, adequate |
| Typed manifest + schema | serde + schemars | good | pydantic |
| Single Windows exe | yes | yes (self-contained) | no |

**Rationale.** Most of the work is plain JSON REST, so an SDK adds little. The traits that matter most are portable determinism, typed manifest schemas and robust concurrency. Rust also keeps the tool on the repo's existing lane, CI and agent rules.

**Switch to C#** if the migration code under test is .NET and wants to reuse the manifest types directly.

### 11.2 Layout (standalone repo, or `tools/ado-workitem-generator/` as its own workspace outside the Cimmeria workspace; D-AG2)

```
ado-workitem-generator/
  Cargo.toml                 (workspace)
  crates/adogen-cli/         clap commands, exit codes
  crates/adogen-plan/        rng streams, apportionment, timelines, workflow walks, edge-case catalog, text/blob generators
  crates/adogen-ado/         REST client, auth, redaction, throttle/AIMD, retry, endpoints
  crates/adogen-exec/        executor, per-item locks, sqlite state, reconcile, phases
  crates/adogen-manifest/    types, schema, writer, snapshot, verify, diff, html normalizer
  schemas/manifest.schema.json
  config/adogen.sample.toml
  tests/fixtures/            recorded ADO responses (scrubbed)
```

Per-item RNG is `ChaCha20Rng::from_seed(H(seed, gen_key, domain))`, so changing one WIT's count does not reshuffle the others' content. Files keep to a 500-line soft cap and a 700-line hard cap. If the tool lives in Cimmeria, every compile goes through `pwsh tools/build-lane/lane.ps1 cargo ...`.

### 11.3 Milestones

| M | Scope | Exit criterion |
|---|---|---|
| M0 | `probe` spike: every R and U row in Sections 6.4 and 10, the markdown path, $batch temp IDs, the type-change rules | `discovery.json` from a real sandbox; this spec updated with measured values |
| M1 | plan engine (offline): apportionment, workflows, timelines, edge catalog, link graph with planned negatives | golden test: seed 42 plan hash is stable; stats match Section 2.2 |
| M2 | REST client, auth, redaction, throttle, retry, sqlite state | mock-server fault suite green |
| M3 | provision; create and field revisions with backdating | 500-item smoke on the sandbox; `verify` clean |
| M4 | links (all types, negatives), artifacts, attachments, inline images | 2,000-item run; `verify` clean |
| M5 | phase B: comments API, deletes, restore, destroy, test plans and suites, churn | |
| M6 | `snapshot`, `verify`, `diff`, html normalizer | diff of a run against itself is empty; against a mutated copy, every mutation is found |
| M7 | full 10k run per process (4 runs), teardown | four manifests archived; run time and TSTU logged |

## 12. Test strategy

| Layer | What | Proves |
|---|---|---|
| Unit | apportionment sums; workflow walk only uses discovered edges (except planned EC-WF-03); patch builder output; redaction | correctness |
| Property (proptest) | text generators always yield valid UTF-8 JSON; the per-item date sequence is strictly increasing and inside the window; the hierarchy graph is acyclic except for planned negatives | invariants |
| Golden | plan for (seed, config, fixture discovery) hashes to a committed value | reproducibility (G2) |
| Mock ADO (wiremock) | 429 with and without Retry-After, X-RateLimit-Delay, 503 storms, timeouts after the server commits, failed `test /rev`, partial `$batch` failure | throttle, idempotency, reconcile |
| Kill/resume | kill the process at random op counts (20 trials); resume; the final manifest equals that of an uninterrupted run | G4 |
| Secret scan | grep every artifact and log for the PAT value | G6 |
| Live smoke (sandbox) | tier `smoke` (200 items) per process | real API semantics |
| Self-diff | `diff` of the manifest against its own source project is empty; seeded mutations are all detected | the diff is not theatre |

## 13. Risks, open questions, decisions

| ID | Item | Type |
|---|---|---|
| D-AG1 | Language: Rust (proposed) or C# (if the migration code is .NET) | **BlockedDecision** |
| D-AG2 | Location: standalone repo (recommended; ADO tooling is off-mission for the SGW emulator) or `tools/` in Cimmeria | **BlockedDecision** |
| D-AG3 | Services only, or also Azure DevOps Server 2022 (different limits; on-prem collection URL) | **BlockedDecision** |
| D-AG4 | Sandbox org, identity pool size and licences, and whether a deactivated user is acceptable | **BlockedDecision** |
| D-AG5 | What the migration is required to preserve (the default diff profile), e.g. full revision fidelity vs final state | **BlockedDecision** |
| D-AG6 | Is tier `extreme` (revision-limit item, 1,000-link hub, attachment at the size limit) on by default? | **BlockedDecision** |
| D-AG7 | Process churn phase (picklist removal, node rename/delete) in scope? | **BlockedDecision** |
| D-AG8 | Migration target: ADO to ADO, cross-process, or ADO to another tool | **BlockedDecision** |
| R1 | Link side-effect revision dating (Section 6.4) can break backdating | risk; M0 decides `link_strategy` |
| R2 | Comments API is preview and live-dated only; backdated comments rely on System.History | risk |
| R3 | Run time of 4 to 8 h per 10k run under TSTU, so 4 processes is about 1 day | risk; mitigated by `extra_pats` |
| R4 | Markdown format API is recent and may change (R/U) | risk; feature-flagged |
| R5 | Inline images reference attachment URLs in the source org, and the migration must re-host them; the generator's normalizer must compare by sha256 | design |
| R6 | ArtifactLink to non-existent targets may be rejected or silently kept (U) | probe |
| R7 | The recycle bin may auto-purge (U), so a manifest can go stale | snapshot timestamped; the diff warns |
