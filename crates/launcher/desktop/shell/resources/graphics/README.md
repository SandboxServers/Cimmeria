# Development D3D9 resource

The development Mac bundle uses unmodified D9VK from WoWSilicon v3.2.2 commit
`5276d92627f26334f6580270eabadf22361d3717`, resource
`Sources/WoWSiliconSwift/Resources/Patching/d9vk/d3d9.dll`.

- [Pinned binary](https://github.com/WoWSilicon/WoWSilicon/blob/5276d92627f26334f6580270eabadf22361d3717/Sources/WoWSiliconSwift/Resources/Patching/d9vk/d3d9.dll)
- [Pinned upstream notice](https://github.com/WoWSilicon/WoWSilicon/blob/5276d92627f26334f6580270eabadf22361d3717/Sources/WoWSiliconSwift/Resources/Patching/d9vk/LICENSE), retained as `D9VK-LICENSE.txt`.
- Size: 3,399,680 bytes.
- Git blob: `5badd1a7883a05eb498f450348ec8e16d9ec1350`.
- SHA-256: `033cef34aafc409075c764684cd97751903f5850b4f8b9d9c09eb5b6e882045e`.

Download the pinned raw binary into this directory and verify both identities
before building with `CIMMERIA_D3D9_SHA256` set to that SHA-256. The binary is
ignored by Git. Native bundle resources include this directory; Play also
rechecks the compiled digest. Missing or changed bytes leave Play unavailable.

The first development Play UAT uses stock Rosetta without the optional x87
accelerator. This resource and notice do not establish licensing clearance for
the entire assembled distribution, graphics success, signing or clean-machine
startup. See [launch](../../../docs/launch.md) and
[runtime provenance](../../../../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md).
