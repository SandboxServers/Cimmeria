export interface ZoneInfo {
  name: string;
  path: string;
  tile_count: number;
}

export interface ZoneSummary {
  zone_name: string;
  tile_count: number;
  actor_count: number;
  class_counts: Record<string, number>;
  bounds: ZoneBounds;
}

export interface ZoneBounds {
  min_x: number; max_x: number;
  min_y: number; max_y: number;
  min_z: number; max_z: number;
}

export interface ActorEntry {
  key: string;
  class_name: string;
  object_name: string;
  tile: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
  pitch: number;
  roll: number;
  draw_scale: number;
  draw_scale_x: number;
  draw_scale_y: number;
  draw_scale_z: number;
  static_mesh: string;
  properties: PropertyPair[];
}

export interface PropertyPair {
  name: string;
  value: string;
}

export interface ActorListEntry {
  key: string;
  class_name: string;
  object_name: string;
  tile: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
  pitch: number;
  roll: number;
  draw_scale: number;
  draw_scale_x: number;
  draw_scale_y: number;
  draw_scale_z: number;
  static_mesh: string;
}

/** Map class names to display colors for canvas rendering. */
export const ACTOR_COLORS: Record<string, string> = {
  StaticMeshActor: '#6b7280',
  InterpActor: '#a855f7',
  PointLight: '#fbbf24',
  SpotLight: '#f59e0b',
  DirectionalLight: '#fde68a',
  Emitter: '#06b6d4',
  AmbientSoundSimple: '#10b981',
  AmbientSoundSGW: '#10b981',
  AmbientSound: '#10b981',
  CoverLink: '#ef4444',
  SGWSpecCoverNode: '#ef4444',
  PathNode: '#3b82f6',
  Trigger: '#f97316',
  TriggerVolume: '#fb923c',
  BlockingVolume: '#64748b',
  PhysicsVolume: '#64748b',
  SGWSpawnRegion: '#22c55e',
  SGWSpawnSet: '#22c55e',
  SGWMob: '#dc2626',
  SGWNpc: '#8b5cf6',
  SGWStargate: '#0ea5e9',
  SGWInteractable: '#06b6d4',
  SGWDoor: '#a855f7',
  SGWTeleporter: '#0ea5e9',
  SGWMinigame: '#14b8a6',
  PlayerStart: '#14b8a6',
  Brush: '#475569',
  Terrain: '#84cc16',
  WorldInfo: '#fde68a',
  SkeletalMeshActor: '#c084fc',
  DecalActor: '#94a3b8',
  SpeedTreeActor: '#22c55e',
  KActor: '#f472b6',
  _default: '#9ca3af',
};

/** Mesh geometry returned from `load_mesh` IPC command. */
export interface MeshData {
  positions: number[];
  normals: number[];
  uvs: number[];
  indices: number[];
  /** [origin_x, origin_y, origin_z, extent_x, extent_y, extent_z] */
  bounds: [number, number, number, number, number, number];
}

export interface DbStatus {
  connected: boolean;
  host: string;
  dbname: string;
}

export interface WorldInfo {
  world_id: number;
  world_name: string;
  client_map: string;
}

/** A server-side entity placement from the database. */
export interface DbEntity {
  id: number;
  source_table: string;    // "spawnlist" | "spawn_points" | "generic_regions" | "stargates"
  entity_type: string;     // SGWMob, SGWNpc, Region, Stargate, etc.
  name: string;
  x: number;               // UE3 centimeters (already converted from BigWorld by backend)
  y: number;
  z: number;
  heading: number;          // degrees
  world_id: number;
  template_id: number | null;
  tag: string | null;
  radius: number | null;
  height: number | null;
  handler: string | null;
  gate_address: string | null;
}

export interface EntityTemplate {
  template_id: number;
  template_name: string;
  class: string;
  name: string | null;
  static_mesh: string | null;
  level: number | null;
}

/** Colors for DB entity markers - distinct from .umap actor colors */
export const DB_ENTITY_COLORS: Record<string, string> = {
  SGWMob: '#ff4444',
  SGWNpc: '#aa55ff',
  SGWSpawnRegion: '#44ff44',
  Region: '#ffaa00',
  Stargate: '#00ccff',
  SpawnPoint: '#44ff88',
  _default: '#ff8800',
};

export type ViewportType = 'top' | 'front' | 'side' | 'perspective';

export type RenderMode = 'wireframe' | 'unlit' | 'lit';

export interface ViewportState {
  type: ViewportType;
  zoom: number;
  panX: number;
  panY: number;
  realtime: boolean;
  renderMode: RenderMode;
  showGrid: boolean;
  showActors: boolean;
}

// ---- Content Browser types ----

export interface PackageScanResult {
  package_count: number;
  total_exports: number;
  class_counts: [string, number][];
}

export interface PackageInfo {
  name: string;
  export_count: number;
}

export interface ExportInfo {
  index: number;
  class_name: string;
  object_name: string;
  full_path: string;
  serial_size: number;
}

export interface SearchResult {
  package_name: string;
  export_index: number;
  class_name: string;
  object_name: string;
  full_path: string;
}

export interface TextureThumbnail {
  width: number;
  height: number;
  format: string;
  data_base64: string;
}

export interface NavMeshData {
  vertices: number[];
  area_ids: number[];
  bounds_min: [number, number, number];
  bounds_max: [number, number, number];
  poly_count: number;
  tri_count: number;
  agent_height: number;
  agent_radius: number;
}

export interface NavMeshInfo {
  name: string;
  path: string;
  size_bytes: number;
}
