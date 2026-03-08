# 3D Game Development Principles

> Core 3D game development concepts relevant to Stargate Worlds server emulation. Apply when working on spatial systems, navigation, rendering concepts, or game world architecture.

---

## 1. Rendering Pipeline

```
Vertex Processing → Transform geometry
Rasterization → Convert to pixels
Fragment Processing → Color pixels
Output → To screen
```

### Optimization Principles

| Technique | Purpose |
|-----------|---------|
| **Frustum culling** | Don't process off-screen objects |
| **Occlusion culling** | Don't render hidden objects |
| **LOD** | Less detail at distance |
| **Batching** | Combine draw calls |

## 2. 3D Physics & Collision

### Collision Shapes

| Shape | Use Case |
|-------|----------|
| **Box** | Buildings, crates, walls |
| **Sphere** | Quick proximity checks |
| **Capsule** | Characters (player, NPC) |
| **Mesh** | Terrain (expensive, use sparingly) |

### Principles

- Simple colliders, complex visuals
- Layer-based collision filtering
- Raycasting for line-of-sight checks
- Spatial partitioning for broad-phase

## 3. Camera Systems

| Type | Use |
|------|-----|
| **Third-person** | SGW default gameplay view |
| **Orbital** | Admin panel space viewer inspection |
| **Top-down** | Map/minimap overview |

### Camera Feel

- Smooth following via interpolation (lerp)
- Collision avoidance (wall clipping prevention)
- Look-ahead for movement direction
- FOV adjustment for speed effects

## 4. Lighting

| Type | Use |
|------|-----|
| **Directional** | Sun, moon (outdoor zones) |
| **Point** | Torches, lamps, Goa'uld devices |
| **Spot** | Flashlights, focused beams |
| **Ambient** | Base illumination |

### Performance

- Bake lighting when possible for static environments
- Shadow cascades for large world spaces
- Real-time shadows only for important dynamic objects

## 5. Level of Detail (LOD)

| Distance | Model |
|----------|-------|
| Near | Full detail mesh |
| Medium | 50% triangle reduction |
| Far | 25% triangles or billboard |

## 6. Navigation & Pathfinding

### SGW Navigation System

This project uses **Recast/Detour** for navigation meshes:

- **Recast** — generates navigation meshes from world geometry
- **Detour** — runtime pathfinding queries on navmeshes
- **NavMesh v7** format (circa 2013)

### Key Concepts

| Concept | Description |
|---------|-------------|
| **NavMesh** | Walkable surface polygons |
| **NavMesh Query** | Find path from A to B |
| **Off-mesh links** | Teleporters, ladders, jumps |
| **Crowd simulation** | Multiple agents avoiding each other |

### Server-Side Pathfinding

```
NPC AI Request → Detour Query → Path Points → Movement Updates → Client Sync
```

## 7. Area of Interest (AoI)

### SGW AoI System

The server manages what each player can "see":

| Parameter | Config Key |
|-----------|-----------|
| Vision distance | `grid_vision_distance` |
| Hysteresis | `grid_hysteresis` |
| Chunk size | `grid_chunk_size` |

### Ghost Entities

When an entity enters a player's AoI:
1. Server creates a "ghost" representation
2. Sends initial property sync to client
3. Streams position/property updates
4. Destroys ghost when entity leaves AoI

## 8. Cell-Based World Partitioning

SGW divides the world into cells:
- Each cell managed by a CellApp instance
- Entities can migrate between cells
- Cell boundaries must handle handoff seamlessly
- `cell_spaces.xml` defines the spatial configuration

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Mesh colliders everywhere | Simple shapes |
| Real-time shadows on everything | Bake where possible |
| One LOD for all distances | Distance-based LOD |
| Unoptimized pathfinding | Cache paths, use navmesh properly |
| Send all entities to all clients | AoI filtering |
| Process entire world every tick | Spatial partitioning |

## Project Context

- **NavBuilder** tool generates navmeshes from zone geometry
- **CellApp** handles spatial simulation (movement, AoI, entity proximity)
- **24 zones** total (6 partially playable)
- Entity positions synced via Mercury UDP protocol
- Position updates sent at configurable tick rate
