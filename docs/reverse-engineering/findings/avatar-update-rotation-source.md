# avatarUpdate rotation source: pawn rotation, not the camera

> **Status:** recovered by static analysis (headless Ghidra, read-only) on 2026-10-18 for packet PC-06 of the player-capture-bots campaign. No live client was used.
> **Verdict:** the three angle bytes in `avatarUpdateExplicit` / `avatarUpdateImplicit` carry the **controlled pawn's `Rotation`**. The camera's view yaw and pitch never reach the wire. The server cannot see where the player is aiming the camera.

## Summary

Every client move message comes from one periodic sender, `FUN_00dd06a0`. It reads the controlled entity's UE3 actor and converts that actor's `Rotation` (pawn `+0xE8`) into three radian floats. It hands those to `ServerConnection_addMove` (`0x00dd9330`), which packs them with `FUN_00de1720`. The camera object (`ASGWCamera_Player`) keeps its own view rotator at `+0x368..+0x370`. That rotator only *reads* the pawn's yaw. Nothing copies it back into the pawn or into the move message.

For a walking or falling avatar the pitch byte is always 0, because `SGWGamePawn.FaceRotation` zeroes `Pitch` for those physics modes. Turning the camera while standing still sends nothing new: rotation does not trigger a send, and the pawn's yaw does not change.

## Answers

### Q1: which rotator reaches `addMove` and the packer?

**The controlled entity's pawn `Rotation`.** Proven.

Caller chain:

1. `FUN_00c6edf0` (connection tick, called from `FUN_00c6f470` and `FUN_00c6f690`) calls `FUN_00dd8780(conn)` and then `FUN_00dd06a0([0x01ef244c], dt)`.
2. `FUN_00dd06a0(EntityManager*, double dt)` is the periodic move sender.
3. `FUN_0084a8a0` converts the FRotator to radians.
4. `ServerConnection_addMove` (`0x00dd9330`) builds the message.
5. `FUN_00de1720` packs the three angles into bytes.

`FUN_00dd06a0`, cleaned up:

```c
Entity* ent = *(Entity**)(this + 0x10);        // controlled entity
int actor   = *(int*)(ent + 0x08);             // its UE3 actor (the pawn)
float inv   = 1.0f / DAT_018cad90;             // 1/100.0
pos = (actor+0xe0, actor+0xe4, actor+0xdc) * inv;   // BW (x,y,z) = UE (Y,Z,X)/100
vel = (actor+0xf8, actor+0xfc, actor+0xf4) * inv;
FUN_0084a8a0((int*)(actor + 0xe8), &yaw, &pitch, &roll);
ServerConnection_addMove(*(this+8), *(ent+0xc), *(ent+0x24), *(ent+0x2c),
                         pos, vel, yaw, pitch, roll,
                         *(char*)(actor + 0x58),   // Physics
                         pos, /*forced*/ 0);
```

`FUN_0084a8a0(int* rot, float* a, float* b, float* c)`, with `k = DAT_018cae9c` (2π/65536):

```c
*a = (float) rot[1] * k;    // Rotation.Yaw   (+0xEC)
*b = (float)-rot[0] * k;    // -Rotation.Pitch (+0xE8)
*c = (float)-rot[2] * k;    // -Rotation.Roll  (+0xF0)
```

`FUN_00de1720(out, p1, p2, p3)` (constants: `DAT_01851a30` = 128.0, `DAT_01847294` = 256.0, `DAT_019002fc` = 1/π, `DAT_01816a8c` = 0.5 bias):

```c
out[0] = round(p1 * 128.0 * (1/π));   // yaw:   256 steps per turn
out[1] = round(p2 * 256.0 * (1/π));   // pitch: 512 steps per turn (half-angle)
out[2] = round(p3 * 256.0 * (1/π));   // roll:  512 steps per turn (half-angle)
```

Why `EM+0x10` is the pawn:

- `FUN_00dd0c10` is EntityManager vtable slot 2 (`onEntityControl(id, bool)`). It sets `this+0x10` to the in-world `Entity*` from the map at `+0x18` (or NULL), then raises `Event_Player_EntityControl`. `EM+0x10` is the server-designated controlled entity, normally the player avatar.
- `FUN_00de8430` (GameProxyPlayer level post-load) takes the engine's PlayerController and stores it at `proxy+0xc`. It then copies the controller's Location and Rotation (`+0xdc..+0xf0`) **into** the actors at `[proxy+8]+8` and `[proxy+4]+8`. So those entity actors are separate objects from the controller: they are pawns.

`addMove` has only two callers: `FUN_00dd06a0` (from `0x00dd092a`) and `ServerConnection_forcedPosition` (`0x00dd9ee0`). The second echoes the server's own angles back with `forced=1`. No camera function calls either one.

### Q2: is the pitch byte ever non-zero for a walking avatar?

**No.** Inferred, but with strong evidence.

`SGWGamePawn.FaceRotation` (decompiled editor script, `SGWGame/Classes/SGWGamePawn.uc`) does the following:

- On a ladder (Physics 9) it takes the rotation from `OnLadder.WallDir`.
- For Walking (1) or Falling (2) it sets `NewRotation.Pitch = 0`.
- For Physics 0 it keeps the current pitch and yaw.
- It then calls `SetRotation(NewRotation)`.

`SGWPlayerController.uc` does not override any of this. So `Rotation.Pitch` is 0 whenever the pawn walks or falls, and byte1 is 0. A non-zero pitch is possible only in other physics modes, such as a ladder or Physics 0. Roll gets no such clamp in script, but no walking code path was found that writes a non-zero roll.

### Q3: does the camera yaw field (`+0x36C`) feed the sent yaw?

**No.** Proven.

An instruction scan for `0x36c]` found three camera functions that touch the field, and none of them writes the pawn:

| Function | What it does with camera `+0x36C` |
|---|---|
| `FUN_00e7e600(cam, float counts)` | The yaw turn handler. Adds mouse counts, wraps the value, and writes `+0x36C`. Writes nothing else. |
| `FUN_00e7e680` | Sets `+0x36C` from a float. |
| `FUN_00e7eae0` | The camera calculation (see below). Reads the pawn and writes only camera fields and the output view. |

The other `0x36c]` hits are byte accesses on `ServerConnection+0x36c` inside `addMove`, which is an unrelated field.

`FUN_00e7eae0` finds the pawn at `[[cam+0x1b0]+0x1b0]` (camera → controller → pawn) and reads its location (`+0xdc`) and velocity (`+0xf4`, `+0xf8`). When the horizontal speed exceeds a threshold and camera flag bit `0x2` (`+0x348`) is set, it moves camera `+0x36C` toward the **pawn's** `Rotation.Yaw` (`+0xEC`). The rate is `speed / pawn.GroundSpeed(+0x25c) * cam+0x35c`. It then returns the view rotator from `cam+0x368..0x370` and puts the camera behind the pawn along that rotation. The data flows one way, from the pawn into the camera.

### Q4: does the client send `avatarUpdateExplicit` while standing still and only turning the camera?

**No camera-triggered send.** Inferred from the gating code in `FUN_00dd06a0`.

`FUN_00dd06a0` sends a move only in these cases:

- velocity changes from zero to non-zero, or back (it caches the last velocity at `this+0x70..0x78`);
- the moving interval at `this+0x7c` has elapsed while the avatar moves;
- the idle interval at `this+0x80` has elapsed.

Rotation is not part of any condition. A camera turn changes neither the pawn's velocity nor its rotation. So nothing is sent until the idle interval fires, and that send still carries the unchanged pawn yaw. The two intervals are configured in the EntityManager constructor.

Turning the pawn itself (Q/E keys, which turn the pawn at about 110°/s per the 2026-10-12 lab notes) changes `Rotation.Yaw`. While standing still, though, that new yaw also reaches the server only on the next idle-interval send.

### Q5: does the pawn rotation at `[[[0x01ef244c]+0x0c]+0x08]+0xE8` match the sent yaw?

**Yes, in the normal case.** Proven, with one caveat.

The sender reads `[[EM+0x10]+0x08]+0xE8`. PC-15 reads `[[EM+0x0c]+0x08]+0xE8`. `EM+0x0c` is the local player entity (see [client-entity-lifecycle.md](client-entity-lifecycle.md)) and `EM+0x10` is the controlled entity. They are the same entity unless the server hands control to something else (a vehicle or a ward), and in that case the client sends the Ward variants of the message anyway.

The same FRotator therefore produces both values:

- sent byte0 = `round(Yaw_ue * 2π/65536 * 128/π)`, which is `Yaw_ue / 256` rounded;
- sent byte1 = `round(-Pitch_ue * 2π/65536 * 256/π)`, which is `-Pitch_ue / 128` rounded.

## Wire byte table

The 3 angle bytes of the client move message, in wire order (payload offset 32 of `avatarUpdateExplicit` as the base connect loop reads it):

| Byte | Client source | Meaning | Confidence |
|---|---|---|---|
| byte0 | pawn `Rotation.Yaw` (`+0xEC`) × 2π/65536, packed × 128/π | pawn yaw; 256 steps per turn (about 1.41° each); `yaw_rad = byte0 × 2π/256` | proven |
| byte1 | −pawn `Rotation.Pitch` (`+0xE8`) × 2π/65536, packed × 256/π | negated pawn pitch; 512 steps per turn (about 0.70° each); `pitch_rad = byte1 × π/256`; 0 while walking or falling | proven (0-while-walking: inferred) |
| byte2 | −pawn `Rotation.Roll` (`+0xF0`) × 2π/65536, packed × 256/π | negated pawn roll; 512 steps per turn; `roll_rad = byte2 × π/256` | proven |
| aux[0..2] | residual of each angle against its packed value (see [position-updates.md](../../protocol/position-updates.md#angle-delta-packing-auxiliary-data)) | rounding remainder; not a camera angle | as documented there |

None of the bytes depends on `ASGWCamera_Player +0x368..+0x370`.

## What `player_yaw_deg` and `wire_pitch_deg` mean on `interaction.pose`

`player_yaw_deg` is the **pawn's body facing**: the direction the character model points, as last reported by the client's periodic move. It is not the camera's look direction. A player who stands still and swings the mouse to look at something leaves it unchanged. It follows pressing Q/E, steering while moving, and the slow pull of the camera toward the walking direction. It is stale by up to one idle send interval when the player turns in place. `wire_pitch_deg` is the **pawn's pitch, not the camera pitch**. For a walking or falling avatar it is always 0, so it carries no aim information. A non-zero value means the pawn is in another physics mode (for example a ladder). Neither field can tell the server what the player is looking at.

Server-side caveat: `SpaceManager::update_entity_position` unpacks all three bytes with `unpack_angle` (2π/256 per step). That is right for yaw. For pitch and roll the client packs at π/256 per step, so the stored `direction.x` (pitch) is **twice** the true value and carries the BigWorld sign (the negated UE pitch). Because the server re-packs with the same scale when it broadcasts, the byte round-trips unchanged and other clients see the right pitch. Only a reader that converts `direction.x` to degrees, such as `wire_pitch_deg`, sees the doubling. To get the true BigWorld pitch, halve `wire_pitch_deg`. Since the value is 0 while walking, this rarely shows. This was noted, not changed (out of scope for PC-06).

## Side findings

1. **The packer scale in `position-updates.md` was wrong for pitch and roll.** That doc said all three angles use 256/(2π). The decompile shows yaw at 128/π (equal to 256/(2π)) but pitch and roll at 256/π: BigWorld half-angle packing, 512 steps per turn. The doc is corrected in the same change.
2. **The camera "offset" naming looks wrong.** `crates/lab/src/supervisor/world/memory.rs` and [cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) describe camera `+0x368` / `+0x36C` as offsets relative to the pawn's rotation. `FUN_00e7eae0` instead treats `+0x36C` as an **absolute** view yaw: it moves it toward the pawn's absolute `Rotation.Yaw` and returns it directly as the view rotator. Confidence: inferred, strong. Not fixed here (out of scope); a lab read of camera `+0x36C` next to pawn `+0xEC` while walking straight would settle it.
3. **Ghidra naming.** `FUN_00dd06a0` is a good candidate for the name `EntityManager_sendMoveUpdate`, and `FUN_0084a8a0` for `FRotator_toBWRadians`. They were not renamed (the session was read-only).

## Open questions

- The exact idle and moving send intervals (`EM+0x7c`, `EM+0x80`) are read from config in the EntityManager constructor. Their values on a stock client were not recovered here.
- The script-level pitch clamp was read from decompiled editor script, not from the cooked package's bytecode. A lab read of pawn `+0xE8` while walking would turn "inferred" into "proven".
- If the answer is ever in doubt, one lab probe settles it: stand still, turn the camera 90° with the mouse, and compare the camera `+0x36C`, pawn `+0xEC` and the next `avatarUpdateExplicit` byte0 in the packet tap.

## Evidence index

| Claim | Anchor |
|---|---|
| Periodic move sender, reads `[[EM+0x10]+8]+0xE8` | `FUN_00dd06a0`, call site `0x00dd092a` |
| Rotator to radians, yaw / −pitch / −roll | `FUN_0084a8a0`, `DAT_018cae9c` |
| Angle byte packer and scales | `FUN_00de1720`, `DAT_01851a30`, `DAT_01847294`, `DAT_019002fc` |
| Only other `addMove` caller | `ServerConnection_forcedPosition` `0x00dd9ee0` |
| Connection tick drives the sender | `FUN_00c6edf0` |
| `EM+0x10` is the controlled entity | `FUN_00dd0c10` (vtable `0x019aaec4` slot 2) |
| Entity actors are pawns, not the controller | `FUN_00de8430` |
| Camera yaw writers | `FUN_00e7e600`, `FUN_00e7e680`, `FUN_00e7eae0` |
| Pitch clamp while walking or falling | `SGWGamePawn.FaceRotation` (editor script) |
| Server unpack scale | `crates/cell-world/src/cell/space_manager/entities.rs` `update_entity_position` |
