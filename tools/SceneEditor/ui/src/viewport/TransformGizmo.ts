import * as THREE from 'three';

export type GizmoMode = 'move' | 'rotate' | 'scale';

const AXIS_COLORS = {
  x: 0xe74c3c,
  y: 0x2ecc71,
  z: 0x3498db,
};

const GIZMO_LENGTH = 120;
const ARROW_HEAD_LENGTH = 30;
const ARROW_HEAD_WIDTH = 8;
const SCALE_CUBE_SIZE = 10;
const TORUS_RADIUS = 80;
const TORUS_TUBE = 2;

function makeAxisMaterial(axis: 'x' | 'y' | 'z', hovered: boolean): THREE.MeshBasicMaterial {
  return new THREE.MeshBasicMaterial({
    color: hovered ? 0xffff00 : AXIS_COLORS[axis],
    depthTest: false,
    depthWrite: false,
    transparent: true,
    opacity: hovered ? 1 : 0.85,
  });
}

function makeLineMaterial(axis: 'x' | 'y' | 'z', hovered: boolean): THREE.LineBasicMaterial {
  return new THREE.LineBasicMaterial({
    color: hovered ? 0xffff00 : AXIS_COLORS[axis],
    depthTest: false,
    depthWrite: false,
    linewidth: 2,
  });
}

/**
 * Create arrow geometry for the move gizmo.
 * Each axis gets a shaft (Line) + cone (Mesh).
 */
function createMoveGizmo(): THREE.Group {
  const group = new THREE.Group();

  const axes: Array<{ axis: 'x' | 'y' | 'z'; dir: THREE.Vector3 }> = [
    { axis: 'x', dir: new THREE.Vector3(1, 0, 0) },
    { axis: 'y', dir: new THREE.Vector3(0, 1, 0) },
    { axis: 'z', dir: new THREE.Vector3(0, 0, 1) },
  ];

  for (const { axis, dir } of axes) {
    // Shaft line
    const lineGeo = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(0, 0, 0),
      dir.clone().multiplyScalar(GIZMO_LENGTH - ARROW_HEAD_LENGTH),
    ]);
    const line = new THREE.Line(lineGeo, makeLineMaterial(axis, false));
    line.userData = { gizmoAxis: axis };
    line.renderOrder = 999;
    group.add(line);

    // Arrow cone
    const coneGeo = new THREE.ConeGeometry(ARROW_HEAD_WIDTH, ARROW_HEAD_LENGTH, 8);
    const cone = new THREE.Mesh(coneGeo, makeAxisMaterial(axis, false));
    cone.userData = { gizmoAxis: axis };
    cone.renderOrder = 999;

    // Position at end of shaft, pointing along axis
    cone.position.copy(dir.clone().multiplyScalar(GIZMO_LENGTH - ARROW_HEAD_LENGTH / 2));
    // Rotate cone to point along axis
    if (axis === 'x') cone.rotation.z = -Math.PI / 2;
    else if (axis === 'y') { /* default Y-up */ }
    else if (axis === 'z') cone.rotation.x = Math.PI / 2;

    group.add(cone);
  }

  return group;
}

/**
 * Create rotation rings for the rotate gizmo.
 */
function createRotateGizmo(): THREE.Group {
  const group = new THREE.Group();
  const torusGeo = new THREE.TorusGeometry(TORUS_RADIUS, TORUS_TUBE, 8, 48);

  // X-axis ring (rotates around X, lies in YZ plane)
  const xRing = new THREE.Mesh(torusGeo, makeAxisMaterial('x', false));
  xRing.rotation.y = Math.PI / 2;
  xRing.userData = { gizmoAxis: 'x' };
  xRing.renderOrder = 999;
  group.add(xRing);

  // Y-axis ring (rotates around Y, lies in XZ plane)
  const yRing = new THREE.Mesh(torusGeo, makeAxisMaterial('y', false));
  yRing.rotation.x = Math.PI / 2;
  yRing.userData = { gizmoAxis: 'y' };
  yRing.renderOrder = 999;
  group.add(yRing);

  // Z-axis ring (rotates around Z, lies in XY plane)
  const zRing = new THREE.Mesh(torusGeo, makeAxisMaterial('z', false));
  zRing.userData = { gizmoAxis: 'z' };
  zRing.renderOrder = 999;
  group.add(zRing);

  return group;
}

/**
 * Create scale handles — cubes at the end of axis lines.
 */
function createScaleGizmo(): THREE.Group {
  const group = new THREE.Group();
  const cubeGeo = new THREE.BoxGeometry(SCALE_CUBE_SIZE, SCALE_CUBE_SIZE, SCALE_CUBE_SIZE);

  const axes: Array<{ axis: 'x' | 'y' | 'z'; dir: THREE.Vector3 }> = [
    { axis: 'x', dir: new THREE.Vector3(1, 0, 0) },
    { axis: 'y', dir: new THREE.Vector3(0, 1, 0) },
    { axis: 'z', dir: new THREE.Vector3(0, 0, 1) },
  ];

  for (const { axis, dir } of axes) {
    // Shaft line
    const lineGeo = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(0, 0, 0),
      dir.clone().multiplyScalar(GIZMO_LENGTH),
    ]);
    const line = new THREE.Line(lineGeo, makeLineMaterial(axis, false));
    line.userData = { gizmoAxis: axis };
    line.renderOrder = 999;
    group.add(line);

    // End cube
    const cube = new THREE.Mesh(cubeGeo, makeAxisMaterial(axis, false));
    cube.position.copy(dir.clone().multiplyScalar(GIZMO_LENGTH));
    cube.userData = { gizmoAxis: axis };
    cube.renderOrder = 999;
    group.add(cube);
  }

  return group;
}

export function createGizmo(mode: GizmoMode): THREE.Group {
  switch (mode) {
    case 'move': return createMoveGizmo();
    case 'rotate': return createRotateGizmo();
    case 'scale': return createScaleGizmo();
  }
}

/**
 * Update the gizmo scale so it appears a constant screen size regardless of distance.
 */
export function updateGizmoScale(gizmo: THREE.Group, camera: THREE.PerspectiveCamera): void {
  const distance = camera.position.distanceTo(gizmo.position);
  const fov = camera.fov * (Math.PI / 180);
  const screenFraction = 0.15; // gizmo takes up ~15% of viewport height
  const scale = distance * Math.tan(fov / 2) * screenFraction / GIZMO_LENGTH;
  gizmo.scale.setScalar(Math.max(scale, 0.01));
}

/**
 * Highlight a specific axis on the gizmo (for hover feedback).
 */
export function highlightGizmoAxis(gizmo: THREE.Group, axis: 'x' | 'y' | 'z' | null): void {
  gizmo.traverse(child => {
    const childAxis = child.userData?.gizmoAxis as string | undefined;
    if (!childAxis) return;
    const isHovered = childAxis === axis;
    if (child instanceof THREE.Mesh) {
      child.material = makeAxisMaterial(childAxis as 'x' | 'y' | 'z', isHovered);
    } else if (child instanceof THREE.Line) {
      child.material = makeLineMaterial(childAxis as 'x' | 'y' | 'z', isHovered);
    }
  });
}

/**
 * Raycast against gizmo objects to find which axis the mouse is hovering over.
 */
export function pickGizmoAxis(
  gizmo: THREE.Group,
  ndc: THREE.Vector2,
  camera: THREE.PerspectiveCamera,
): 'x' | 'y' | 'z' | null {
  const raycaster = new THREE.Raycaster();
  raycaster.setFromCamera(ndc, camera);
  // Increase line precision for easier picking
  raycaster.params.Line = { threshold: 10 };

  const meshes: THREE.Object3D[] = [];
  gizmo.traverse(child => {
    if (child instanceof THREE.Mesh || child instanceof THREE.Line) {
      meshes.push(child);
    }
  });

  const hits = raycaster.intersectObjects(meshes, false);
  if (hits.length > 0 && hits[0].object.userData?.gizmoAxis) {
    return hits[0].object.userData.gizmoAxis as 'x' | 'y' | 'z';
  }
  return null;
}
