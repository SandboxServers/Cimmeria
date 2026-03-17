import { useRef, useEffect, useCallback, useState, useMemo } from 'react';
import * as THREE from 'three';
import { invoke } from '../lib/tauri';
import type { ActorListEntry, MeshData, RenderMode, ZoneBounds, DbEntity, NavMeshData } from '../lib/types';
import { ACTOR_COLORS, DB_ENTITY_COLORS } from '../lib/types';
import { createGizmo, updateGizmoScale, highlightGizmoAxis, pickGizmoAxis, type GizmoMode } from './TransformGizmo';
import { getClassGeometry } from './ActorVisuals';

interface Viewport3DProps {
  actors: ActorListEntry[];
  bounds: ZoneBounds | null;
  selectedKeys: Set<string>;
  showGrid: boolean;
  renderMode: RenderMode;
  focusRequest: number;
  transformMode: 'move' | 'rotate' | 'scale';
  gridSnap: number;
  onSelect: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onMouseMove: (pos: { x: number; y: number; z: number }) => void;
  onMoveActors: (keys: string[], newX: number[], newY: number[], newZ: number[]) => void;
  onRotateActors: (keys: string[], newPitch: number[], newYaw: number[], newRoll: number[]) => void;
  onScaleActors: (keys: string[], newDS: number[], newDSX: number[], newDSY: number[], newDSZ: number[]) => void;
  onBoxSelect: (keys: string[]) => void;
  onContextMenu?: (x: number, y: number) => void;
  placementMode?: boolean;
  onPlaceSpawn?: (x: number, y: number, z: number) => void;
  selectedTemplateName?: string | null;
  dbEntities?: DbEntity[];
  selectedDbEntityIds?: Set<number>;
  onSelectDbEntity?: (id: number) => void;
  onUpdateSpawnPosition?: (spawnId: number, x: number, y: number, z: number, heading: number) => void;
  navMeshData?: NavMeshData | null;
}

interface GizmoDragState {
  mode: 'move' | 'rotate' | 'scale';
  axis: 'x' | 'y' | 'z';
  axisDir: THREE.Vector3;
  pivotStart: THREE.Vector3;
  startParam: number;
  // Move mode
  originalUE3: Map<string, { x: number; y: number; z: number }>;
  originalThree: Map<string, THREE.Vector3>;
  // Rotate mode
  originalRotUE3: Map<string, { pitch: number; yaw: number; roll: number }>;
  originalEulers: Map<string, THREE.Euler>;
  // Scale mode
  originalScaleUE3: Map<string, { ds: number; dsx: number; dsy: number; dsz: number }>;
  originalScaleThree: Map<string, THREE.Vector3>;
}

const AXIS_DIRS: Record<string, THREE.Vector3> = {
  x: new THREE.Vector3(1, 0, 0),
  y: new THREE.Vector3(0, 1, 0),
  z: new THREE.Vector3(0, 0, 1),
};

/**
 * Project a screen point onto a constrained 3D axis.
 * Returns the parameter along the axis (distance from origin).
 */
function projectMouseOnAxis(
  clientX: number,
  clientY: number,
  rect: DOMRect,
  camera: THREE.PerspectiveCamera,
  axisDir: THREE.Vector3,
  origin: THREE.Vector3,
): number | null {
  const ndc = new THREE.Vector2(
    ((clientX - rect.left) / rect.width) * 2 - 1,
    -((clientY - rect.top) / rect.height) * 2 + 1,
  );

  const raycaster = new THREE.Raycaster();
  raycaster.setFromCamera(ndc, camera);

  // Build a plane containing the axis, oriented to face the camera
  const toCamera = new THREE.Vector3().subVectors(camera.position, origin);
  const planeNormal = new THREE.Vector3()
    .crossVectors(axisDir, toCamera)
    .cross(axisDir)
    .normalize();

  // Fallback if camera looks straight down the axis
  if (planeNormal.lengthSq() < 0.0001) {
    planeNormal.set(0, 1, 0);
    if (Math.abs(axisDir.dot(planeNormal)) > 0.9) planeNormal.set(1, 0, 0);
    planeNormal.crossVectors(planeNormal, axisDir).normalize();
  }

  const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(planeNormal, origin);
  const intersection = new THREE.Vector3();
  if (!raycaster.ray.intersectPlane(plane, intersection)) return null;

  return intersection.sub(origin).dot(axisDir);
}

/**
 * Project a screen point to an angle on the rotation plane perpendicular to the axis.
 */
function projectMouseToAngle(
  clientX: number,
  clientY: number,
  rect: DOMRect,
  camera: THREE.PerspectiveCamera,
  axisDir: THREE.Vector3,
  origin: THREE.Vector3,
): number | null {
  const ndc = new THREE.Vector2(
    ((clientX - rect.left) / rect.width) * 2 - 1,
    -((clientY - rect.top) / rect.height) * 2 + 1,
  );
  const raycaster = new THREE.Raycaster();
  raycaster.setFromCamera(ndc, camera);

  const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(axisDir, origin);
  const intersection = new THREE.Vector3();
  if (!raycaster.ray.intersectPlane(plane, intersection)) return null;

  const local = intersection.sub(origin);
  let temp = new THREE.Vector3(1, 0, 0);
  if (Math.abs(axisDir.dot(temp)) > 0.9) temp.set(0, 0, 1);
  const basis1 = new THREE.Vector3().crossVectors(axisDir, temp).normalize();
  const basis2 = new THREE.Vector3().crossVectors(axisDir, basis1).normalize();
  return Math.atan2(local.dot(basis2), local.dot(basis1));
}

const ROT_TO_RAD = (2 * Math.PI) / 65536;
// fallback only — class-specific geo is in ActorVisuals.ts
const geometryCache = new Map<string, THREE.BufferGeometry>();
const loadingMeshes = new Set<string>();

function createGeometryFromMeshData(data: MeshData): THREE.BufferGeometry {
  const geo = new THREE.BufferGeometry();
  geo.setAttribute('position', new THREE.Float32BufferAttribute(data.positions, 3));
  geo.setAttribute('normal', new THREE.Float32BufferAttribute(data.normals, 3));
  geo.setAttribute('uv', new THREE.Float32BufferAttribute(data.uvs, 2));
  geo.setIndex(new THREE.Uint32BufferAttribute(data.indices, 1));
  geo.computeBoundingSphere();
  return geo;
}

function getMeshMaterial(renderMode: RenderMode, isSelected: boolean): THREE.Material {
  const color = isSelected ? 0xf59e0b : 0x8899aa;
  switch (renderMode) {
    case 'wireframe':
      return new THREE.MeshBasicMaterial({ color, wireframe: true });
    case 'unlit':
      return new THREE.MeshBasicMaterial({ color, side: THREE.DoubleSide });
    case 'lit':
      return new THREE.MeshStandardMaterial({ color, roughness: 0.7, metalness: 0.1, side: THREE.DoubleSide });
  }
}

function getIndicatorMaterial(className: string, isSelected: boolean, wireframe: boolean): THREE.Material {
  const baseColor = ACTOR_COLORS[className] ?? ACTOR_COLORS._default;
  const color = isSelected ? 0xf59e0b : parseInt(baseColor.replace('#', ''), 16);
  if (wireframe) {
    return new THREE.MeshBasicMaterial({ color, wireframe: true, transparent: true, opacity: isSelected ? 0.8 : 0.5 });
  }
  return new THREE.MeshBasicMaterial({ color });
}

function placeObject(obj: THREE.Object3D, actor: ActorListEntry) {
  // UE3 (X-forward, Y-right, Z-up) -> Three.js (X-right, Y-up, Z-into-screen)
  obj.position.set(actor.x, actor.z, actor.y);
  obj.setRotationFromEuler(new THREE.Euler(
    actor.pitch * ROT_TO_RAD,
    actor.yaw * ROT_TO_RAD,
    actor.roll * ROT_TO_RAD,
    'YXZ',
  ));
  const s = actor.draw_scale || 1;
  obj.scale.set(s * (actor.draw_scale_x || 1), s * (actor.draw_scale_z || 1), s * (actor.draw_scale_y || 1));
}

function disposeMaterials(obj: THREE.Object3D) {
  if (obj instanceof THREE.Mesh) {
    const mat = obj.material;
    if (Array.isArray(mat)) mat.forEach(m => m.dispose());
    else if (mat) mat.dispose();
  }
}

export function Viewport3D({
  actors,
  bounds,
  selectedKeys,
  showGrid,
  renderMode,
  focusRequest,
  transformMode,
  gridSnap,
  onSelect,
  onToggleSelect,
  onAddSelect,
  onMouseMove,
  onMoveActors,
  onRotateActors,
  onScaleActors,
  onBoxSelect,
  onContextMenu: onContextMenuProp,
  placementMode,
  onPlaceSpawn,
  selectedTemplateName,
  dbEntities = [],
  selectedDbEntityIds = new Set(),
  onSelectDbEntity,
  onUpdateSpawnPosition,
  navMeshData = null,
}: Viewport3DProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const frameIdRef = useRef(0);
  const meshGroupRef = useRef<THREE.Group | null>(null);
  const gridRef = useRef<THREE.GridHelper | null>(null);
  const actorObjMap = useRef<Map<string, THREE.Object3D>>(new Map());
  const outlineGroupRef = useRef<THREE.Group | null>(null);
  const radiusGroupRef = useRef<THREE.Group | null>(null);
  const connectionGroupRef = useRef<THREE.Group | null>(null);
  const dbEntityGroupRef = useRef<THREE.Group | null>(null);
  const navMeshGroupRef = useRef<THREE.Group | null>(null);
  const gizmoRef = useRef<THREE.Group | null>(null);
  const gizmoModeRef = useRef<GizmoMode>(transformMode);
  const hoveredAxisRef = useRef<'x' | 'y' | 'z' | null>(null);
  const gizmoDragRef = useRef<GizmoDragState | null>(null);
  const initialFitDone = useRef(false);
  const needsRender = useRef(true);
  const renderModeRef = useRef(renderMode);
  renderModeRef.current = renderMode;
  const gridSnapRef = useRef(gridSnap);
  gridSnapRef.current = gridSnap;

  // Refs for current state (avoids stale closures in event handlers)
  const actorsRef = useRef(actors);
  actorsRef.current = actors;
  const selectedKeysRef = useRef(selectedKeys);
  selectedKeysRef.current = selectedKeys;
  const placementModeRef = useRef(placementMode);
  placementModeRef.current = placementMode;
  const onPlaceSpawnRef = useRef(onPlaceSpawn);
  onPlaceSpawnRef.current = onPlaceSpawn;

  // O(1) actor lookup map — avoids O(n) .find() on large actor lists
  const actorByKey = useMemo(() => {
    const m = new Map<string, ActorListEntry>();
    for (const a of actors) m.set(a.key, a);
    return m;
  }, [actors]);
  const actorByKeyRef = useRef(actorByKey);
  actorByKeyRef.current = actorByKey;

  // Camera orbit state
  const orbitRef = useRef({
    theta: -Math.PI / 4,
    phi: Math.PI / 4,
    distance: 5000,
    target: new THREE.Vector3(0, 0, 0),
    isDragging: false,
    isPanning: false,
    lastX: 0,
    lastY: 0,
  });

  // Box select state
  const boxSelectRef = useRef<{ startX: number; startY: number; active: boolean } | null>(null);
  const [boxSelectRect, setBoxSelectRect] = useState<{ left: number; top: number; width: number; height: number } | null>(null);

  // Gizmo drag info display
  const [dragInfo, setDragInfo] = useState<string | null>(null);
  const setDragInfoRef = useRef(setDragInfo);
  setDragInfoRef.current = setDragInfo;

  // Camera bookmarks (Ctrl+1-9 save, 1-9 restore)
  const cameraBookmarks = useRef<Map<string, { theta: number; phi: number; distance: number; target: THREE.Vector3 }>>(new Map());

  // RMB click vs orbit detection
  const rmbStartRef = useRef<{ x: number; y: number } | null>(null);

  // WASD fly state
  const keysDown = useRef(new Set<string>());
  const flyActive = useRef(false);

  // ---- Update camera from orbit state ----
  const updateCamera = useCallback(() => {
    const camera = cameraRef.current;
    if (!camera) return;
    const o = orbitRef.current;
    camera.position.set(
      o.target.x + o.distance * Math.sin(o.phi) * Math.cos(o.theta),
      o.target.y + o.distance * Math.cos(o.phi),
      o.target.z + o.distance * Math.sin(o.phi) * Math.sin(o.theta),
    );
    camera.lookAt(o.target);
    needsRender.current = true;
  }, []);

  // Ref so the keyboard handler (in a one-time useEffect) can always call the latest version
  const updateCameraRef = useRef(updateCamera);
  updateCameraRef.current = updateCamera;

  // ---- Focus camera on an actor position ----
  const focusOnActor = useCallback((actor: ActorListEntry) => {
    const o = orbitRef.current;
    o.target.set(actor.x, actor.z, actor.y);
    // Pull in to a reasonable distance based on mesh bounds or default
    const geo = actor.static_mesh ? geometryCache.get(actor.static_mesh) : null;
    if (geo?.boundingSphere) {
      o.distance = Math.max(geo.boundingSphere.radius * 3, 200);
    } else {
      o.distance = 500;
    }
    updateCamera();
  }, [updateCamera]);

  // ---- Initialize Three.js ----
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    renderer.setPixelRatio(window.devicePixelRatio);
    renderer.setClearColor(0x0f1419);
    container.appendChild(renderer.domElement);
    rendererRef.current = renderer;

    const scene = new THREE.Scene();
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(60, 1, 1, 500000);
    cameraRef.current = camera;

    scene.add(new THREE.AmbientLight(0xffffff, 0.4));
    const dirLight = new THREE.DirectionalLight(0xffffff, 0.8);
    dirLight.position.set(1, 2, 1).normalize();
    scene.add(dirLight);

    const meshGroup = new THREE.Group();
    scene.add(meshGroup);
    meshGroupRef.current = meshGroup;

    const outlineGroup = new THREE.Group();
    scene.add(outlineGroup);
    outlineGroupRef.current = outlineGroup;

    const radiusGroup = new THREE.Group();
    scene.add(radiusGroup);
    radiusGroupRef.current = radiusGroup;

    const connectionGroup = new THREE.Group();
    scene.add(connectionGroup);
    connectionGroupRef.current = connectionGroup;

    const dbEntityGroup = new THREE.Group();
    scene.add(dbEntityGroup);
    dbEntityGroupRef.current = dbEntityGroup;

    const navMeshGroup = new THREE.Group();
    scene.add(navMeshGroup);
    navMeshGroupRef.current = navMeshGroup;

    const grid = new THREE.GridHelper(100000, 100, 0x333333, 0x222222);
    scene.add(grid);
    gridRef.current = grid;

    const rect = container.getBoundingClientRect();
    renderer.setSize(rect.width, rect.height);
    camera.aspect = rect.width / rect.height;
    camera.updateProjectionMatrix();

    const obs = new ResizeObserver(entries => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          renderer.setSize(width, height);
          camera.aspect = width / height;
          camera.updateProjectionMatrix();
          needsRender.current = true;
        }
      }
    });
    obs.observe(container);

    // WASD fly loop — runs inside the render loop
    const FLY_SPEED = 5;
    const loop = () => {
      frameIdRef.current = requestAnimationFrame(loop);

      // Process WASD movement
      const keys = keysDown.current;
      if (keys.size > 0 && flyActive.current) {
        const o = orbitRef.current;
        const forward = new THREE.Vector3();
        camera.getWorldDirection(forward);
        const right = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
        const speed = o.distance * 0.005 * FLY_SPEED;

        if (keys.has('w')) o.target.addScaledVector(forward, speed);
        if (keys.has('s')) o.target.addScaledVector(forward, -speed);
        if (keys.has('a')) o.target.addScaledVector(right, -speed);
        if (keys.has('d')) o.target.addScaledVector(right, speed);
        if (keys.has('q') || keys.has('e')) {
          const up = keys.has('e') ? speed : -speed;
          o.target.y += up;
        }

        camera.position.set(
          o.target.x + o.distance * Math.sin(o.phi) * Math.cos(o.theta),
          o.target.y + o.distance * Math.cos(o.phi),
          o.target.z + o.distance * Math.sin(o.phi) * Math.sin(o.theta),
        );
        camera.lookAt(o.target);
        needsRender.current = true;
      }

      // Keep gizmo scaled to constant screen size
      if (gizmoRef.current) {
        updateGizmoScale(gizmoRef.current, camera);
      }

      if (needsRender.current) {
        renderer.render(scene, camera);
        needsRender.current = false;
      }
    };
    loop();

    // Keyboard handlers for WASD + camera bookmarks + Escape to cancel drag
    const handleKeyDown = (e: KeyboardEvent) => {
      // Escape cancels gizmo drag
      if (e.key === 'Escape' && gizmoDragRef.current) {
        const drag = gizmoDragRef.current;
        gizmoDragRef.current = null;
        for (const [key, origThree] of drag.originalThree) {
          const obj = actorObjMap.current.get(key);
          if (obj) obj.position.copy(origThree);
        }
        for (const [key, origEuler] of drag.originalEulers) {
          const obj = actorObjMap.current.get(key);
          if (obj) obj.setRotationFromEuler(origEuler);
        }
        for (const [key, origScale] of drag.originalScaleThree) {
          const obj = actorObjMap.current.get(key);
          if (obj) obj.scale.copy(origScale);
        }
        if (gizmoRef.current) gizmoRef.current.position.copy(drag.pivotStart);
        setDragInfoRef.current(null);
        needsRender.current = true;
        return;
      }

      const k = e.key.toLowerCase();
      if ('wasdeq'.includes(k) && k.length === 1) {
        keysDown.current.add(k);
        flyActive.current = true;
        e.stopPropagation(); // Prevent EditorFrame W/E/R transform mode switch
        return;
      }
      // Camera bookmarks: Ctrl+1-9 save, 1-9 restore
      if (e.key >= '0' && e.key <= '9') {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          const o = orbitRef.current;
          cameraBookmarks.current.set(e.key, {
            theta: o.theta, phi: o.phi, distance: o.distance,
            target: o.target.clone(),
          });
        } else if (!e.shiftKey && !e.altKey) {
          const bm = cameraBookmarks.current.get(e.key);
          if (bm) {
            const o = orbitRef.current;
            o.theta = bm.theta;
            o.phi = bm.phi;
            o.distance = bm.distance;
            o.target.copy(bm.target);
            updateCameraRef.current();
          }
        }
      }
    };
    const handleKeyUp = (e: KeyboardEvent) => {
      const k = e.key.toLowerCase();
      keysDown.current.delete(k);
      if (keysDown.current.size === 0) flyActive.current = false;
    };
    container.addEventListener('keydown', handleKeyDown);
    container.addEventListener('keyup', handleKeyUp);

    // Make container focusable for keyboard events
    container.tabIndex = -1;
    container.style.outline = 'none';

    return () => {
      cancelAnimationFrame(frameIdRef.current);
      obs.disconnect();
      container.removeEventListener('keydown', handleKeyDown);
      container.removeEventListener('keyup', handleKeyUp);
      renderer.dispose();
      container.removeChild(renderer.domElement);
      rendererRef.current = null;
      sceneRef.current = null;
      cameraRef.current = null;
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Auto-fit camera to zone bounds ----
  useEffect(() => {
    if (initialFitDone.current || !bounds || actors.length === 0) return;
    const cx = (bounds.min_x + bounds.max_x) / 2;
    const cy = (bounds.min_z + bounds.max_z) / 2;
    const cz = (bounds.min_y + bounds.max_y) / 2;
    const maxRange = Math.max(
      bounds.max_x - bounds.min_x,
      bounds.max_y - bounds.min_y,
      bounds.max_z - bounds.min_z,
      1000,
    );
    orbitRef.current.target.set(cx, cy, cz);
    orbitRef.current.distance = maxRange * 0.8;
    updateCamera();
    initialFitDone.current = true;
  }, [bounds, actors, updateCamera]);

  // ---- Focus on selected actor when F is pressed ----
  useEffect(() => {
    if (focusRequest === 0) return;
    if (selectedKeys.size === 0) return;
    const lastKey = [...selectedKeys][selectedKeys.size - 1];
    const actor = actorByKey.get(lastKey);
    if (actor) focusOnActor(actor);
  }, [focusRequest]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Grid visibility ----
  useEffect(() => {
    if (gridRef.current) {
      gridRef.current.visible = showGrid;
      needsRender.current = true;
    }
  }, [showGrid]);

  // ---- Transform gizmo ----
  useEffect(() => {
    const scene = sceneRef.current;
    if (!scene) return;

    // Remove old gizmo
    if (gizmoRef.current) {
      scene.remove(gizmoRef.current);
      gizmoRef.current = null;
    }

    // Show gizmo for zone actors OR for a selected DB entity (spawnlist only)
    let px = 0, py = 0, pz = 0, count = 0;
    let isDbEntityGizmo = false;

    if (selectedKeys.size > 0) {
      // Zone actor gizmo
      for (const key of selectedKeys) {
        const actor = actorByKey.get(key);
        if (actor) {
          px += actor.x; py += actor.z; pz += actor.y; // UE3->Three.js swap
          count++;
        }
      }
    } else if (selectedDbEntityIds.size === 1) {
      // DB entity gizmo (only for spawnlist entities — they're movable)
      const dbId = [...selectedDbEntityIds][0];
      const dbEnt = dbEntities.find(e => e.id === dbId);
      if (dbEnt && dbEnt.source_table === 'spawnlist') {
        px = dbEnt.x; py = dbEnt.z; pz = -dbEnt.y;
        count = 1;
        isDbEntityGizmo = true;
      }
    }

    if (count === 0) {
      needsRender.current = true;
      return;
    }
    px /= count; py /= count; pz /= count;

    const gizmo = createGizmo(isDbEntityGizmo ? 'move' : transformMode);
    gizmo.position.set(px, py, pz);
    gizmo.userData = { isDbEntityGizmo };
    gizmoModeRef.current = isDbEntityGizmo ? 'move' : transformMode;

    // Scale to camera distance
    const camera = cameraRef.current;
    if (camera) updateGizmoScale(gizmo, camera);

    scene.add(gizmo);
    gizmoRef.current = gizmo;
    needsRender.current = true;
  }, [selectedKeys, actors, transformMode, selectedDbEntityIds, dbEntities]);

  // ---- Place all actors ----
  useEffect(() => {
    const group = meshGroupRef.current;
    if (!group) return;

    while (group.children.length > 0) {
      disposeMaterials(group.children[0]);
      group.remove(group.children[0]);
    }
    actorObjMap.current.clear();

    const mode = renderModeRef.current;
    const meshActors: ActorListEntry[] = [];
    const indicatorActors: ActorListEntry[] = [];
    for (const a of actors) {
      if (a.static_mesh && a.static_mesh.length > 0) meshActors.push(a);
      else indicatorActors.push(a);
    }

    // Indicator actors (class-specific visuals)
    for (const actor of indicatorActors) {
      const visual = getClassGeometry(actor.class_name);
      const mat = getIndicatorMaterial(actor.class_name, selectedKeys.has(actor.key), visual.wireframe);
      const mesh = new THREE.Mesh(visual.geometry, mat);
      placeObject(mesh, actor);
      mesh.userData = { actorKey: actor.key };
      group.add(mesh);
      actorObjMap.current.set(actor.key, mesh);
    }

    // Mesh actors
    const placeAllMeshActors = () => {
      const toRemove: THREE.Object3D[] = [];
      for (const child of group.children) {
        if (child.userData?.actorKey && meshActors.some(a => a.key === child.userData.actorKey)) {
          toRemove.push(child);
        }
      }
      for (const c of toRemove) {
        disposeMaterials(c);
        group.remove(c);
        actorObjMap.current.delete(c.userData.actorKey);
      }

      for (const actor of meshActors) {
        const cached = geometryCache.get(actor.static_mesh);
        const geo = cached ?? new THREE.BoxGeometry(50, 50, 50);
        const isSelected = selectedKeys.has(actor.key);
        const mat = cached ? getMeshMaterial(mode, isSelected) : getIndicatorMaterial(actor.class_name, isSelected, false);
        const mesh = new THREE.Mesh(geo, mat);
        placeObject(mesh, actor);
        mesh.userData = { actorKey: actor.key };
        group.add(mesh);
        actorObjMap.current.set(actor.key, mesh);
      }
      needsRender.current = true;
    };

    const uniqueNames = new Set(meshActors.map(a => a.static_mesh));
    const promises: Promise<void>[] = [];
    for (const name of uniqueNames) {
      if (geometryCache.has(name) || loadingMeshes.has(name)) continue;
      loadingMeshes.add(name);
      promises.push(
        invoke<MeshData>('load_mesh', { meshName: name })
          .then(data => { geometryCache.set(name, createGeometryFromMeshData(data)); })
          .catch(err => { console.warn(`Mesh '${name}':`, err); })
          .finally(() => { loadingMeshes.delete(name); }),
      );
    }

    placeAllMeshActors();
    if (promises.length > 0) {
      Promise.all(promises).then(placeAllMeshActors);
    }
  }, [actors, renderMode]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- DB entity visualization (spawns, regions, stargates) ----
  useEffect(() => {
    const group = dbEntityGroupRef.current;
    if (!group) return;

    // Clear old
    while (group.children.length > 0) {
      disposeMaterials(group.children[0]);
      group.remove(group.children[0]);
    }

    const regionGeo = new THREE.CylinderGeometry(1, 1, 1, 16);
    const spawnGeo = new THREE.OctahedronGeometry(30);
    const gateGeo = new THREE.TorusGeometry(80, 12, 8, 24);

    for (const ent of dbEntities) {
      const isSelected = selectedDbEntityIds.has(ent.id);
      const colorHex = DB_ENTITY_COLORS[ent.entity_type] ?? DB_ENTITY_COLORS._default ?? '#888888';
      const color = new THREE.Color(colorHex);

      if (ent.source_table === 'generic_regions' && ent.radius && ent.height) {
        // Render as translucent cylinder
        const mat = new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: isSelected ? 0.35 : 0.15,
          depthWrite: false,
          side: THREE.DoubleSide,
        });
        const mesh = new THREE.Mesh(regionGeo, mat);
        mesh.scale.set(ent.radius, ent.height, ent.radius);
        mesh.position.set(ent.x, ent.z + ent.height / 2, -ent.y);
        mesh.userData = { dbEntityId: ent.id, dbSourceTable: ent.source_table };
        group.add(mesh);

        // Wireframe outline
        const wireMat = new THREE.MeshBasicMaterial({
          color,
          wireframe: true,
          transparent: true,
          opacity: isSelected ? 0.6 : 0.3,
        });
        const wire = new THREE.Mesh(regionGeo, wireMat);
        wire.scale.copy(mesh.scale);
        wire.position.copy(mesh.position);
        wire.userData = { dbEntityId: ent.id, dbSourceTable: ent.source_table };
        group.add(wire);
      } else if (ent.source_table === 'stargates') {
        // Render as torus (gate ring)
        const mat = new THREE.MeshBasicMaterial({
          color: isSelected ? 0x00ffff : 0x2299ff,
          transparent: true,
          opacity: 0.6,
        });
        const mesh = new THREE.Mesh(gateGeo, mat);
        mesh.position.set(ent.x, ent.z, -ent.y);
        mesh.rotation.x = Math.PI / 2;
        mesh.userData = { dbEntityId: ent.id, dbSourceTable: ent.source_table };
        group.add(mesh);
      } else {
        // Render spawns/spawn_points as diamond markers
        const mat = new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: isSelected ? 0.9 : 0.5,
        });
        const mesh = new THREE.Mesh(spawnGeo, mat);
        mesh.position.set(ent.x, ent.z, -ent.y);
        mesh.userData = { dbEntityId: ent.id, dbSourceTable: ent.source_table };
        group.add(mesh);
      }
    }

    needsRender.current = true;
  }, [dbEntities, selectedDbEntityIds]);

  // ---- NavMesh visualization ----
  useEffect(() => {
    const group = navMeshGroupRef.current;
    if (!group) return;

    // Clear old
    while (group.children.length > 0) {
      disposeMaterials(group.children[0]);
      group.remove(group.children[0]);
    }

    if (!navMeshData || navMeshData.vertices.length === 0) {
      needsRender.current = true;
      return;
    }

    // Area ID → color mapping
    const areaColors: Record<number, number> = {
      0: 0xff2222, // blocked = red
      1: 0x22cc44, // ground = green
      2: 0x2266ff, // custom = blue
      3: 0xcccc22, // yellow
    };

    // Build a buffer geometry from the triangle vertices
    const positions = new Float32Array(navMeshData.vertices);
    const colors = new Float32Array(navMeshData.vertices.length);

    // Assign colors per-vertex (3 verts per tri, each tri has an area_id)
    for (let t = 0; t < navMeshData.area_ids.length; t++) {
      const areaId = navMeshData.area_ids[t];
      const colorHex = areaColors[areaId] ?? 0x888888;
      const r = ((colorHex >> 16) & 0xff) / 255;
      const g = ((colorHex >> 8) & 0xff) / 255;
      const b = (colorHex & 0xff) / 255;
      for (let v = 0; v < 3; v++) {
        const ci = (t * 3 + v) * 3;
        colors[ci] = r;
        colors[ci + 1] = g;
        colors[ci + 2] = b;
      }
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geo.setAttribute('color', new THREE.BufferAttribute(colors, 3));
    geo.computeVertexNormals();

    // Translucent fill
    const fillMat = new THREE.MeshBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity: 0.25,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    const fillMesh = new THREE.Mesh(geo, fillMat);
    group.add(fillMesh);

    // Wireframe overlay
    const wireMat = new THREE.MeshBasicMaterial({
      vertexColors: true,
      wireframe: true,
      transparent: true,
      opacity: 0.4,
    });
    const wireMesh = new THREE.Mesh(geo, wireMat);
    group.add(wireMesh);

    needsRender.current = true;
  }, [navMeshData]);

  // ---- Radius visualization and connection lines ----
  useEffect(() => {
    const radiusGroup = radiusGroupRef.current;
    const connectionGroup = connectionGroupRef.current;
    if (!radiusGroup || !connectionGroup) return;

    // Clear old
    while (radiusGroup.children.length > 0) {
      disposeMaterials(radiusGroup.children[0]);
      radiusGroup.remove(radiusGroup.children[0]);
    }
    while (connectionGroup.children.length > 0) {
      disposeMaterials(connectionGroup.children[0]);
      connectionGroup.remove(connectionGroup.children[0]);
    }

    const RADIUS_MAP: Record<string, { radius: number; color: number }> = {
      PointLight: { radius: 200, color: 0xffdd44 },
      PointLightMovable: { radius: 200, color: 0xffdd44 },
      SpotLight: { radius: 300, color: 0xffdd44 },
      DirectionalLight: { radius: 150, color: 0xffdd44 },
      SkyLight: { radius: 150, color: 0xffdd44 },
      AmbientSoundSGW: { radius: 500, color: 0x44aaff },
      AmbientSound: { radius: 500, color: 0x44aaff },
      SGWSpawnRegion: { radius: 800, color: 0xff6644 },
      SGWSpawnSet: { radius: 400, color: 0xff6644 },
      Trigger: { radius: 300, color: 0x44ff88 },
      TriggerVolume: { radius: 300, color: 0x44ff88 },
    };

    const sphereGeo = new THREE.SphereGeometry(1, 24, 16);

    for (const actor of actors) {
      const cfg = RADIUS_MAP[actor.class_name];
      if (!cfg) continue;
      const mat = new THREE.MeshBasicMaterial({
        color: cfg.color,
        transparent: true,
        opacity: 0.06,
        wireframe: false,
        side: THREE.BackSide,
        depthWrite: false,
      });
      const sphere = new THREE.Mesh(sphereGeo, mat);
      sphere.position.set(actor.x, actor.z, actor.y);
      sphere.scale.setScalar(cfg.radius);
      radiusGroup.add(sphere);

      // Wireframe ring
      const ringGeo = new THREE.RingGeometry(cfg.radius - 2, cfg.radius, 48);
      const ringMat = new THREE.MeshBasicMaterial({
        color: cfg.color,
        transparent: true,
        opacity: 0.2,
        side: THREE.DoubleSide,
        depthWrite: false,
      });
      const ring = new THREE.Mesh(ringGeo, ringMat);
      ring.position.set(actor.x, actor.z, actor.y);
      ring.rotation.x = -Math.PI / 2; // Lie flat on the ground plane
      radiusGroup.add(ring);
    }

    // Connection lines for PathNodes and Cover nodes
    const CONNECTION_CLASSES = new Set(['PathNode', 'SGWSpecCoverNode', 'SGWCoverSet', 'CoverLink']);
    const CONNECTION_MAX_DIST: Record<string, number> = {
      PathNode: 1500,
      SGWSpecCoverNode: 800,
      SGWCoverSet: 800,
      CoverLink: 800,
    };

    for (const cls of CONNECTION_CLASSES) {
      const classActors = actors.filter(a => a.class_name === cls);
      if (classActors.length < 2) continue;
      const maxDist = CONNECTION_MAX_DIST[cls] ?? 1000;
      const maxDistSq = maxDist * maxDist;
      const lineMat = new THREE.LineBasicMaterial({
        color: cls === 'PathNode' ? 0x4488ff : 0xff8844,
        transparent: true,
        opacity: 0.3,
        depthWrite: false,
      });

      for (let i = 0; i < classActors.length; i++) {
        for (let j = i + 1; j < classActors.length; j++) {
          const a = classActors[i], b = classActors[j];
          const dx = a.x - b.x, dy = a.y - b.y, dz = a.z - b.z;
          if (dx * dx + dy * dy + dz * dz <= maxDistSq) {
            const lineGeo = new THREE.BufferGeometry().setFromPoints([
              new THREE.Vector3(a.x, a.z, a.y),
              new THREE.Vector3(b.x, b.z, b.y),
            ]);
            connectionGroup.add(new THREE.Line(lineGeo, lineMat));
          }
        }
      }
    }

    needsRender.current = true;
  }, [actors]);

  // ---- Update selection highlight + wireframe outlines ----
  useEffect(() => {
    const mode = renderModeRef.current;
    const outlineGroup = outlineGroupRef.current;

    // Update materials
    for (const [key, obj] of actorObjMap.current) {
      if (!(obj instanceof THREE.Mesh)) continue;
      const actor = actorByKey.get(key);
      if (!actor) continue;
      disposeMaterials(obj);
      const hasMeshGeo = actor.static_mesh && geometryCache.has(actor.static_mesh);
      if (hasMeshGeo) {
        obj.material = getMeshMaterial(mode, selectedKeys.has(key));
      } else {
        const visual = getClassGeometry(actor.class_name);
        obj.material = getIndicatorMaterial(actor.class_name, selectedKeys.has(key), visual.wireframe);
      }
    }

    // Rebuild wireframe edge outlines for selected mesh actors
    if (outlineGroup) {
      while (outlineGroup.children.length > 0) {
        const c = outlineGroup.children[0];
        disposeMaterials(c);
        outlineGroup.remove(c);
      }

      const outlineMat = new THREE.LineBasicMaterial({
        color: 0xf59e0b,
        depthTest: true,
        transparent: true,
        opacity: 0.7,
      });

      for (const key of selectedKeys) {
        const obj = actorObjMap.current.get(key);
        if (!(obj instanceof THREE.Mesh)) continue;
        const actor = actorByKey.get(key);
        if (!actor?.static_mesh || !geometryCache.has(actor.static_mesh)) continue;
        const edges = new THREE.EdgesGeometry(obj.geometry, 30);
        const wireframe = new THREE.LineSegments(edges, outlineMat);
        wireframe.position.copy(obj.position);
        wireframe.rotation.copy(obj.rotation);
        wireframe.scale.copy(obj.scale);
        outlineGroup.add(wireframe);
      }
    }

    needsRender.current = true;
  }, [selectedKeys, actors, renderMode]);

  // ---- Mouse: orbit / pan / pick / gizmo drag ----
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    // Focus container for keyboard events
    containerRef.current?.focus();

    const o = orbitRef.current;
    if (e.button === 2) {
      o.isDragging = true;
      o.lastX = e.clientX;
      o.lastY = e.clientY;
      rmbStartRef.current = { x: e.clientX, y: e.clientY };
    } else if (e.button === 1) {
      e.preventDefault();
      o.isPanning = true;
      o.lastX = e.clientX;
      o.lastY = e.clientY;
    } else if (e.button === 0) {
      const container = containerRef.current;
      const camera = cameraRef.current;
      const group = meshGroupRef.current;
      const gizmo = gizmoRef.current;
      if (!container || !camera || !group) return;

      const rect = container.getBoundingClientRect();
      const ndc = new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1,
      );

      // Check gizmo hit first (zone actors OR DB entity selected)
      const hasGizmoTarget = selectedKeysRef.current.size > 0 ||
        (gizmo?.userData?.isDbEntityGizmo && selectedDbEntityIds.size === 1);
      if (gizmo && hasGizmoTarget) {
        const axis = pickGizmoAxis(gizmo, ndc, camera);
        if (axis) {
          const mode = gizmoModeRef.current;
          const axisDir = AXIS_DIRS[axis].clone();
          const pivotStart = gizmo.position.clone();
          const originalUE3 = new Map<string, { x: number; y: number; z: number }>();
          const originalThree = new Map<string, THREE.Vector3>();
          const originalRotUE3 = new Map<string, { pitch: number; yaw: number; roll: number }>();
          const originalEulers = new Map<string, THREE.Euler>();
          const originalScaleUE3 = new Map<string, { ds: number; dsx: number; dsy: number; dsz: number }>();
          const originalScaleThree = new Map<string, THREE.Vector3>();

          for (const key of selectedKeysRef.current) {
            const actor = actorByKeyRef.current.get(key);
            const obj = actorObjMap.current.get(key);
            if (actor) {
              originalUE3.set(key, { x: actor.x, y: actor.y, z: actor.z });
              originalRotUE3.set(key, { pitch: actor.pitch, yaw: actor.yaw, roll: actor.roll });
              originalScaleUE3.set(key, { ds: actor.draw_scale, dsx: actor.draw_scale_x, dsy: actor.draw_scale_y, dsz: actor.draw_scale_z });
            }
            if (obj) {
              originalThree.set(key, obj.position.clone());
              originalEulers.set(key, obj.rotation.clone());
              originalScaleThree.set(key, obj.scale.clone());
            }
          }

          let startParam: number | null = null;
          if (mode === 'rotate') {
            startParam = projectMouseToAngle(e.clientX, e.clientY, rect, camera, axisDir, pivotStart);
          } else {
            startParam = projectMouseOnAxis(e.clientX, e.clientY, rect, camera, axisDir, pivotStart);
          }

          if (startParam !== null) {
            gizmoDragRef.current = {
              mode, axis, axisDir, pivotStart, startParam,
              originalUE3, originalThree,
              originalRotUE3, originalEulers,
              originalScaleUE3, originalScaleThree,
            };
            return; // Don't fall through to actor picking
          }
        }
      }

      // Normal actor picking
      const raycaster = new THREE.Raycaster();
      raycaster.setFromCamera(ndc, camera);
      const hits = raycaster.intersectObjects(group.children, false);

      if (hits.length > 0 && hits[0].object.userData?.actorKey) {
        const key = hits[0].object.userData.actorKey;
        if (e.ctrlKey || e.metaKey) {
          onToggleSelect(key);
        } else if (e.shiftKey) {
          onAddSelect(key);
        } else {
          onSelect(key);
        }
      } else {
        // Check if a DB entity was clicked
        const dbGroup = dbEntityGroupRef.current;
        if (dbGroup && dbGroup.children.length > 0) {
          const dbHits = raycaster.intersectObjects(dbGroup.children, false);
          if (dbHits.length > 0 && dbHits[0].object.userData?.dbEntityId != null) {
            const dbId = dbHits[0].object.userData.dbEntityId as number;
            if (onSelectDbEntity) onSelectDbEntity(dbId);
            return; // Don't fall through to placement or deselect
          }
        }

        // Empty space click
        if (placementModeRef.current && onPlaceSpawnRef.current) {
          // In placement mode: raycast to ground plane (Y=0) to get position
          const groundPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
          const intersection = new THREE.Vector3();
          if (raycaster.ray.intersectPlane(groundPlane, intersection)) {
            // Convert Three.js coords (X-right, Y-up, Z-forward) back to UE3
            // In our viewport: Three.js X = UE3 X, Three.js Y = UE3 Z, Three.js Z = -UE3 Y
            // But the viewport already uses UE3 coordinates directly
            onPlaceSpawnRef.current(intersection.x, intersection.z, intersection.y);
          }
        } else {
          // Normal mode: start tracking for potential box select
          boxSelectRef.current = { startX: e.clientX, startY: e.clientY, active: false };
        }
      }
    }
  }, [onSelect, onToggleSelect, onAddSelect]);

  // Double-click to focus on actor
  const handleDoubleClick = useCallback((e: React.MouseEvent) => {
    const container = containerRef.current;
    const camera = cameraRef.current;
    const group = meshGroupRef.current;
    if (!container || !camera || !group) return;

    const rect = container.getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1,
    );
    const raycaster = new THREE.Raycaster();
    raycaster.setFromCamera(ndc, camera);
    const hits = raycaster.intersectObjects(group.children, false);

    if (hits.length > 0) {
      const key = hits[0].object.userData?.actorKey;
      const actor = key ? actorByKeyRef.current.get(key) ?? null : null;
      if (actor) {
        onSelect(key);
        focusOnActor(actor);
      }
    }
  }, [onSelect, focusOnActor]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const o = orbitRef.current;
    const drag = gizmoDragRef.current;

    // ---- Gizmo drag in progress ----
    if (drag) {
      const container = containerRef.current;
      const camera = cameraRef.current;
      const gizmo = gizmoRef.current;
      if (!container || !camera) return;

      const rect = container.getBoundingClientRect();
      const snap = gridSnapRef.current;

      if (drag.mode === 'move') {
        const currentParam = projectMouseOnAxis(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        if (currentParam === null) return;
        let delta = currentParam - drag.startParam;
        if (snap > 0) delta = Math.round(delta / snap) * snap;
        const offset = drag.axisDir.clone().multiplyScalar(delta);
        for (const [key, origThree] of drag.originalThree) {
          const obj = actorObjMap.current.get(key);
          if (obj) obj.position.copy(origThree).add(offset);
        }
        if (gizmo) gizmo.position.copy(drag.pivotStart).add(offset);
        setDragInfo(`${drag.axis.toUpperCase()}: ${delta > 0 ? '+' : ''}${delta.toFixed(1)}`);
      } else if (drag.mode === 'rotate') {
        const currentAngle = projectMouseToAngle(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        if (currentAngle === null) return;
        let deltaRad = currentAngle - drag.startParam;
        // Snap rotation in degrees (reuse grid snap value as degree increment)
        if (snap > 0) {
          const snapRad = snap * Math.PI / 180;
          deltaRad = Math.round(deltaRad / snapRad) * snapRad;
        }
        for (const [key, origEuler] of drag.originalEulers) {
          const obj = actorObjMap.current.get(key);
          if (obj) {
            const r = origEuler.clone();
            if (drag.axis === 'x') r.x += deltaRad;
            else if (drag.axis === 'y') r.y += deltaRad;
            else r.z += deltaRad;
            obj.setRotationFromEuler(r);
          }
        }
        const deltaDeg = (deltaRad * 180) / Math.PI;
        setDragInfo(`${drag.axis.toUpperCase()}: ${deltaDeg > 0 ? '+' : ''}${deltaDeg.toFixed(1)}\u00B0`);
      } else if (drag.mode === 'scale') {
        const currentParam = projectMouseOnAxis(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        if (currentParam === null) return;
        let factor = Math.abs(drag.startParam) > 0.01
          ? currentParam / drag.startParam
          : 1;
        factor = Math.max(0.01, Math.min(100, factor));
        for (const [key, origScale] of drag.originalScaleThree) {
          const obj = actorObjMap.current.get(key);
          if (obj) {
            const s = origScale.clone();
            if (drag.axis === 'x') s.x *= factor;
            else if (drag.axis === 'y') s.y *= factor;
            else s.z *= factor;
            obj.scale.copy(s);
          }
        }
        setDragInfo(`${drag.axis.toUpperCase()}: \u00D7${factor.toFixed(3)}`);
      }

      needsRender.current = true;
      return;
    }

    // ---- Box select drag ----
    const bs = boxSelectRef.current;
    if (bs) {
      const bsDx = e.clientX - bs.startX;
      const bsDy = e.clientY - bs.startY;
      if (!bs.active && Math.sqrt(bsDx * bsDx + bsDy * bsDy) > 4) {
        bs.active = true;
      }
      if (bs.active) {
        const container = containerRef.current;
        if (container) {
          const cRect = container.getBoundingClientRect();
          setBoxSelectRect({
            left: Math.min(bs.startX, e.clientX) - cRect.left,
            top: Math.min(bs.startY, e.clientY) - cRect.top,
            width: Math.abs(bsDx),
            height: Math.abs(bsDy),
          });
        }
        return;
      }
    }

    // ---- Normal mouse movement ----
    const dx = e.clientX - o.lastX;
    const dy = e.clientY - o.lastY;
    o.lastX = e.clientX;
    o.lastY = e.clientY;

    if (o.isDragging) {
      o.theta -= dx * 0.005;
      o.phi = Math.max(0.05, Math.min(Math.PI - 0.05, o.phi - dy * 0.005));
      updateCamera();
    } else if (o.isPanning) {
      const camera = cameraRef.current;
      if (!camera) return;
      const panSpeed = o.distance * 0.001;
      const right = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld, 0);
      const up = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
      o.target.addScaledVector(right, -dx * panSpeed);
      o.target.addScaledVector(up, dy * panSpeed);
      updateCamera();
    } else {
      // Gizmo hover highlight
      const gizmo = gizmoRef.current;
      const container = containerRef.current;
      const camera = cameraRef.current;
      if (gizmo && container && camera) {
        const rect = container.getBoundingClientRect();
        const ndc = new THREE.Vector2(
          ((e.clientX - rect.left) / rect.width) * 2 - 1,
          -((e.clientY - rect.top) / rect.height) * 2 + 1,
        );
        const axis = pickGizmoAxis(gizmo, ndc, camera);
        if (axis !== hoveredAxisRef.current) {
          hoveredAxisRef.current = axis;
          highlightGizmoAxis(gizmo, axis);
          needsRender.current = true;
        }
      }
    }
  }, [updateCamera]);

  const handleMouseUp = useCallback((e: React.MouseEvent) => {
    if (e.button === 2) {
      orbitRef.current.isDragging = false;
      // If RMB was clicked without significant drag, show context menu
      const rmbStart = rmbStartRef.current;
      rmbStartRef.current = null;
      if (rmbStart && onContextMenuProp) {
        const dx = e.clientX - rmbStart.x;
        const dy = e.clientY - rmbStart.y;
        if (Math.sqrt(dx * dx + dy * dy) < 4) {
          onContextMenuProp(e.clientX, e.clientY);
        }
      }
    }
    if (e.button === 1) orbitRef.current.isPanning = false;

    // Finalize gizmo drag
    if (e.button === 0 && gizmoDragRef.current) {
      const drag = gizmoDragRef.current;
      gizmoDragRef.current = null;
      setDragInfo(null);

      const container = containerRef.current;
      const camera = cameraRef.current;
      if (!container || !camera) return;

      const rect = container.getBoundingClientRect();
      const snap = gridSnapRef.current;

      if (drag.mode === 'move') {
        const currentParam = projectMouseOnAxis(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        let delta = currentParam !== null ? currentParam - drag.startParam : 0;
        if (snap > 0) delta = Math.round(delta / snap) * snap;

        if (Math.abs(delta) < 0.01) {
          for (const [key, origThree] of drag.originalThree) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.position.copy(origThree);
          }
          if (gizmoRef.current) gizmoRef.current.position.copy(drag.pivotStart);
          needsRender.current = true;
          return;
        }

        const keys: string[] = [];
        const newX: number[] = [];
        const newY: number[] = [];
        const newZ: number[] = [];
        for (const [key, orig] of drag.originalUE3) {
          keys.push(key);
          let dx = 0, dy = 0, dz = 0;
          if (drag.axis === 'x') dx = delta;
          else if (drag.axis === 'y') dz = delta;
          else if (drag.axis === 'z') dy = delta;
          newX.push(orig.x + dx);
          newY.push(orig.y + dy);
          newZ.push(orig.z + dz);
        }
        // Check if this was a DB entity gizmo drag
        if (gizmoRef.current?.userData?.isDbEntityGizmo && onUpdateSpawnPosition && selectedDbEntityIds.size === 1) {
          const dbId = [...selectedDbEntityIds][0];
          const dbEnt = dbEntities.find(e => e.id === dbId);
          if (dbEnt) {
            // Compute new UE3 position from the Three.js gizmo final position
            const gPos = gizmoRef.current.position;
            // Three.js: X=ue3_x, Y=ue3_z, Z=-ue3_y
            const newUe3X = gPos.x;
            const newUe3Y = -gPos.z;
            const newUe3Z = gPos.y;
            onUpdateSpawnPosition(dbId, newUe3X, newUe3Y, newUe3Z, dbEnt.heading);
          }
        } else {
          onMoveActors(keys, newX, newY, newZ);
        }
      } else if (drag.mode === 'rotate') {
        const currentAngle = projectMouseToAngle(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        let deltaRad = currentAngle !== null ? currentAngle - drag.startParam : 0;
        if (snap > 0) {
          const snapRad = snap * Math.PI / 180;
          deltaRad = Math.round(deltaRad / snapRad) * snapRad;
        }

        if (Math.abs(deltaRad) < 0.001) {
          for (const [key, origEuler] of drag.originalEulers) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.setRotationFromEuler(origEuler);
          }
          needsRender.current = true;
          return;
        }

        const deltaUE3 = deltaRad / ROT_TO_RAD;
        const keys: string[] = [];
        const newPitch: number[] = [];
        const newYaw: number[] = [];
        const newRoll: number[] = [];
        for (const [key, orig] of drag.originalRotUE3) {
          keys.push(key);
          // Gizmo axis → UE3: x→pitch, y→yaw, z→roll
          newPitch.push(drag.axis === 'x' ? orig.pitch + deltaUE3 : orig.pitch);
          newYaw.push(drag.axis === 'y' ? orig.yaw + deltaUE3 : orig.yaw);
          newRoll.push(drag.axis === 'z' ? orig.roll + deltaUE3 : orig.roll);
        }
        onRotateActors(keys, newPitch, newYaw, newRoll);
      } else if (drag.mode === 'scale') {
        const currentParam = projectMouseOnAxis(e.clientX, e.clientY, rect, camera, drag.axisDir, drag.pivotStart);
        let factor = Math.abs(drag.startParam) > 0.01 && currentParam !== null
          ? currentParam / drag.startParam
          : 1;
        factor = Math.max(0.01, Math.min(100, factor));

        if (Math.abs(factor - 1) < 0.001) {
          for (const [key, origScale] of drag.originalScaleThree) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.scale.copy(origScale);
          }
          needsRender.current = true;
          return;
        }

        const keys: string[] = [];
        const newDS: number[] = [];
        const newDSX: number[] = [];
        const newDSY: number[] = [];
        const newDSZ: number[] = [];
        for (const [key, orig] of drag.originalScaleUE3) {
          keys.push(key);
          newDS.push(orig.ds);
          // Three.js X→UE3 dsx, Three.js Y→UE3 dsz, Three.js Z→UE3 dsy
          newDSX.push(drag.axis === 'x' ? orig.dsx * factor : orig.dsx);
          newDSY.push(drag.axis === 'z' ? orig.dsy * factor : orig.dsy);
          newDSZ.push(drag.axis === 'y' ? orig.dsz * factor : orig.dsz);
        }
        onScaleActors(keys, newDS, newDSX, newDSY, newDSZ);
      }
    }

    // Finalize box select
    if (e.button === 0 && boxSelectRef.current) {
      const bs = boxSelectRef.current;
      boxSelectRef.current = null;
      setBoxSelectRect(null);

      if (bs.active) {
        // Project all actors to screen coords, select those inside the rect
        const camera = cameraRef.current;
        const container = containerRef.current;
        if (camera && container) {
          const cRect = container.getBoundingClientRect();
          const minSX = Math.min(bs.startX, e.clientX) - cRect.left;
          const minSY = Math.min(bs.startY, e.clientY) - cRect.top;
          const maxSX = Math.max(bs.startX, e.clientX) - cRect.left;
          const maxSY = Math.max(bs.startY, e.clientY) - cRect.top;

          const selected: string[] = [];
          const proj = new THREE.Vector3();
          for (const actor of actorsRef.current) {
            proj.set(actor.x, actor.z, actor.y); // UE3 -> Three.js
            proj.project(camera);
            const sx = (proj.x * 0.5 + 0.5) * cRect.width;
            const sy = (-proj.y * 0.5 + 0.5) * cRect.height;
            // Only include actors in front of the camera (z < 1 in NDC)
            if (proj.z < 1 && sx >= minSX && sx <= maxSX && sy >= minSY && sy <= maxSY) {
              selected.push(actor.key);
            }
          }
          onBoxSelect(selected);
        }
      } else {
        // Just a click in empty space — deselect
        if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
          onSelect(null);
        }
      }
    }
  }, [onMoveActors, onRotateActors, onScaleActors, onBoxSelect, onSelect]);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const o = orbitRef.current;
    o.distance = Math.max(10, Math.min(200000, o.distance * (e.deltaY > 0 ? 1.1 : 0.9)));
    updateCamera();
  }, [updateCamera]);

  return (
    <div
      ref={containerRef}
      className="absolute inset-0"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onDoubleClick={handleDoubleClick}
      onWheel={handleWheel}
      onMouseLeave={() => {
        orbitRef.current.isDragging = false;
        orbitRef.current.isPanning = false;
        keysDown.current.clear();
        flyActive.current = false;
        // Cancel any in-progress box select
        boxSelectRef.current = null;
        setBoxSelectRect(null);
        // Cancel any in-progress gizmo drag and restore positions
        const drag = gizmoDragRef.current;
        if (drag) {
          gizmoDragRef.current = null;
          for (const [key, origThree] of drag.originalThree) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.position.copy(origThree);
          }
          for (const [key, origEuler] of drag.originalEulers) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.setRotationFromEuler(origEuler);
          }
          for (const [key, origScale] of drag.originalScaleThree) {
            const obj = actorObjMap.current.get(key);
            if (obj) obj.scale.copy(origScale);
          }
          const gizmo = gizmoRef.current;
          if (gizmo) gizmo.position.copy(drag.pivotStart);
          setDragInfo(null);
          needsRender.current = true;
        }
      }}
      onContextMenu={e => e.preventDefault()}
      style={{ cursor: placementMode ? 'cell' : gizmoDragRef.current ? 'grabbing' : hoveredAxisRef.current ? 'grab' : 'crosshair' }}
    >
      {boxSelectRect && (
        <div
          className="pointer-events-none absolute z-10 border border-dashed border-blue-400/80 bg-blue-400/10"
          style={boxSelectRect}
        />
      )}
      {/* Viewport HUD — transform mode + grid snap */}
      <div className="pointer-events-none absolute left-2 top-2 z-10 flex flex-col gap-0.5 text-[10px] font-mono text-white/40">
        <span>{transformMode.charAt(0).toUpperCase() + transformMode.slice(1)} | Grid: {gridSnap}</span>
        {placementMode && selectedTemplateName && (
          <span className="text-emerald-400">Placing: {selectedTemplateName}</span>
        )}
        {navMeshData && (
          <span className="text-cyan-400">NavMesh: {navMeshData.tri_count.toLocaleString()} tris</span>
        )}
      </div>
      {/* Drag delta display */}
      {dragInfo && (
        <div className="pointer-events-none absolute bottom-8 left-1/2 z-20 -translate-x-1/2 rounded bg-black/70 px-3 py-1 text-sm font-mono font-medium text-amber-400">
          {dragInfo}
        </div>
      )}
      {/* Selected actor info overlay */}
      {selectedKeys.size === 1 && (() => {
        const actor = actorByKey.get([...selectedKeys][0]);
        return actor ? (
          <div className="pointer-events-none absolute right-2 top-2 z-10 rounded bg-black/50 px-2 py-1 text-[10px] font-mono text-white/70">
            <div className="text-amber-400">{actor.class_name}</div>
            <div>{actor.object_name}</div>
            <div>Pos: {actor.x.toFixed(0)}, {actor.y.toFixed(0)}, {actor.z.toFixed(0)}</div>
          </div>
        ) : null;
      })()}
      {selectedKeys.size > 1 && (
        <div className="pointer-events-none absolute right-2 top-2 z-10 rounded bg-black/50 px-2 py-1 text-[10px] font-mono text-amber-400/70">
          {selectedKeys.size} actors selected
        </div>
      )}
    </div>
  );
}
