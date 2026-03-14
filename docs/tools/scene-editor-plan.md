# SGW 3D Scene Editor — Detailed Implementation Plan

> **Project**: Full 3D scene editor for Stargate Worlds content
> **Foundation**: `cimmeria-upk` crate (100% SGW package parsing, 812K actors, 487K Kismet nodes)
> **UE3 Reference Source**: `github.com/CodeRedModding/UnrealEngine3` (Build 10897, 2013 — has FUntypedBulkData, modern StaticMesh, version 491-867 range brackets SGW's 486)
> **Secondary Reference**: `github.com/gameboys84/unrealengine3` (early 2004 build — too old, uses TLazyArray, not FUntypedBulkData)
> **Created**: 2026-03-12

---

## 1. Architecture

Tauri 2 desktop app with Rust backend (parsing, rendering, DB) and React+TypeScript frontend (UI panels).

```
┌─────────────────────────────────────────────────────────────┐
│                     Tauri 2 Application                      │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │              Rust Backend (tokio runtime)              │  │
│  │                                                        │  │
│  │  ┌─────────────┐ ┌──────────────┐ ┌──────────────┐   │  │
│  │  │ cimmeria-upk│ │ sgw-renderer │ │ sgw-editor   │   │  │
│  │  │ (parsing)   │ │ (wgpu scene) │ │ (app logic)  │   │  │
│  │  └──────┬──────┘ └──────┬───────┘ └──────┬───────┘   │  │
│  │         │               │                │            │  │
│  │  ┌──────┴───────────────┴────────────────┴─────────┐  │  │
│  │  │    Asset Pipeline (decode → GPU upload)          │  │  │
│  │  │    Scene Graph (transforms, culling, picking)    │  │  │
│  │  │    Database Layer (sqlx → PostgreSQL)            │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────┘  │
│                        │ Tauri IPC                            │
│  ┌────────────────────────────────────────────────────────┐  │
│  │            Webview Frontend (React + TS)               │  │
│  │                                                        │  │
│  │  ┌──────────┐ ┌────────────┐ ┌──────────────────┐    │  │
│  │  │ Outliner │ │ Properties │ │ Asset Browser    │    │  │
│  │  │ (tree)   │ │ (inspector)│ │ (pkg contents)   │    │  │
│  │  ├──────────┤ ├────────────┤ ├──────────────────┤    │  │
│  │  │ Toolbar  │ │ Kismet     │ │ DB Entity Palette│    │  │
│  │  │ & Status │ │ (ReactFlow)│ │ (spawn/region)   │    │  │
│  │  └──────────┘ └────────────┘ └──────────────────┘    │  │
│  └────────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌────────────────────────────────────────────────────────┐  │
│  │         3D Viewport (wgpu + winit window)              │  │
│  │   Camera, picking, gizmos, scene rendering             │  │
│  └────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

The 3D viewport runs as a wgpu surface in a native winit window managed by the Tauri backend. The webview handles all 2D UI panels. IPC bridges selection, camera commands, and transform updates between the two.

**Phase 1 simplification**: Use a 2D HTML Canvas in the webview for the top-down view. Upgrade to wgpu in Phase 5.

---

## 2. Crate Structure

```
crates/
  upk/                          # EXISTS — Package parsing
  upk-objects/                  # NEW — UE3 object deserializers
  sgw-renderer/                 # NEW — wgpu scene graph, camera, picking, gizmos
  sgw-editor-core/              # NEW — Editor state, undo/redo, selection, transforms

tools/
  SceneEditor/                  # NEW — Tauri 2 app shell
    Cargo.toml
    build.rs
    tauri.conf.json
    capabilities/
    src/
      main.rs                   # Tauri builder, wgpu window lifecycle
      state.rs                  # AppState: DB pool, asset cache, scene, editor state
      commands/
        mod.rs
        scene.rs                # load_zone, list_actors, select_actor, move_actor
        assets.rs               # browse_packages, search_assets, get_texture_preview
        database.rs             # connect_db, query_spawns, save_spawn
        kismet.rs               # load_kismet_graph, get_node_details
        viewport.rs             # set_camera, pick_object, toggle_grid
    ui/                         # React + Vite frontend
      package.json
      src/
        App.tsx
        layout/
          EditorFrame.tsx       # Main window: menu bar + toolbar + viewports + status bar
          DockManager.tsx       # Dockable/floating panel management (allotment)
        menu/
          MenuBar.tsx           # File|Edit|View|Brush|Build|Play|Tools|Preferences|Help
          FileMenu.tsx          # New/Open/Save/SaveAs/SaveAll/Import/Export/RecentFiles
          EditMenu.tsx          # Undo/Redo/Cut/Copy/Paste/Duplicate/Delete/SelectAll/SelectNone
          ViewMenu.tsx          # Outliner/Properties/ContentBrowser/Kismet/EntityPalette toggle
          BrushMenu.tsx         # Add/Subtract/Intersect/Deintersect/CSG operations
          BuildMenu.tsx         # Build Geometry/Lighting/Paths/All
          ToolsMenu.tsx         # MapCheck/ActorErrors/Search/Replace
        toolbar/
          MainToolbar.tsx       # UnrealEd main toolbar replica
          ToolbarGroup.tsx      # Separator-grouped button cluster
          # Groups: FileOps | EditOps | Transform(Translate/Rotate/Scale/NonUniform) |
          #         CoordSystem(World/Local) | Search/Content/Kismet |
          #         ViewportConfig | BuildOps | PlayInEditor
        viewport/
          ViewportContainer.tsx # 2x2 splitter with 4 viewports (default: Front/Side/Top/Perspective)
          ViewportPanel.tsx     # Single viewport + per-viewport toolbar
          ViewportToolbar.tsx   # Options|TypeSelector|Realtime|RenderMode|GameView|CameraSpeed
          ViewportCanvas.tsx    # HTML Canvas (Phase 1) or wgpu surface (Phase 5+)
          ViewportConfig.tsx    # Layout switcher: 2x2 / 1x3 / 1+2 / 1x1H / 1x1V
        panels/
          Outliner.tsx          # Actor tree (hierarchical by tile/class)
          PropertyWindow.tsx    # Category tree → name|value splitter, favorites, search filter
          PropertyCategory.tsx  # FCategoryPropertyNode: collapsible group
          PropertyItem.tsx      # FItemPropertyNode: name | value editor
          ContentBrowser.tsx    # Package tree + class filter + search + thumbnail grid
          EntityPalette.tsx     # DB entity templates for drag-to-place
          ZoneSelector.tsx      # Zone/world picker
        editors/
          KismetEditor.tsx      # React Flow graph for Kismet sequences
          KismetNode.tsx        # Custom node: diamond(event)/rect(action)/circle(variable)
          KismetConnection.tsx  # Custom edge: black(logic)/red(event)/yellow(hover)
        statusbar/
          StatusBar.tsx         # Full UnrealEd status bar replica
          ExecCombo.tsx         # Command input field (exec console)
          SnapControls.tsx      # Grid/Rotation/Scale snap value displays
          PositionDisplay.tsx   # Mouse worldspace position (X, Y, Z)
          ActorInfo.tsx         # Selected actor name + DrawScale display
```

### Dependency graph:

```
tools/SceneEditor (Tauri app)
  ├── cimmeria-upk              (package parsing)
  ├── upk-objects               (mesh/texture/terrain deserialization)
  │     └── cimmeria-upk
  ├── sgw-renderer              (wgpu rendering)
  │     └── upk-objects
  ├── sgw-editor-core           (editor logic, undo/redo)
  │     ├── upk-objects
  │     └── sgw-renderer
  ├── sqlx                      (database)
  ├── tauri                     (app shell)
  └── serde/serde_json          (IPC serialization)
```

---

## 3. Data Pipeline

### Asset flow from .upk to GPU:

```
.upk/.umap file on disk
    │
    ▼
cimmeria-upk: Package::open()
    ├── Header, name/import/export tables
    ├── LZO decompression if compressed
    └── pkg.read_export_data(export) → raw bytes
           │
           ▼
upk-objects: per-class deserializer
    ├── StaticMesh::deserialize(bytes, pkg)
    │     → vertices, normals, uvs, indices, LODs, bounds
    ├── Texture2D::deserialize(bytes, pkg)
    │     → DXT compressed mip data, dimensions, format
    ├── Terrain::deserialize(bytes, pkg)
    │     → heightmap, layers, alpha maps, bounds
    ├── Brush::deserialize(bytes, pkg)
    │     → BSP polys: vertices, planes, surface flags
    └── MaterialInstanceConstant::deserialize(bytes, pkg)
          → parent ref, texture parameter overrides, scalar params
           │
           ▼
sgw-renderer: GPU upload
    ├── MeshGpuData { vertex_buffer, index_buffer, bounds }
    ├── TextureGpuData { wgpu::Texture, wgpu::TextureView, sampler }
    ├── TerrainChunk { height_texture, splat_maps, vertex_buffer }
    └── MaterialInstance { shader_pipeline, texture_bindings }
           │
           ▼
Scene graph node
    ├── transform: Mat4 (from actor Location/Rotation/DrawScale)
    ├── mesh: Arc<MeshGpuData>
    ├── materials: Vec<Arc<MaterialInstance>>
    └── metadata: { class_name, object_name, export_index, source_tile }
```

### Cross-package reference resolution:

StaticMeshActors in .umap files reference meshes via import entries pointing to .upk packages. The editor needs a **package index** built at startup:

```rust
struct PackageIndex {
    // (package_name, object_name) → (file_path, export_index)
    exports: HashMap<(String, String), (PathBuf, usize)>,
}
```

Building this requires scanning all 5,021 packages' export tables (~50 seconds). Cache to disk via bincode for instant subsequent launches.

---

## 4. UE3 Format Work Needed

### 4a. StaticMesh — HIGH priority, HARD

**Reference**: `Engine/Src/UnStaticMesh.cpp`, `Engine/Inc/UnStaticMesh.h`

```
UStaticMesh::Serialize(FArchive& Ar):
  1. UObject base (tagged properties)
  2. InternalVersion: i32
  3. LODModels: TArray<FStaticMeshRenderData>
     Per LOD:
       - Elements: TArray<FStaticMeshElement> (material sections)
       - PositionVertexBuffer: FUntypedBulkData (Vec3 array)
       - VertexBuffer: FUntypedBulkData (normals, tangent, UV channels)
       - IndexBuffer: FUntypedBulkData (u16 or u32 indices)
       - NumVertices, NumTriangles
  4. kDOPTree: collision tree
  5. Bounds: FBoxSphereBounds
```

FUntypedBulkData format: flags + element_count + element_size + inline/offset data.

**Approach**: Start with LOD 0 only. Validate against umodel-extracted meshes. If SGW modified the format, disassemble `UStaticMesh::Serialize` in Ghidra.

### 4b. Texture2D — HIGH priority, MEDIUM difficulty

**Reference**: `Engine/Src/UnTexture.cpp`

```
UTexture2D::Serialize:
  1. Tagged properties (SizeX, SizeY, Format as enum)
  2. Mips: TArray<FTexture2DMipMap>
     Per mip: FUntypedBulkData + SizeX + SizeY
```

Pixel formats: PF_DXT1, PF_DXT3, PF_DXT5, PF_A8R8G8B8. wgpu supports BC1/BC2/BC3 natively — upload DXT data directly to GPU without CPU decompression. Fall back to `texture2ddecoder` crate for thumbnail generation.

**Complication**: Some textures store mip data in separate .tfc (texture file cache) files.

### 4c. Terrain — MEDIUM priority, HARD

Heightmap + multi-layer material blending. For initial implementation: render as a simple heightfield mesh (single color). Full material layers later.

**Risk**: Heavily version-dependent. Attempt after StaticMesh works.

### 4d. Brush/BSP — LOW priority, MEDIUM

BSP geometry as UModel objects. Most SGW environments use StaticMesh. BSP mainly used for invisible blocking volumes — render as wireframe outlines.

### 4e. MaterialInstanceConstant — MEDIUM priority, LOW for basic

Extract parent material ref + diffuse texture reference. Apply as simple textured material. Full material graph reconstruction is a stretch goal.

---

## 5. Rendering Approach

### sgw-renderer core:

```rust
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub camera: Camera,
    pub grid: GridSettings,
    pub gizmo: Option<GizmoState>,
    pub selection: HashSet<NodeId>,
}

pub struct SceneNode {
    pub id: NodeId,
    pub transform: Transform,        // position, rotation, scale
    pub mesh: Option<Arc<GpuMesh>>,
    pub material: Option<Arc<GpuMaterial>>,
    pub bounds: BoundingBox,
    pub visible: bool,
    pub metadata: ActorMetadata,
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    depth_pass: DepthPass,
    color_pass: ColorPass,
    outline_pass: OutlinePass,       // Selection highlight
    gizmo_pass: GizmoPass,          // Translate/rotate/scale gizmos
    grid_pass: GridPass,             // Infinite ground grid
    debug_pass: DebugPass,          // Wireframes, bounds, nav mesh
}
```

### Camera modes:
- **Orbit** (default): Click+drag orbit, scroll zoom, middle-drag pan
- **Fly**: WASD + mouse look
- **Top-down**: Orthographic, scroll zoom, drag pan
- **Focus**: Press F to frame selection

### Object picking:
GPU color-ID picking: render each node with a unique color to an offscreen framebuffer. Read pixel at cursor → map to NodeId. Faster than CPU raycasting for complex meshes.

### Selection and gizmos:
- Click select, Ctrl+click multi-select, box select
- Outline shader for selection highlight
- Translation gizmo (W), Rotation gizmo (E), Scale gizmo (R)
- Grid snapping (configurable distance)

### Performance for 812K actors:
1. **Frustum culling** — only draw visible nodes
2. **Distance LOD** — StaticMesh LODs already in data
3. **Impostor rendering** — distant actors as colored billboards
4. **Tile-based streaming** — only load nearby .umap tile mesh data
5. **Instanced rendering** — shared meshes (cover nodes, path nodes) via GPU instancing

---

## 6. UI Layout — UnrealEd Faithful

> **Source**: `UnrealEd/Src/EditorFrame.cpp`, `MainToolBar.cpp`, `LevelViewportToolBar.cpp`,
> `StatusBars.cpp`, `PropertyWindow.cpp`, `Kismet.cpp`, `UnSequenceDraw.cpp`
> from `github.com/CodeRedModding/UnrealEngine3` (Build 10897)

### 6a. Main Window Structure

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Menu Bar                                                                │
│  File │ Edit │ View │ Brush │ Build │ Play │ Tools │ Preferences │ Help │
├─────────────────────────────────────────────────────────────────────────┤
│ Main Toolbar                                                            │
│  [New][Open][Save][SaveAll] │ [Undo][Redo] │ [T][R][S][N] [W/L] │     │
│  [Search][Content][Kismet] │ [2x2][1+2]... │ [Build][Play] │          │
├──────────┬──────────────────────────────────┬───────────┬───────────────┤
│          │ ┌──VP Toolbar──────────────────┐ │           │               │
│          │ │ [Opt][Front▼][RT][ ][Unlit▼] │ │           │               │
│          │ ├──────────────────────────────┤ │           │               │
│ Outliner │ │                              │ │ Properties│  Content      │
│  (dock)  │ │   Front (XZ)                 │ │  Window   │  Browser      │
│          │ │                              │ │  (dock)   │  (dock)       │
│          │ │                              │ │           │               │
│          │ ├──VP Toolbar──────────────────┤ │           │               │
│          │ │ [Opt][Top▼][RT][ ][Unlit▼]   │ │           │               │
│          │ ├──────────────────────────────┤ │           │               │
│          │ │                              │ │  Entity   │               │
│          │ │   Top (XY)                   │ │  Palette  │               │
│          │ │                              │ │  (dock)   │               │
│          ├──┬────────────────────────────┬┤ │           │               │
│          │  │ ┌──VP Toolbar────────────┐ ││ │           │               │
│          │  │ │ [Opt][Side▼][RT]...    │ ││ │           │               │
│          │  │ ├────────────────────────┤ ││ │           │               │
│          │  │ │                        │ ││ │           │               │
│          │  │ │   Side (YZ)            │ ││ │           │               │
│          │  │ │                        │ ││ │           │               │
│          │  ├─┼────────────────────────┤─┤│ │           │               │
│          │  │ │ [Opt][Persp▼][RT][🎮]  │ ││ │           │               │
│          │  │ ├────────────────────────┤ ││ │           │               │
│          │  │ │                        │ ││ │           │               │
│          │  │ │  Perspective (3D)      │ ││ │           │               │
│          │  │ │                        │ ││ │           │               │
│          │  │ └────────────────────────┘ ││ │           │               │
├──────────┴──┴────────────────────────────┴┴─┴───────────┴───────────────┤
│ Status Bar                                                               │
│  [Cmd>______] │ SCC │ Lighting │ Paths │ ActorName │ X,Y,Z │ G:10 R:5  │
└──────────────────────────────────────────────────────────────────────────┘
```

### 6b. Menu Bar

| Menu | Items | Notes |
|------|-------|-------|
| **File** | New Level, Open Level, Save, Save As, Save All, Import, Export, Recent Files | Export writes SQL seed + JSON |
| **Edit** | Undo, Redo, Cut, Copy, Paste, Duplicate, Delete, Select All, Select None, Select Inverse | Undo/redo stack in sgw-editor-core |
| **View** | Outliner, Properties, Content Browser, Kismet, Entity Palette, Log | Toggle dockable panels |
| **Brush** | Add, Subtract, Intersect, Deintersect | CSG ops — display only for SGW reference |
| **Build** | Build Geometry, Build Lighting, Build Paths, Build All | Triggers navmesh rebuild, lighting preview |
| **Play** | Play In Editor (PIE), Simulate | Connect to running Cimmeria server for live test |
| **Tools** | Map Check, Search Actors, Replace Actor, Find in Kismet | Validation and search utilities |
| **Preferences** | Grid, Snap, Viewport, Theme, Key Bindings | Persistent settings |
| **Help** | About, Documentation, Keyboard Shortcuts | Links to docs/ |

### 6c. Main Toolbar

Left-to-right button groups (matching `MainToolBar.cpp` `CreateMainToolBar()`):

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ [New][Open][Save][SaveAll] │ [Undo][Redo] │ [T][R][S][N] [W▼L] │            │
│ ─── File Ops ─────────────  ── Edit Ops ── ── Transform ──────── ── Coord ── │
│                                                                               │
│ [🔍Search][📦Content][⚡Kismet] │ [2x2][1+2][1H][1V] │ [⚙Build][▶Play] │   │
│ ─── Panels ───────────────────── ── Viewport Config ── ── Actions ────────── │
└───────────────────────────────────────────────────────────────────────────────┘
```

| Group | Buttons | Hotkeys |
|-------|---------|---------|
| File Ops | New, Open, Save, Save All | Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S |
| Edit Ops | Undo, Redo | Ctrl+Z, Ctrl+Y |
| Transform | Translate (T), Rotate (R), Scale (S), NonUniform Scale (N) | W, E, R, — |
| Coord System | World / Local toggle dropdown | Ctrl+~ |
| Panels | Search Actors, Content Browser, Kismet Editor | —, Ctrl+Shift+F, K |
| Viewport Config | 2×2, 1+2 (one big + two small), 1×1 Horizontal, 1×1 Vertical | — |
| Actions | Build All, Play In Editor | Ctrl+Shift+B, Alt+P |

### 6d. Viewport Container

Default layout: **2×2 split** (matching `Viewports.cpp`):

| Quadrant | View | Projection | Axes Shown |
|----------|------|------------|------------|
| Top-Left | **Front** | Orthographic | X horizontal, Z vertical |
| Top-Right | **Side** | Orthographic | Y horizontal, Z vertical |
| Bottom-Left | **Top** | Orthographic | X horizontal, Y vertical |
| Bottom-Right | **Perspective** | Perspective | Free orbit |

Layout configurations (toolbar buttons):
- **2×2** — Default, four equal viewports
- **1+2** — One large viewport + two stacked small
- **1×1 Horizontal** — Two viewports side-by-side
- **1×1 Vertical** — Two viewports top-and-bottom

Splitters are **drag-resizable**. Double-click a viewport title bar to maximize it (toggle).

### 6e. Per-Viewport Toolbar

Each viewport has its own toolbar strip (from `LevelViewportToolBar.cpp`):

```
[Options ▼] [Front ▼] [Realtime] [ ] [Unlit ▼] [🎮 GameView] [📷 Speed ▼] [↗ Tearoff]
```

| Control | Function |
|---------|----------|
| **Options ▼** | Show flags: actors, BSP, static meshes, volumes, paths, nav, grid, bounds |
| **Type Selector ▼** | Perspective, Front, Side, Top — changes viewport projection |
| **Realtime Toggle** | Enable/disable continuous rendering (vs render-on-demand) |
| **Render Mode ▼** | BrushWireframe, Wireframe, Unlit, Lit, DetailLighting, LightingOnly, LightComplexity, TextureDensity, ShaderComplexity, LightmapDensity |
| **Game View** | Toggle HUD/helper overlay visibility (show what player sees) |
| **Camera Speed ▼** | 1-8 speed presets for fly-through mode |
| **Tearoff** | Pop viewport into standalone floating window |

### 6f. Status Bar

Full-width strip at window bottom (from `StatusBars.cpp`):

```
┌──────────────┬─────┬──────────┬───────┬─────────────────┬───────────────┬──────────────────┬──────────┐
│ Cmd> _______ │ SCC │ Lighting │ Paths │ StaticMeshActor │ X:-1234 Y:567 │ G:10 R:512 S:0.5 │ Autosave │
│              │  ●  │    ✓     │   ✓   │  SM_Door_01     │    Z:89       │                  │  2:34    │
└──────────────┴─────┴──────────┴───────┴─────────────────┴───────────────┴──────────────────┴──────────┘
```

| Field | Source | Description |
|-------|--------|-------------|
| **Exec Combo** | `StatusBars.cpp` | Command input field — type console commands |
| **SCC Status** | `StatusBars.cpp` | Source control indicator (●/○) — maps to git status |
| **Lighting Built** | `StatusBars.cpp` | Checkmark if lighting is current |
| **Paths Built** | `StatusBars.cpp` | Checkmark if nav paths are current |
| **Actor Name** | `StatusBars.cpp` | Class + name of actor under cursor or selected |
| **Mouse Position** | `StatusBars.cpp` | World-space X, Y, Z at cursor |
| **DrawScale** | `StatusBars.cpp` | Selected actor's DrawScale, DrawScale3D.X/Y/Z |
| **Grid Snap** | `StatusBars.cpp` | Current grid size (1, 2, 4, 8, 10, 16, 32, 64, 128, 256) |
| **Rotation Snap** | `StatusBars.cpp` | Current rotation snap (512=~2.8°, 1024=~5.6°, etc.) |
| **Scale Snap** | `StatusBars.cpp` | Current scale snap (0.25, 0.5, 1.0) |
| **Autosave** | `StatusBars.cpp` | Countdown to next autosave |

### 6g. Property Window

Matches UE3's `PropertyWindow.cpp` hierarchy:

```
┌─────────────────────────────────────────┐
│ 🔍 Filter: [___________]  ⭐ Favorites │
├─────────────────────────────────────────┤
│ ▼ Display                               │
│    DrawScale          │  1.0            │
│    DrawScale3D        │  (1.0, 1.0, 1.0)│
│    bHidden            │  ☐              │
│ ▼ Movement                              │
│    Location           │  (-1234, 567, 89)│
│    Rotation           │  (0, 16384, 0)  │
│    Physics            │  PHYS_None ▼    │
│ ▼ Collision                             │
│    bCollideActors     │  ☑              │
│    bBlockActors       │  ☑              │
│ ▼ Object                                │
│    Class              │  StaticMeshActor│
│    Name               │  SM_Door_01     │
│    Tag                │  ___________    │
│ ▼ StaticMeshComponent                   │
│    StaticMesh         │  [SM_Door_A] 🔗 │
│    Materials[0]       │  [MI_Door_A] 🔗 │
│ ▶ Advanced (collapsed)                  │
├─────────────────────────────────────────┤
│ ⭐ Favorites:                           │
│    Location           │  (-1234, 567, 89)│
│    DrawScale          │  1.0            │
└─────────────────────────────────────────┘
```

**Architecture** (from `FObjectPropertyNode` hierarchy):
- **FObjectPropertyNode** — Root: represents the selected UObject
- **FCategoryPropertyNode** — Collapsible group (Display, Movement, Collision, etc.)
- **FItemPropertyNode** — Leaf: property name | value editor

**Features**:
- **Search/filter**: Type to filter properties by name — instant filtering
- **Favorites**: Star properties to pin them to a favorites section at bottom
- **Name|Value splitter**: Drag-resizable column divider
- **Multi-select editing**: When multiple actors selected, show common properties; mixed values shown as "—"
- **Object links**: 🔗 button on object references opens that asset in Content Browser
- **Enum dropdowns**: Enum properties render as dropdown selectors
- **Color properties**: Inline color swatch + picker dialog
- **Vector/Rotator**: Inline X/Y/Z fields with drag-to-adjust

### 6h. Kismet Node Visual Styling

Matches `UnSequenceDraw.cpp` and `Kismet.cpp` drawing routines:

**Node shapes by type**:

| Node Type | Shape | Title Bar Color | Connector Color |
|-----------|-------|----------------|-----------------|
| **SeqEvent_*** | Diamond / pointed-side rect | Red accent | Red (255, 0, 0) |
| **SeqAct_*** | Rounded rectangle | Gray (112, 112, 112) | Black (0, 0, 0) |
| **SeqCond_*** | Rounded rectangle | Gray (112, 112, 112) | Black |
| **SeqVar_*** | Circle / pill | Per-variable `ObjColor` | Matches ObjColor |
| **Sequence** | Rectangle (container) | Blue (112, 112, 200) | — |

**Variable type colors** (from `ObjColor` defaults):
- `SeqVar_Bool` — Red (255, 0, 0)
- `SeqVar_Int` — Cyan (0, 255, 255)
- `SeqVar_Float` — Green (0, 255, 0)
- `SeqVar_String` — Magenta (255, 0, 255)
- `SeqVar_Object` — Blue (0, 0, 255)
- `SeqVar_Named` — Orange (255, 128, 0)
- `SeqVar_External` — Purple (160, 0, 255)

**Connection wires**:
- Logic connections (output→input): **Black** lines, cubic bezier
- Event connections: **Red** lines
- Variable connections: **Match variable ObjColor**
- Hover highlight: **Yellow** (225, 225, 0)
- Selected wire: **White** glow
- Connection routing: Avoid sharp angles, route around nodes when possible

**Node interior**:
- Title: bold white text on colored title bar
- Body: dark gray (64, 64, 64) background
- `ObjComment`: Italic text below title (lighter gray)
- Input pins: Left edge, labeled
- Output pins: Right edge, labeled
- Variable pins: Bottom edge
- Event pins: Bottom edge (red diamond markers)

### 6i. Content Browser

```
┌─────────────────────────────────────────────────────────┐
│ 🔍 Search: [____________] │ Class: [All ▼] │ Zone: [...] │
├──────────────┬──────────────────────────────────────────┤
│ Package Tree │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │
│              │  │ T2D  │ │ T2D  │ │ SM   │ │ MAT  │   │
│ ▼ Castle     │  │[thumb]│ │[thumb]│ │[thumb]│ │[thumb]│   │
│   ▼ Meshes   │  │Name_A│ │Name_B│ │Mesh_C│ │Mat_D │   │
│     SM_Door  │  └──────┘ └──────┘ └──────┘ └──────┘   │
│     SM_Wall  │  ┌──────┐ ┌──────┐ ┌──────┐            │
│   ▼ Textures │  │ SND  │ │ T2D  │ │ SEQ  │            │
│     T_Brick  │  │[icon]│ │[thumb]│ │[icon]│            │
│   ▶ Materials│  │Snd_E │ │Tex_F │ │Seq_G │            │
│ ▶ Abydos     │  └──────┘ └──────┘ └──────┘            │
│ ▶ Dakara     │                                         │
├──────────────┴──────────────────────────────────────────┤
│ 847 assets │ 234 textures │ 412 meshes │ 201 other     │
└─────────────────────────────────────────────────────────┘
```

### 6j. Docking System

All panels (Outliner, Properties, Content Browser, Kismet, Entity Palette) are **dockable**:
- Dock to left/right/bottom edge
- Float as independent windows
- Tab-stack with other panels
- Show/hide via View menu
- Layout saved to `editor-layout.json` and restored on launch

Default layout:
- **Left dock**: Outliner
- **Center**: 2×2 Viewport Container
- **Right dock**: Property Window (top), Entity Palette (bottom, tabbed)
- **Bottom dock**: Content Browser (hidden by default, toggle with Ctrl+Shift+F)
- **Floating**: Kismet Editor (opens on demand via K or toolbar)

### 6k. UnrealEd → React Component Mapping

| UnrealEd Source | React Component | Key Behavior |
|-----------------|-----------------|--------------|
| `EditorFrame.cpp` | `EditorFrame.tsx` | Root layout: menu → toolbar → viewports → status |
| `MainToolBar.cpp` | `MainToolbar.tsx` | Grouped icon buttons with separators |
| `LevelViewportToolBar.cpp` | `ViewportToolbar.tsx` | Per-viewport options, type, render mode dropdowns |
| `Viewports.cpp` | `ViewportContainer.tsx` | 2×2 splitter, drag-resize, maximize toggle |
| `StatusBars.cpp` | `StatusBar.tsx` | ExecCombo + status indicators + position + snap values |
| `PropertyWindow.cpp` | `PropertyWindow.tsx` | Category tree, name\|value columns, favorites, filter |
| `GenericBrowser.cpp` | `ContentBrowser.tsx` | Package tree + thumbnail grid + search + class filter |
| `Kismet.cpp` | `KismetEditor.tsx` | React Flow canvas with custom node shapes/colors |
| `UnSequenceDraw.cpp` | `KismetNode.tsx` | Diamond/rect/circle shapes, colored title bars |

---

## 7. Database Integration

### Two data domains:

**Read-only: .umap actor data** — original SGW level design, displayed as reference geometry (gray/transparent).

**Read-write: Server entity placements** — from `resources.spawnlist`, `resources.spawn_sets`, `resources.spawn_points`, `resources.generic_regions`, `resources.stargates`, `resources.respawners`, `resources.entity_interactions`. Editable, saves back via sqlx.

### Workflow:
1. Load zone → see .umap actors as reference geometry
2. See DB entity placements overlaid as colored markers
3. Drag entities from Entity Palette onto viewport
4. Adjust via gizmos and property panel
5. Save to database, or export as SQL seed files

### Coordinate conversion (UE3 ↔ BigWorld):
```
UE3_X = BW_Z * 100    (cm vs m)
UE3_Y = BW_X * 100
UE3_Z = BW_Y * 100    (Z-up vs Y-up)
```

Viewport renders in UE3 space (since we visualize .umap data). DB positions convert on read/write.

---

## 8. Phased Implementation

### Phase 1: 2D Zone Viewer + Actor Markers (2-3 weeks)

**Goal**: Working Tauri app, load zone tiles, show actors on 2D top-down canvas.

**Deliverables**:
- `tools/SceneEditor/` Tauri 2 app skeleton (pattern from ContentEditor)
- Tauri commands: `load_zone(name)`, `list_actors(filters)`, `get_actor_details(id)`
- React frontend: Outliner, 2D Canvas (HTML Canvas orthographic), PropertyPanel
- Zone selector from `CookedPC/Maps/` directories
- Actor class filtering checkboxes
- Click-to-select, show properties in inspector
- Status bar with zone stats

**No GPU, no mesh decoding.** Actor positions as colored dots/icons.

### Phase 2: Database Entity Overlay + Placement (2-3 weeks)

**Goal**: Overlay server DB entities, add placement and editing.

**Deliverables**:
- DB connection dialog (reuse ContentEditor pattern)
- Load/display spawnlist, spawn_sets, generic_regions, stargates as colored markers
- Entity Palette from entity_templates table
- Click-to-place entities on canvas
- Drag to move, property editing
- Save to DB via sqlx, export as SQL seed files
- Undo/redo stack (Rust-side)
- UE3 ↔ BigWorld coordinate conversion

### Phase 3: Package Index + Texture Preview (2-3 weeks)

**Goal**: Cross-package asset index, texture decoding for asset browser.

**Deliverables**:
- `crates/upk-objects/` crate with `Texture2D` deserializer
- DXT decompression (native wgpu BC or `texture2ddecoder` for thumbnails)
- Package index builder: scan all exports, cache to disk (bincode)
- Asset Browser panel: tree view, object list, texture thumbnails
- Cross-package search by class/name

### Phase 4: StaticMesh Deserialization (3-4 weeks)

**Goal**: Decode UE3 StaticMesh into renderable vertex/index buffers.

**Deliverables**:
- `upk-objects::StaticMesh` deserializer (FUntypedBulkData, LOD models, buffers)
- Validation against umodel-extracted meshes
- Basic MaterialInstanceConstant parser (diffuse texture reference)
- Cross-package mesh resolution (follow .umap imports to .upk)
- Unit tests against known SGW meshes

**Risk mitigation**: Compare with umodel output. Use Ghidra if formats diverge.

### Phase 5: wgpu 3D Viewport (3-4 weeks)

**Goal**: Replace 2D canvas with 3D rendering of decoded meshes.

**Deliverables**:
- `crates/sgw-renderer/` crate: wgpu device, forward rendering pipeline
- Orbit camera with mouse controls
- Ground grid rendering
- StaticMeshActor rendering (resolve → decode → upload → draw)
- Placeholder markers for non-mesh actors (lights, triggers, volumes)
- Frustum culling
- Tile-based streaming (load/unload as camera moves)
- Integration with Tauri (separate winit window or render-to-texture)

### Phase 6: Selection, Gizmos, Scene Editing (2-3 weeks)

**Goal**: Interactive 3D editing.

**Deliverables**:
- GPU color-ID picking
- Selection outline shader
- Translation/rotation/scale gizmos
- Multi-select (Ctrl+click, box select)
- Transform snapping
- Undo/redo for gizmo operations
- Focus-on-selection (F key)

### Phase 7: Terrain + Environment (2-3 weeks)

**Goal**: Render terrain and basic environment.

**Deliverables**:
- `upk-objects::Terrain` deserializer (heightmap)
- Terrain mesh generation from heightmap
- Basic terrain texturing (single color or first layer)
- Sky dome placeholder, directional light
- BSP wireframe for blocking volumes

### Phase 8: Kismet Viewer (2 weeks)

**Goal**: Visual Kismet graph integrated into editor.

**Deliverables**:
- React Flow Kismet graph (pattern from ContentEditor ScriptEditor)
- Load from zone .umap files via existing `objects::kismet` extraction
- Custom node components for SeqEvent/SeqAct/SeqCond/SeqVar
- Link rendering from existing wiring data
- Click node → highlight associated actor in viewport
- Search/filter by class

### Phase 9: Advanced Features (ongoing)

- Multi-texture material rendering
- Light visualization (cones, radii)
- NavMesh overlay
- Sound emitter radius spheres
- Foliage/SpeedTree rendering
- Zone portals and streaming volumes
- Multi-zone view
- Performance optimization

---

## 9. Risk Assessment

### HIGH Risk

| Risk | Impact | Mitigation |
|------|--------|------------|
| **StaticMesh deserialization** | No 3D meshes → dot viewer only | Use umodel as oracle; Ghidra for SGW-specific format |
| **wgpu + Tauri integration** | Complex multi-window coordination | Start with 2D canvas (Phase 1-2); render-to-texture fallback |
| **Scene scale (812K actors)** | Performance/memory issues | Tile streaming, frustum culling, instancing, LOD |

### MEDIUM Risk

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Terrain deserialization** | No ground surface | Simple heightfield mesh; attempt after StaticMesh works |
| **Cross-package references** | Can't resolve mesh imports | Global package index (50s build, cached) |
| **Coordinate confusion** | Misplaced entities | Explicit conversion at every boundary, unit tests |

### LOW Risk / Can Punt

| Item | Notes |
|------|-------|
| Full material graphs | Basic diffuse texture covers 80% |
| BSP geometry | Wireframe outlines sufficient |
| Sound/VFX | Visual niceties for later |
| Kismet editing (vs viewing) | Content engine uses DB chains instead |

---

## 10. New Dependencies

```toml
# Workspace Cargo.toml additions
wgpu = "25"                         # GPU rendering
winit = "0.30"                      # Window management
glam = "0.30"                       # Math (Vec3, Mat4, Quat)
image = "0.25"                      # Image encoding (thumbnails, screenshots)
texture2ddecoder = "0.0.5"          # DXT decompression (CPU fallback)
bincode = "2"                       # Package index cache
notify = "8"                        # File watcher
rfd = "0.15"                        # Native file dialogs
```

---

## Related Documents

- [SGW Editor Tools Plan](sgw-editor-tools-plan.md) — Parent plan, sprint overview
- [CME Framework](../engine/cme-framework.md) — SGW's UE3 modifications, coordinate conversion
- [Source Reconstruction Feasibility](../technical/source-reconstruction-feasibility.md) — What's recoverable from SGW.exe
- [Editor Source Mapping](../reverse-engineering/editor-source-mapping.md) — Ghidra ↔ UE3 function mapping
