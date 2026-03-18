import * as THREE from 'three';

/**
 * Class-specific indicator geometries for non-mesh actors.
 * Cached globally so they're shared across instances.
 */

const LIGHT_SPHERE = new THREE.SphereGeometry(200, 16, 12);
const LIGHT_CONE = new THREE.ConeGeometry(150, 400, 16, 1, true);
const VOLUME_BOX = new THREE.BoxGeometry(200, 200, 200);
const PATH_DIAMOND = new THREE.OctahedronGeometry(25, 0);
const SPAWN_CYLINDER = new THREE.CylinderGeometry(40, 40, 60, 12);
const ARROW_GEO = new THREE.ConeGeometry(20, 80, 8);
const DEFAULT_GEO = new THREE.OctahedronGeometry(30, 0);
const STARGATE_TORUS = new THREE.TorusGeometry(60, 10, 16, 16);
const DOOR_PLANE = new THREE.PlaneGeometry(40, 80);
const PLAYER_CAPSULE = new THREE.CapsuleGeometry(20, 60, 8, 12);
const TELEPORTER_OCTA = new THREE.OctahedronGeometry(30, 0);

// Wireframe material cache
const wireframeMaterials = new Map<number, THREE.MeshBasicMaterial>();

function getWireframeMaterial(color: number): THREE.MeshBasicMaterial {
  let mat = wireframeMaterials.get(color);
  if (!mat) {
    mat = new THREE.MeshBasicMaterial({ color, wireframe: true, transparent: true, opacity: 0.6 });
    wireframeMaterials.set(color, mat);
  }
  return mat;
}

interface ClassVisual {
  geometry: THREE.BufferGeometry;
  wireframe: boolean;
}

const CLASS_VISUALS: Record<string, ClassVisual> = {
  PointLight:        { geometry: LIGHT_SPHERE, wireframe: true },
  SpotLight:         { geometry: LIGHT_CONE, wireframe: true },
  DirectionalLight:  { geometry: ARROW_GEO, wireframe: false },
  BlockingVolume:    { geometry: VOLUME_BOX, wireframe: true },
  TriggerVolume:     { geometry: VOLUME_BOX, wireframe: true },
  PhysicsVolume:     { geometry: VOLUME_BOX, wireframe: true },
  PostProcessVolume: { geometry: VOLUME_BOX, wireframe: true },
  PathNode:          { geometry: PATH_DIAMOND, wireframe: false },
  CoverLink:         { geometry: PATH_DIAMOND, wireframe: false },
  SGWSpecCoverNode:  { geometry: PATH_DIAMOND, wireframe: false },
  SGWSpawnRegion:    { geometry: SPAWN_CYLINDER, wireframe: true },
  SGWSpawnSet:       { geometry: SPAWN_CYLINDER, wireframe: true },
  PlayerStart:       { geometry: PLAYER_CAPSULE, wireframe: false },
  SGWStargate:       { geometry: STARGATE_TORUS, wireframe: true },
  SGWDoor:           { geometry: DOOR_PLANE, wireframe: false },
  SGWTeleporter:     { geometry: TELEPORTER_OCTA, wireframe: true },
};

/**
 * Get the appropriate indicator geometry for an actor class.
 * Returns the geometry and whether it should be wireframe.
 */
export function getClassGeometry(className: string): { geometry: THREE.BufferGeometry; wireframe: boolean } {
  return CLASS_VISUALS[className] ?? { geometry: DEFAULT_GEO, wireframe: false };
}

/**
 * Check if a class name represents a volume type.
 */
export function isVolumeClass(className: string): boolean {
  return className.endsWith('Volume') || className === 'Brush';
}

/**
 * Check if a class name represents a light type.
 */
export function isLightClass(className: string): boolean {
  return className.endsWith('Light');
}

// ---- Radius visualization helpers ----

const ACTOR_RADII: Record<string, number> = {
  PointLight: 200,
  PointLightMovable: 200,
  SpotLight: 300,
  AmbientSoundSGW: 500,
  AmbientSound: 500,
  SGWSpawnRegion: 800,
  SGWSpawnSet: 400,
  Trigger: 300,
  TriggerVolume: 300,
  BlockingVolume: 250,
  PhysicsVolume: 250,
};

/**
 * Returns the visualization radius for actors that have one, or null for classes without a radius.
 */
export function getActorRadius(className: string): number | null {
  return ACTOR_RADII[className] ?? null;
}

/**
 * Returns a Three.js hex color for the radius visualization ring.
 */
export function getActorRadiusColor(className: string): number {
  if (className.endsWith('Light')) return 0xffdd44;
  if (className.startsWith('AmbientSound')) return 0x44aaff;
  if (className.startsWith('SGWSpawn')) return 0xff6644;
  if (className.startsWith('Trigger')) return 0x44ff88;
  if (className.endsWith('Volume')) return 0xaa88ff;
  return 0x888888;
}

// ---- Connection line helpers ----

const CONNECTION_CLASSES = new Set([
  'PathNode',
  'SGWCoverSet',
  'CoverLink',
  'SGWSpecCoverNode',
  'SGWSpawnRegion',
]);

/**
 * Returns true for classes that show connection lines to nearby same-class actors.
 */
export function getConnectionTargets(className: string): boolean {
  return CONNECTION_CLASSES.has(className);
}

const MAX_CONNECTION_DISTANCES: Record<string, number> = {
  PathNode: 1500,
  SGWCoverSet: 800,
  CoverLink: 800,
  SGWSpecCoverNode: 800,
  SGWSpawnRegion: 2000,
};

/**
 * Max distance for connection lines between same-class actors.
 */
export function getMaxConnectionDistance(className: string): number {
  return MAX_CONNECTION_DISTANCES[className] ?? 1000;
}

// ---- Class category helpers ----

const SPAWN_CLASSES = new Set([
  'SGWSpawnRegion',
  'SGWSpawnSet',
  'SGWMob',
  'SGWNpc',
  'PlayerStart',
]);

/**
 * Returns true for spawn-related classes.
 */
export function isSpawnClass(className: string): boolean {
  return SPAWN_CLASSES.has(className);
}

const SGW_CLASSES = new Set([
  'SGWStargate',
  'SGWInteractable',
  'SGWDoor',
  'SGWTeleporter',
  'SGWMinigame',
]);

/**
 * Returns true for SGW gameplay object classes.
 */
export function isSGWClass(className: string): boolean {
  return SGW_CLASSES.has(className);
}

// ---- Actor icon labels ----

const ACTOR_ICONS: Record<string, string> = {
  PointLight: 'L',
  PointLightMovable: 'L',
  SpotLight: 'L',
  DirectionalLight: 'L',
  AmbientSoundSGW: 'S',
  AmbientSoundSimple: 'S',
  AmbientSound: 'S',
  PathNode: 'P',
  PlayerStart: 'PS',
  SGWMob: 'M',
  SGWNpc: 'N',
  SGWStargate: 'SG',
  SGWInteractable: 'I',
  SGWDoor: 'D',
  CoverLink: 'C',
  SGWCoverSet: 'C',
  SGWSpecCoverNode: 'C',
};

/**
 * Returns a short 1-2 char text label for overlay display on actor indicators.
 */
export function getActorIcon(className: string): string {
  const exact = ACTOR_ICONS[className];
  if (exact !== undefined) return exact;
  if (className.startsWith('Trigger')) return 'T';
  if (className.endsWith('Volume')) return 'V';
  return '';
}
