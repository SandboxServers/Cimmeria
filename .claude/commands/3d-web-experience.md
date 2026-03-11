# 3D Web Experience

> Guidelines for building 3D experiences with Three.js in this project. Use when working on the space viewer or any 3D visualization.

---

## Role

3D Web Experience Architect — understand when dimensional depth enhances UX versus serving as unnecessary embellishment. Balance aesthetic impact with performance and accessibility.

## Core Stack

This project uses:
- **Three.js** (`three` package) for 3D rendering
- **Vite** for bundling (tree-shakes Three.js well)
- **TypeScript** for type safety

## 3D Stack Selection

| Tool | Best For |
|------|----------|
| **Three.js vanilla** | Full control, custom rendering, this project |
| **React Three Fiber** | React-integrated 3D (consider if migrating) |
| **Spline** | Visual design-first, no-code 3D |
| **Babylon.js** | Game-oriented, physics-heavy |

## Model Pipeline

### Format Selection

| Format | Use Case |
|--------|----------|
| **GLB/GLTF** | Web standard, compressed, preferred |
| **FBX** | Legacy import from game assets |
| **OBJ** | Simple geometry |

### Optimization

- Compress with `gltf-transform`
- Use Draco compression for geometry
- Target < 2MB for web models
- LOD (Level of Detail) for complex scenes

### Loading Pattern

```typescript
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { DRACOLoader } from 'three/addons/loaders/DRACOLoader.js';

const dracoLoader = new DRACOLoader();
dracoLoader.setDecoderPath('/draco/');

const loader = new GLTFLoader();
loader.setDRACOLoader(dracoLoader);

// Always show loading progress
loader.load(
  url,
  (gltf) => { scene.add(gltf.scene); },
  (progress) => { updateLoadingBar(progress.loaded / progress.total); },
  (error) => { showFallback2D(); }
);
```

## Performance Patterns

| Technique | Purpose |
|-----------|---------|
| Frustum culling | Don't render off-screen objects |
| LOD switching | Less detail at distance |
| Instanced meshes | Duplicate geometry efficiently |
| Texture atlases | Reduce draw calls |
| Object pooling | Reuse objects instead of creating/destroying |

### Frame Budget

- Target 60fps (16.6ms per frame)
- Profile with `renderer.info` for draw calls and triangles
- Use `requestAnimationFrame` properly
- Dispose geometries and materials when removing objects

## Camera Systems

For the admin space viewer:

| Type | Use |
|------|-----|
| **Orbital** | Default for space inspection/editing |
| **First-person** | Immersive walkthrough |
| **Top-down** | Map/overview mode |

```typescript
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.dampingFactor = 0.05;
controls.maxDistance = 1000;
controls.minDistance = 10;
```

## Lighting

| Type | Use |
|------|-----|
| **Directional** | Sun, primary light |
| **Ambient** | Base illumination (prevent pure black) |
| **Point** | Local lights (torches, lamps) |
| **Hemisphere** | Sky/ground ambient |

- Bake lighting when possible for static scenes
- Use shadow maps sparingly (expensive)
- Consider shadow cascades for large world spaces

## Critical Anti-Patterns

1. **3D for 3D's sake** — 3D should serve a purpose (space visualization = good, random floating shapes = bad)
2. **Desktop-only** — test on lower-end hardware, provide 2D fallback
3. **Missing loading states** — users think it's broken without progress indicators
4. **No dispose cleanup** — memory leaks from undisposed geometries/textures/materials
5. **Blocking main thread** — use Web Workers for heavy computation

## Project Context

The **Space Viewer** in this admin panel visualizes Stargate Worlds game spaces:
- Zone geometry and terrain
- Entity positions (NPCs, players, objects)
- Navigation meshes (Recast/Detour data)
- Cell boundaries and AoI (Area of Interest) regions

Key considerations:
- Game spaces can be large (open world zones)
- Need to display real-time entity positions from the backend
- Navigation mesh overlay for debugging pathfinding
- Should work smoothly even with many entities visible
