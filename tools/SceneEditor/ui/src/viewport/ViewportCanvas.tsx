import { useRef, useEffect, useCallback, useState, useMemo } from 'react';
import type { ActorListEntry, ViewportType, ZoneBounds, DbEntity } from '../lib/types';
import { ACTOR_COLORS, DB_ENTITY_COLORS } from '../lib/types';
import { isLightClass, isVolumeClass } from './ActorVisuals';

interface ViewportCanvasProps {
  type: ViewportType;
  actors: ActorListEntry[];
  bounds: ZoneBounds | null;
  selectedKeys: Set<string>;
  showGrid: boolean;
  focusRequest?: number;
  onSelect: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onBoxSelect: (keys: string[]) => void;
  onMouseMove: (pos: { x: number; y: number; z: number }) => void;
  dbEntities?: DbEntity[];
  selectedDbEntityIds?: Set<number>;
}

/** Which world axes map to canvas horizontal/vertical for each viewport type. */
function getViewportAxes(type: ViewportType): {
  hAxis: 'x' | 'y' | 'z';
  vAxis: 'x' | 'y' | 'z';
  hLabel: string;
  vLabel: string;
  hFlip: boolean;
  vFlip: boolean;
} {
  switch (type) {
    case 'top':
      return { hAxis: 'x', vAxis: 'y', hLabel: 'X', vLabel: 'Y', hFlip: false, vFlip: true };
    case 'front':
      return { hAxis: 'x', vAxis: 'z', hLabel: 'X', vLabel: 'Z', hFlip: false, vFlip: true };
    case 'side':
      return { hAxis: 'y', vAxis: 'z', hLabel: 'Y', vLabel: 'Z', hFlip: false, vFlip: true };
    case 'perspective':
      // Phase 1: perspective renders as top-down
      return { hAxis: 'x', vAxis: 'y', hLabel: 'X', vLabel: 'Y', hFlip: false, vFlip: true };
  }
}

function worldToCanvas(
  wx: number,
  wy: number,
  zoom: number,
  panX: number,
  panY: number,
  canvasW: number,
  canvasH: number,
  hFlip: boolean,
  vFlip: boolean,
): [number, number] {
  const cx = canvasW / 2 + (wx + panX) * zoom * (hFlip ? -1 : 1);
  const cy = canvasH / 2 + (wy + panY) * zoom * (vFlip ? -1 : 1);
  return [cx, cy];
}

function canvasToWorld(
  cx: number,
  cy: number,
  zoom: number,
  panX: number,
  panY: number,
  canvasW: number,
  canvasH: number,
  hFlip: boolean,
  vFlip: boolean,
): [number, number] {
  const wx = (cx - canvasW / 2) / (zoom * (hFlip ? -1 : 1)) - panX;
  const wy = (cy - canvasH / 2) / (zoom * (vFlip ? -1 : 1)) - panY;
  return [wx, wy];
}

const HIT_RADIUS_PX = 8;
const DOT_RADIUS = 3;
const SELECTED_RING_RADIUS = 7;
const MIN_ZOOM = 0.00001;
const MAX_ZOOM = 10;

export function ViewportCanvas({
  type,
  actors,
  bounds,
  selectedKeys,
  showGrid,
  focusRequest = 0,
  onSelect,
  onToggleSelect,
  onAddSelect,
  onBoxSelect,
  onMouseMove,
  dbEntities = [],
  selectedDbEntityIds = new Set(),
}: ViewportCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 300, h: 200 });
  const [zoom, setZoom] = useState(0.01);

  // O(1) actor lookup map
  const actorByKey = useMemo(() => {
    const m = new Map<string, ActorListEntry>();
    for (const a of actors) m.set(a.key, a);
    return m;
  }, [actors]);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [isPanning, setIsPanning] = useState(false);
  const [boxStart, setBoxStart] = useState<{ x: number; y: number } | null>(null);
  const [boxEnd, setBoxEnd] = useState<{ x: number; y: number } | null>(null);
  const [hoverInfo, setHoverInfo] = useState<{ x: number; y: number; name: string; cls: string } | null>(null);
  const lastMouseRef = useRef({ x: 0, y: 0 });
  const initialFitDone = useRef(false);

  const axes = getViewportAxes(type);

  // ---- ResizeObserver ----
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const obs = new ResizeObserver(entries => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0 && height > 0) {
          setSize({ w: Math.floor(width), h: Math.floor(height) });
        }
      }
    });
    obs.observe(container);
    return () => obs.disconnect();
  }, []);

  // ---- Auto-fit on first load ----
  useEffect(() => {
    if (initialFitDone.current || !bounds || actors.length === 0) return;
    if (size.w < 10 || size.h < 10) return;

    const getAxisVal = (a: ActorListEntry, axis: 'x' | 'y' | 'z') => a[axis];
    const hVals = actors.map(a => getAxisVal(a, axes.hAxis));
    const vVals = actors.map(a => getAxisVal(a, axes.vAxis));

    const hMin = Math.min(...hVals);
    const hMax = Math.max(...hVals);
    const vMin = Math.min(...vVals);
    const vMax = Math.max(...vVals);

    const hRange = Math.max(hMax - hMin, 100);
    const vRange = Math.max(vMax - vMin, 100);

    const padding = 0.9;
    const fitZoom = Math.min(
      (size.w * padding) / hRange,
      (size.h * padding) / vRange,
    );

    setZoom(Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, fitZoom)));
    setPanX(-(hMin + hRange / 2));
    setPanY(-(vMin + vRange / 2));
    initialFitDone.current = true;
  }, [bounds, actors, size, axes]);

  // Reset auto-fit when type changes
  useEffect(() => {
    initialFitDone.current = false;
  }, [type]);

  // Focus on selected actor when F is pressed
  useEffect(() => {
    if (focusRequest === 0 || selectedKeys.size === 0) return;
    const lastKey = [...selectedKeys][selectedKeys.size - 1];
    const actor = actorByKey.get(lastKey);
    if (!actor) return;
    setPanX(-actor[axes.hAxis]);
    setPanY(-actor[axes.vAxis]);
  }, [focusRequest]); // eslint-disable-line react-hooks/exhaustive-deps

  // ---- Draw ----
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = size.w * dpr;
    canvas.height = size.h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    // Clear
    ctx.fillStyle = '#0f1419';
    ctx.fillRect(0, 0, size.w, size.h);

    // Grid
    if (showGrid) {
      drawGrid(ctx, size.w, size.h, zoom, panX, panY, axes.hFlip, axes.vFlip);
    }

    // Origin crosshair
    const [ox, oy] = worldToCanvas(0, 0, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
    ctx.strokeStyle = 'rgba(255, 50, 50, 0.4)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(ox, 0);
    ctx.lineTo(ox, size.h);
    ctx.stroke();
    ctx.strokeStyle = 'rgba(50, 255, 50, 0.4)';
    ctx.beginPath();
    ctx.moveTo(0, oy);
    ctx.lineTo(size.w, oy);
    ctx.stroke();

    // Connection lines (drawn before dots so dots render on top)
    const CONNECTION_GROUPS: Array<{ classes: Set<string>; maxDist: number; color: string }> = [
      { classes: new Set(['PathNode']), maxDist: 1500, color: 'rgba(59, 130, 246, 0.2)' },
      { classes: new Set(['CoverLink', 'SGWSpecCoverNode', 'SGWCoverSet']), maxDist: 800, color: 'rgba(255, 136, 68, 0.25)' },
      { classes: new Set(['SGWSpawnRegion', 'SGWSpawnSet']), maxDist: 2000, color: 'rgba(255, 102, 68, 0.15)' },
    ];

    for (const group of CONNECTION_GROUPS) {
      const groupActors = actors.filter(a => group.classes.has(a.class_name));
      if (groupActors.length < 2) continue;
      ctx.strokeStyle = group.color;
      ctx.lineWidth = 1;
      for (let i = 0; i < groupActors.length; i++) {
        const a = groupActors[i];
        for (let j = i + 1; j < groupActors.length; j++) {
          const b = groupActors[j];
          const dist = Math.hypot(a.x - b.x, a.y - b.y, a.z - b.z);
          if (dist > group.maxDist) continue;
          const [ax, ay] = worldToCanvas(a[axes.hAxis], a[axes.vAxis], zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
          const [bx, by] = worldToCanvas(b[axes.hAxis], b[axes.vAxis], zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
          if ((ax < -50 && bx < -50) || (ax > size.w + 50 && bx > size.w + 50)) continue;
          if ((ay < -50 && by < -50) || (ay > size.h + 50 && by > size.h + 50)) continue;
          ctx.beginPath();
          ctx.moveTo(ax, ay);
          ctx.lineTo(bx, by);
          ctx.stroke();
        }
      }
    }

    // Actors
    for (const actor of actors) {
      const h = actor[axes.hAxis];
      const v = actor[axes.vAxis];
      const [cx, cy] = worldToCanvas(h, v, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);

      // Skip off-screen
      if (cx < -10 || cx > size.w + 10 || cy < -10 || cy > size.h + 10) continue;

      const color = ACTOR_COLORS[actor.class_name] ?? ACTOR_COLORS._default;

      // Selected ring
      if (selectedKeys.has(actor.key)) {
        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(cx, cy, SELECTED_RING_RADIUS, 0, Math.PI * 2);
        ctx.stroke();
      }

      // Radius visualization for lights, sounds, spawns, triggers
      const RADIUS_CFG: Record<string, { radius: number; color: string }> = {
        PointLight: { radius: 200, color: 'rgba(255, 221, 68, 0.15)' },
        PointLightMovable: { radius: 200, color: 'rgba(255, 221, 68, 0.15)' },
        SpotLight: { radius: 300, color: 'rgba(255, 221, 68, 0.12)' },
        AmbientSoundSGW: { radius: 500, color: 'rgba(68, 170, 255, 0.12)' },
        AmbientSound: { radius: 500, color: 'rgba(68, 170, 255, 0.12)' },
        SGWSpawnRegion: { radius: 800, color: 'rgba(255, 102, 68, 0.08)' },
        SGWSpawnSet: { radius: 400, color: 'rgba(255, 102, 68, 0.10)' },
        Trigger: { radius: 300, color: 'rgba(68, 255, 136, 0.10)' },
        TriggerVolume: { radius: 300, color: 'rgba(68, 255, 136, 0.10)' },
      };
      const radiusCfg = RADIUS_CFG[actor.class_name];
      if (radiusCfg) {
        const rPx = Math.max(radiusCfg.radius * zoom, 6);
        ctx.fillStyle = radiusCfg.color;
        ctx.beginPath();
        ctx.arc(cx, cy, rPx, 0, Math.PI * 2);
        ctx.fill();
        ctx.strokeStyle = radiusCfg.color.replace(/[\d.]+\)$/, '0.4)');
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.arc(cx, cy, rPx, 0, Math.PI * 2);
        ctx.stroke();
      }

      // Class-specific shape
      if (isLightClass(actor.class_name)) {
        // Light: filled dot + wireframe circle for radius
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(cx, cy, DOT_RADIUS + 1, 0, Math.PI * 2);
        ctx.fill();
        const radiusPx = Math.max(200 * zoom, 6);
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.3;
        ctx.beginPath();
        ctx.arc(cx, cy, radiusPx, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalAlpha = 1;
      } else if (isVolumeClass(actor.class_name)) {
        // Volume: wireframe rectangle
        const halfW = Math.max(100 * zoom * (actor.draw_scale_x || 1) * (actor.draw_scale || 1), 4);
        const halfH = Math.max(100 * zoom * (actor.draw_scale_y || 1) * (actor.draw_scale || 1), 4);
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.5;
        ctx.strokeRect(cx - halfW, cy - halfH, halfW * 2, halfH * 2);
        ctx.globalAlpha = 1;
        // Small center dot
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(cx, cy, 2, 0, Math.PI * 2);
        ctx.fill();
      } else {
        // Default dot
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(cx, cy, DOT_RADIUS, 0, Math.PI * 2);
        ctx.fill();
      }
    }

    // ---- DB entity overlay markers ----
    const DB_DIAMOND_SIZE = 5;
    const DB_SELECTED_SIZE = 9;

    for (const dbEnt of dbEntities) {
      const h = dbEnt[axes.hAxis];
      const v = dbEnt[axes.vAxis];
      const [cx, cy] = worldToCanvas(h, v, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);

      // Skip off-screen
      if (cx < -20 || cx > size.w + 20 || cy < -20 || cy > size.h + 20) continue;

      const color = DB_ENTITY_COLORS[dbEnt.entity_type] ?? DB_ENTITY_COLORS._default;
      const isSelected = selectedDbEntityIds.has(dbEnt.id);

      // Radius circle for regions
      if (dbEnt.radius != null && dbEnt.radius > 0) {
        const rPx = Math.max(dbEnt.radius * 100 * zoom, 4);
        ctx.setLineDash([6, 4]);
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.35;
        ctx.beginPath();
        ctx.arc(cx, cy, rPx, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalAlpha = 0.08;
        ctx.fillStyle = color;
        ctx.fill();
        ctx.globalAlpha = 1;
        ctx.setLineDash([]);
      }

      // Stargate ring marker
      if (dbEnt.entity_type === 'Stargate') {
        const outerR = Math.max(12, 400 * zoom);
        const innerR = outerR * 0.65;
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.globalAlpha = 0.6;
        ctx.beginPath();
        ctx.arc(cx, cy, outerR, 0, Math.PI * 2);
        ctx.stroke();
        ctx.beginPath();
        ctx.arc(cx, cy, innerR, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalAlpha = 1;
      }

      // Selected glow
      if (isSelected) {
        ctx.strokeStyle = '#ffffff';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(cx, cy - DB_SELECTED_SIZE);
        ctx.lineTo(cx + DB_SELECTED_SIZE, cy);
        ctx.lineTo(cx, cy + DB_SELECTED_SIZE);
        ctx.lineTo(cx - DB_SELECTED_SIZE, cy);
        ctx.closePath();
        ctx.stroke();

        // Pulsing glow effect via shadow
        ctx.shadowColor = color;
        ctx.shadowBlur = 8;
      }

      // Diamond shape
      ctx.strokeStyle = color;
      ctx.lineWidth = isSelected ? 2 : 1.5;
      ctx.fillStyle = isSelected ? color : 'rgba(0,0,0,0.5)';
      ctx.beginPath();
      ctx.moveTo(cx, cy - DB_DIAMOND_SIZE);
      ctx.lineTo(cx + DB_DIAMOND_SIZE, cy);
      ctx.lineTo(cx, cy + DB_DIAMOND_SIZE);
      ctx.lineTo(cx - DB_DIAMOND_SIZE, cy);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();

      // Reset shadow
      if (isSelected) {
        ctx.shadowColor = 'transparent';
        ctx.shadowBlur = 0;
      }
    }

    // Axis labels in bottom-left corner
    ctx.font = '11px "IBM Plex Mono", monospace';
    ctx.fillStyle = 'rgba(255, 50, 50, 0.7)';
    ctx.fillText(axes.hLabel, 12, size.h - 8);
    ctx.fillStyle = 'rgba(50, 255, 50, 0.7)';
    ctx.fillText(axes.vLabel, 28, size.h - 8);

    // Box select rectangle
    if (boxStart && boxEnd) {
      const x = Math.min(boxStart.x, boxEnd.x);
      const y = Math.min(boxStart.y, boxEnd.y);
      const w2 = Math.abs(boxEnd.x - boxStart.x);
      const h2 = Math.abs(boxEnd.y - boxStart.y);
      ctx.strokeStyle = '#3b82f6';
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      ctx.strokeRect(x, y, w2, h2);
      ctx.setLineDash([]);
      ctx.fillStyle = 'rgba(59, 130, 246, 0.1)';
      ctx.fillRect(x, y, w2, h2);
    }

  }, [actors, selectedKeys, showGrid, zoom, panX, panY, size, axes, type, boxStart, boxEnd, dbEntities, selectedDbEntityIds]);

  // ---- Mouse: Scroll to zoom ----
  const handleWheel = useCallback((e: React.WheelEvent) => {
    e.preventDefault();
    const factor = e.deltaY > 0 ? 0.85 : 1.15;
    setZoom(z => Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, z * factor)));
  }, []);

  // ---- Mouse: Middle-drag to pan ----
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button === 1) {
      // Middle mouse
      e.preventDefault();
      setIsPanning(true);
      lastMouseRef.current = { x: e.clientX, y: e.clientY };
    } else if (e.button === 0) {
      // Left click: pick actor or start box select
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const cx = e.clientX - rect.left;
      const cy = e.clientY - rect.top;

      let bestKey: string | null = null;
      let bestDist = Infinity;

      for (const actor of actors) {
        const h = actor[axes.hAxis];
        const v = actor[axes.vAxis];
        const [ax, ay] = worldToCanvas(h, v, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
        const dist = Math.hypot(ax - cx, ay - cy);
        if (dist < HIT_RADIUS_PX && dist < bestDist) {
          bestDist = dist;
          bestKey = actor.key;
        }
      }

      if (bestKey) {
        if (e.ctrlKey || e.metaKey) {
          onToggleSelect(bestKey);
        } else if (e.shiftKey) {
          onAddSelect(bestKey);
        } else {
          onSelect(bestKey);
        }
      } else {
        // Start box select drag
        setBoxStart({ x: cx, y: cy });
        setBoxEnd({ x: cx, y: cy });
      }
    }
  }, [actors, axes, zoom, panX, panY, size, onSelect, onToggleSelect, onAddSelect]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (isPanning) {
      const dx = e.clientX - lastMouseRef.current.x;
      const dy = e.clientY - lastMouseRef.current.y;
      lastMouseRef.current = { x: e.clientX, y: e.clientY };

      setPanX(p => p + dx / zoom * (axes.hFlip ? -1 : 1));
      setPanY(p => p + dy / zoom * (axes.vFlip ? -1 : 1));
      return;
    }

    // Box select drag
    if (boxStart) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      setBoxEnd({ x: e.clientX - rect.left, y: e.clientY - rect.top });
      return;
    }

    // Update world position in status bar
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;
    const [wh, wv] = canvasToWorld(cx, cy, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);

    const pos = { x: 0, y: 0, z: 0 };
    pos[axes.hAxis] = wh;
    pos[axes.vAxis] = wv;
    onMouseMove(pos);

    // Hover tooltip
    let found = false;
    for (const actor of actors) {
      const h = actor[axes.hAxis];
      const v = actor[axes.vAxis];
      const [ax, ay] = worldToCanvas(h, v, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
      if (Math.hypot(ax - cx, ay - cy) < HIT_RADIUS_PX) {
        setHoverInfo({ x: cx, y: cy, name: actor.object_name, cls: actor.class_name });
        found = true;
        break;
      }
    }
    if (!found) setHoverInfo(null);
  }, [isPanning, boxStart, zoom, panX, panY, size, axes, actors, onMouseMove]);

  const handleMouseUp = useCallback((e: React.MouseEvent) => {
    if (e.button === 1) {
      setIsPanning(false);
    } else if (e.button === 0 && boxStart && boxEnd) {
      // Finish box select
      const bx1 = Math.min(boxStart.x, boxEnd.x);
      const by1 = Math.min(boxStart.y, boxEnd.y);
      const bx2 = Math.max(boxStart.x, boxEnd.x);
      const by2 = Math.max(boxStart.y, boxEnd.y);

      // Only count as box select if dragged more than a few pixels
      if (bx2 - bx1 > 4 || by2 - by1 > 4) {
        const keys: string[] = [];
        for (const actor of actors) {
          const h = actor[axes.hAxis];
          const v = actor[axes.vAxis];
          const [cx, cy] = worldToCanvas(h, v, zoom, panX, panY, size.w, size.h, axes.hFlip, axes.vFlip);
          if (cx >= bx1 && cx <= bx2 && cy >= by1 && cy <= by2) {
            keys.push(actor.key);
          }
        }
        onBoxSelect(keys);
      } else {
        // Tiny drag = deselect
        onSelect(null);
      }
      setBoxStart(null);
      setBoxEnd(null);
    }
  }, [boxStart, boxEnd, actors, axes, zoom, panX, panY, size, onBoxSelect, onSelect]);

  const handleMouseLeave = useCallback(() => {
    setIsPanning(false);
    setBoxStart(null);
    setBoxEnd(null);
  }, []);

  return (
    <div ref={containerRef} className="absolute inset-0">
      <canvas
        ref={canvasRef}
        style={{ width: size.w, height: size.h, cursor: isPanning ? 'grabbing' : boxStart ? 'crosshair' : 'crosshair' }}
        onWheel={handleWheel}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        onContextMenu={e => e.preventDefault()}
      />
      {hoverInfo && !boxStart && !isPanning && (
        <div
          className="pointer-events-none absolute z-10 rounded bg-card/95 px-2 py-1 text-[10px] shadow-lg border"
          style={{ left: hoverInfo.x + 12, top: hoverInfo.y - 8 }}
        >
          <div className="font-medium text-foreground">{hoverInfo.name}</div>
          <div className="text-muted-foreground">{hoverInfo.cls}</div>
        </div>
      )}
    </div>
  );
}

// ---- Grid drawing ----

function drawGrid(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  zoom: number,
  panX: number,
  panY: number,
  hFlip: boolean,
  vFlip: boolean,
) {
  // Adaptive grid spacing based on zoom
  const rawSpacing = 100 / zoom;
  const exp = Math.floor(Math.log10(rawSpacing));
  const base = Math.pow(10, exp);
  let spacing: number;
  if (rawSpacing / base < 2) spacing = base;
  else if (rawSpacing / base < 5) spacing = base * 2;
  else spacing = base * 5;

  // Minor grid (smaller subdivision)
  const minorSpacing = spacing / 5;

  // World bounds visible in canvas
  const [wLeft, wTop] = canvasToWorld(0, 0, zoom, panX, panY, w, h, hFlip, vFlip);
  const [wRight, wBottom] = canvasToWorld(w, h, zoom, panX, panY, w, h, hFlip, vFlip);
  const wMinH = Math.min(wLeft, wRight);
  const wMaxH = Math.max(wLeft, wRight);
  const wMinV = Math.min(wTop, wBottom);
  const wMaxV = Math.max(wTop, wBottom);

  // Draw minor grid
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.04)';
  ctx.lineWidth = 1;
  let startH = Math.floor(wMinH / minorSpacing) * minorSpacing;
  for (let wh = startH; wh <= wMaxH; wh += minorSpacing) {
    const [cx] = worldToCanvas(wh, 0, zoom, panX, panY, w, h, hFlip, vFlip);
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.stroke();
  }
  let startV = Math.floor(wMinV / minorSpacing) * minorSpacing;
  for (let wv = startV; wv <= wMaxV; wv += minorSpacing) {
    const [, cy] = worldToCanvas(0, wv, zoom, panX, panY, w, h, hFlip, vFlip);
    ctx.beginPath();
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
  }

  // Draw major grid
  ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
  ctx.lineWidth = 1;
  startH = Math.floor(wMinH / spacing) * spacing;
  for (let wh = startH; wh <= wMaxH; wh += spacing) {
    const [cx] = worldToCanvas(wh, 0, zoom, panX, panY, w, h, hFlip, vFlip);
    ctx.beginPath();
    ctx.moveTo(cx, 0);
    ctx.lineTo(cx, h);
    ctx.stroke();
  }
  startV = Math.floor(wMinV / spacing) * spacing;
  for (let wv = startV; wv <= wMaxV; wv += spacing) {
    const [, cy] = worldToCanvas(0, wv, zoom, panX, panY, w, h, hFlip, vFlip);
    ctx.beginPath();
    ctx.moveTo(0, cy);
    ctx.lineTo(w, cy);
    ctx.stroke();
  }
}
