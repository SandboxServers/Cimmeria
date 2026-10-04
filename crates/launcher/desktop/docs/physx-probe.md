# PhysX SDK probe: ABI evidence and planned experiment

**Status: experimental implementation; native Windows validation pending.** The
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

## Null-default limitation

The [NVIDIA PhysX 2.8 manual, SDK Initialization (PDF page index 34)](https://www2.denizyuret.com/bib/nvidia/PhysX28/PhysXDocumentation.pdf#page=35)
permits null allocator and output-stream arguments. This is later-version
primary documentation, not verification of the original 2.6.3 implementation.
Using those defaults in this experiment must remain explicitly experimental;
the observed descriptor words do not establish a named public struct layout.

## Implemented SDK call and remaining supervision gates

`windows_physx.rs` opens the 52,256-byte original loader with read-only sharing,
holding the file across hash verification, loading, creation and release to deny
writes/deletion. It pins the SHA above and resolves both cdecl exports before
calling creation. `physx::exercise` supplies the observed version/descriptor and
an error sentinel of `u32::MAX`; Windows passes null allocator/output pointers
under the experimental limitation above. A non-null SDK is released exactly once
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
failure paths have been validated. Native Windows execution of this change remains pending. The earlier schema-1
module-load smoke is not SDK proof; a new helper artifact is required.
