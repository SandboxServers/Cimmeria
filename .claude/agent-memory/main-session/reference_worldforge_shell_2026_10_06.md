# Worldforge shell probe, 2026-10-06

- The `CimmeriaLab` direct package route can clone Agnos small hallway elbow,
  three-way and four-way actors from `Agnos-0002fff9.umap` exports 290, 375
  and 265. Their component tails are the supported unlit form. The same
  elbow in `Agnos-00000005.umap` export 323 has an unsupported 25,868-byte
  native tail and fails closed.
- `upk_patch clone-objects --first-at ... --yaw-degrees 0|90|180|270`
  transforms a donor actor to an absolute cardinal yaw. A regression test
  checks the serialized position and rotation. Build natively on Windows
  through Git Bash and `tools/build-lane/lane.sh`; verify which shell an
  unqualified `bash` resolves to before invoking it from PowerShell.
- A 3 × 3 shell from those modules is packaged as the v6 local client probe.
  Cardinally oriented elbows close corners, three-ways line the edges and
  four-ways form the two central nodes. The south exit connects to the gate
  apron. Structural and property-name audits pass. Client collision and
  visual seams remain unverified. The exterior garden slab is not bounded;
  do not call the whole world sealed.
