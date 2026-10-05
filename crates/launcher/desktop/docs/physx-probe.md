# PhysX SDK probe: ABI evidence and supervised experiment

**Status: experimental implementation; native Windows CI and private Wine SDK lifecycle passed.** The
[probe](prerequisites.md) now separates module loading from a PhysX SDK
creation/release attempt. Production supervision/integration remains unfinished;
this does not start SGW, create graphics devices or claim readiness.

## Original-binary evidence

The ABI advisory inspected the original signed-release binaries identified in
[prerequisite evidence](prerequisites.md#original-client-evidence). Pin the loader
before use to SHA-256:
`863e3ec87198bf1a5d5638a20695529dacc9460b0939f2579fe7a7faad2af924`.
Addresses below are observed virtual addresses unless marked RVA; they are
version-specific evidence, not portable entry points.

| Observation | Evidence |
| --- | --- |
| `NxCreatePhysicsSDK` | Loader RVA `0x1280`; x86 cdecl, five arguments: u32 version, allocator pointer, output pointer, descriptor pointer, error pointer. |
| SGW creation call | `0x005592a8`; caller stack cleanup at `0x005592b9`. |
| SDK version | `0x02060300`, read from SGW global `0x01db4bfc`. |
| Descriptor bytes | Four u32 values `[65536, 256, 2048, 0]` (16 bytes), written at `0x00559288`–`0x005592a0`; field names remain unknown. |
| Optional error pointer | Loader `0x1000129b` initializes a supplied error location to `1`; preserve numeric results without inventing enum meanings. |
| Core lookup | At `0x10001070`, reads `HKLM\Software\Ageia Technologies`, value `PhysXCore Path` of type `REG_SZ`; appends `v2.6.3\PhysXCore.dll`. |
| Core loading | Calls `LoadLibraryA` at `0x10001194`. Registry lookup/load success was not established by static inspection. |
| Release export | RVA `0x1330`, x86 cdecl with one SDK pointer; plain `RET` at RVA `0x1395`. |

`enableLocalPhysXCore` is an adapter involving a six-byte gate at
`0x10001530`/`0x100015e0`, not an established boolean switch. Do not call it using
an inferred boolean signature or treat it as a supported local-core override.

## Exact 2.6.3 core evidence

Read-only research carved the MSI beginning at byte 35,463 of the original
retained PhysX 7.11.13 installer (EXE SHA-256
`920d5e09e6ba0a92342271c18c67472461813424d70b5c0b981b6f13b129fbf6`);
this is not a production extraction/install path.
Its embedded `Cabs.m26` contains
`PhysXCore.dll.FA211449_AC3F_4A7D_A467_6CC0BA89C1A4`, version 2.6.3.5, SHA-256:
`e54919c223e768e0fd12736119102069f7d3bdf1989f09f223119fd9ef0fe31e`.
The ABI advisory found these exact-core behaviors:

| Observation | Evidence |
| --- | --- |
| Version gate | `NpCreatePhysicsSDK`, RVA `0x11f1e0`, compares `0x02060300`; mismatch reports error `2`. |
| Descriptor validation | `0x1011f204`–`0x1011f22a` checks first word `65536` and power-of-two words at offsets 4 and 8; rejection reports error `3`. Field names remain unknown. |
| Null allocator | Foundation code `0x1013d546`–`0x1013d54e` explicitly selects built-in allocator `0x1024a2c0`. |
| Null output | Stored through vtable slot `+4` to `0x10085330`; reporter `0x1013d2cd`–`0x1013d2d2` skips the callback when null. |
| Error pointer | Required by core code at `0x1011f1ed`; the helper supplies a non-null writable location. |

This closes the earlier inference about null allocator/output defaults for this
exact core. The [NVIDIA PhysX 2.8 manual, SDK Initialization (PDF page index 34)](https://www2.denizyuret.com/bib/nvidia/PhysX28/PhysXDocumentation.pdf#page=35)
also permits those defaults, but is supplementary later-version documentation.
Static imports include KERNEL32, WS2_32, SETUPAPI and ADVAPI32; dynamic `user32`
strings mean this inventory does not prove the core cannot display UI. Static
ABI evidence still requires bounded execution in the actual Wine environment.

## Implemented SDK call and remaining supervision gates

`windows_physx.rs` opens the 52,256-byte original loader with read-only sharing,
holding the file across hash verification, loading, creation and release to deny
writes/deletion. It pins the SHA above and resolves both cdecl exports before
calling creation. `physx::exercise` supplies the observed version/descriptor and
an error sentinel of `u32::MAX`; Windows passes null allocator/output pointers
using the exact-core defaults verified above. A non-null SDK is released exactly once
while the module remains loaded. No assumed C++ vtable is invoked.

Report schema 2 carries a tagged `physx_sdk` result: `not_checked`,
`unverified_loader`, `load_failed` with numeric `win32_error`, `missing_export`,
`create_failed` with optional numeric `sdk_error`, or `initialized_and_released`.
A null result never calls release; an unchanged error sentinel becomes null error
information, not an invented code. Success is emitted only after release returns.
Request schema remains 1 and `game_started` remains false. Parent-enforced
deadlines, bounded output, exit/crash observation and owned-process cleanup are
required; missing output, crash or timeout cannot become initialization success.
Native Windows and the pinned private Wine environment need separate evidence.

This tests at most one SDK lifecycle. It would not establish scene simulation,
PhysX cooking, graphics, game startup, login or gameplay. Keep it separate from
production capability/readiness flags until its ABI assumptions and supervised
failure paths have been validated. Native Windows SDK CI `37201203156` passed at
`ebeaaaa47`; artifact `11302724959` contains executable SHA-256
`3ed60ee8fba3a6b02bf860b559e5ca55f4c5b99836d88126b3f86865ebac3ebc`.
The schema-2 before/after Wine result is recorded below. The earlier schema-1
module-load smoke is not SDK proof, and CI success alone does not establish SDK initialization.

## Observed SDK lifecycle under Wine

The original core now has dynamic evidence as well: the private-prefix before/
after test passed (lane `20261004-071507-6004`, 23.074 seconds). With no core
registered, the helper returned `create_failed` with error `1`; after diagnostic
registration of the hash-verified 2.6.3.5 core, it returned
`initialized_and_released`. The observed descriptor and null defaults therefore
worked for one lifecycle in pinned WoWSilicon Wine r17. This does not validate
vendor installation, cooking, scenes, graphics or game launch. Reproduction and
artifact identity are in [prerequisite validation](prerequisites.md#sdk-beforeafter-check).
