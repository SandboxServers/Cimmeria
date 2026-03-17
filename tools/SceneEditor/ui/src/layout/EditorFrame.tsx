import { useEffect, useState, useCallback, useRef, useMemo } from 'react';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import type {
  ZoneSummary,
  ActorEntry,
  ActorListEntry,
  DbStatus,
  DbEntity,
  EntityTemplate,
} from '../lib/types';
import { MenuBar } from '../menu/MenuBar';
import { MainToolbar } from '../toolbar/MainToolbar';
import { Outliner } from '../panels/Outliner';
import { ViewportContainer } from '../viewport/ViewportContainer';
import { PropertyWindow } from '../panels/PropertyWindow';
import { StatusBar } from '../statusbar/StatusBar';
import { ActorSearch } from '../panels/ActorSearch';
import { EntityPalette } from '../panels/EntityPalette';
import { DbEntityList } from '../panels/DbEntityList';
import { DbEntityDetail } from '../panels/DbEntityDetail';
import { ContentBrowser } from '../panels/ContentBrowser';

interface EditorFrameProps {
  loadedZone: ZoneSummary | null;
  actors: ActorListEntry[];
  allActors: ActorListEntry[];
  selectedKeys: Set<string>;
  primarySelectedKey: string | null;
  selectedActorDetail: ActorEntry | null;
  classFilter: Set<string>;
  mouseWorldPos: { x: number; y: number; z: number };
  meshProgress: { loaded: number; total: number } | null;
  focusRequest: number;
  onSelectActor: (key: string | null) => void;
  onToggleSelect: (key: string) => void;
  onAddSelect: (key: string) => void;
  onBoxSelect: (keys: string[]) => void;
  onSelectAll: () => void;
  onFocusSelected: () => void;
  onToggleClass: (className: string) => void;
  onToggleAllClasses: (enable: boolean) => void;
  onMouseWorldPosChange: (pos: { x: number; y: number; z: number }) => void;
  onOpenZoneSelector: () => void;
  onDeleteActors: () => void;
  onDuplicateActors: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onMoveActors: (keys: string[], newX: number[], newY: number[], newZ: number[]) => void;
  onRotateActors: (keys: string[], newPitch: number[], newYaw: number[], newRoll: number[]) => void;
  onScaleActors: (keys: string[], newDS: number[], newDSX: number[], newDSY: number[], newDSZ: number[]) => void;
  onMoveOneActor: (key: string, x: number, y: number, z: number) => void;
  onRotateOneActor: (key: string, pitch: number, yaw: number, roll: number) => void;
  onScaleOneActor: (key: string, drawScale: number, dsX: number, dsY: number, dsZ: number) => void;
  onPasteActors: (keys: string[]) => void;
  onSave: () => void;
  onExportSql: () => void;
  // DB props
  dbStatus: DbStatus | null;
  dbEntities: DbEntity[];
  entityTemplates: EntityTemplate[];
  selectedDbEntityIds: Set<number>;
  selectedTemplate: EntityTemplate | null;
  onOpenDbDialog: () => void;
  onDbConnect: (host: string, port: number, dbname: string, username: string, password: string) => void;
  onDbDisconnect: () => void;
  onLoadDbEntities: () => void;
  onRefreshDbEntities: () => void;
  onSelectDbEntity: (id: number) => void;
  onDeleteDbEntity: (id: number) => void;
  onSelectTemplate: (template: EntityTemplate | null) => void;
  placementMode: boolean;
  onTogglePlacement: () => void;
  onPlaceSpawn: (x: number, y: number, z: number) => void;
  onUpdateSpawnPosition: (spawnId: number, x: number, y: number, z: number, heading: number) => void;
  onCreateRegion: () => void;
  onPropertyChange: (key: string, propName: string, newValue: string) => void;
  onProceduralPlacement: () => void;
  navMeshData: import('../lib/types').NavMeshData | null;
  onExportDbSql: () => void;
  // Content Browser
  showContentBrowser: boolean;
  onToggleContentBrowser: () => void;
  cookedPcPath: string | null;
}

export function EditorFrame({
  loadedZone,
  actors,
  allActors,
  selectedKeys,
  primarySelectedKey,
  selectedActorDetail,
  classFilter,
  mouseWorldPos,
  meshProgress,
  focusRequest,
  onSelectActor,
  onToggleSelect,
  onAddSelect,
  onBoxSelect,
  onSelectAll,
  onFocusSelected,
  onToggleClass,
  onToggleAllClasses,
  onMouseWorldPosChange,
  onOpenZoneSelector,
  onDeleteActors,
  onDuplicateActors,
  onUndo,
  onRedo,
  onMoveActors,
  onRotateActors,
  onScaleActors,
  onMoveOneActor,
  onRotateOneActor,
  onScaleOneActor,
  onPasteActors,
  onSave,
  onExportSql,
  // DB props
  dbStatus,
  dbEntities,
  entityTemplates,
  selectedDbEntityIds,
  selectedTemplate,
  onOpenDbDialog,
  onDbConnect,
  onDbDisconnect,
  onLoadDbEntities,
  onRefreshDbEntities,
  onSelectDbEntity,
  onDeleteDbEntity,
  onSelectTemplate,
  placementMode,
  onTogglePlacement,
  onPlaceSpawn,
  onUpdateSpawnPosition,
  onCreateRegion,
  onPropertyChange,
  onProceduralPlacement,
  navMeshData,
  onExportDbSql,
  showContentBrowser,
  onToggleContentBrowser,
  cookedPcPath,
}: EditorFrameProps) {
  const [showSearch, setShowSearch] = useState(false);
  const clipboardRef = useRef<string[]>([]);

  // O(1) actor lookup map — avoids O(n) .find() on 812K actors
  const actorByKey = useMemo(() => {
    const m = new Map<string, ActorListEntry>();
    for (const a of allActors) m.set(a.key, a);
    return m;
  }, [allActors]);

  // Select all actors of the same class(es) as the current selection
  const handleSelectSameClass = useCallback(() => {
    if (selectedKeys.size === 0) return;
    const selectedClasses = new Set<string>();
    for (const key of selectedKeys) {
      const actor = actorByKey.get(key);
      if (actor) selectedClasses.add(actor.class_name);
    }
    const matching = allActors
      .filter(a => selectedClasses.has(a.class_name) && classFilter.has(a.class_name))
      .map(a => a.key);
    onBoxSelect(matching);
  }, [selectedKeys, allActors, actorByKey, classFilter, onBoxSelect]);

  // Invert selection (select all visible actors NOT currently selected)
  const handleInvertSelection = useCallback(() => {
    const visibleKeys = allActors
      .filter(a => classFilter.has(a.class_name))
      .map(a => a.key);
    const inverted = visibleKeys.filter(k => !selectedKeys.has(k));
    onBoxSelect(inverted);
  }, [allActors, classFilter, selectedKeys, onBoxSelect]);
  const [transformMode, setTransformMode] = useState<'move' | 'rotate' | 'scale'>('move');
  const [coordSystem, setCoordSystem] = useState<'world' | 'local'>('world');

  // Camera bookmarks (Ctrl+1-9 to save, 1-9 to recall)
  const bookmarksRef = useRef<Record<string, { x: number; y: number; z: number }>>(
    JSON.parse(localStorage.getItem('editor_bookmarks') ?? '{}')
  );

  // Actor locking (locked actors can't be moved/deleted)
  const [lockedKeys, setLockedKeys] = useState<Set<string>>(new Set());

  // Actor groups
  const [actorGroups, setActorGroups] = useState<Map<string, Set<string>>>(new Map());

  // Log messages
  const [logMessages, setLogMessages] = useState<string[]>([]);
  const [showLog, setShowLog] = useState(false);

  const addLog = useCallback((msg: string) => {
    const timestamp = new Date().toLocaleTimeString();
    setLogMessages(prev => [`[${timestamp}] ${msg}`, ...prev].slice(0, 200));
  }, []);

  // Measurement mode
  const [measureStart, setMeasureStart] = useState<{ x: number; y: number; z: number } | null>(null);
  const [measureEnd, setMeasureEnd] = useState<{ x: number; y: number; z: number } | null>(null);

  // Prefab system
  const prefabsRef = useRef<Map<string, string[]>>(
    new Map(Object.entries(JSON.parse(localStorage.getItem('editor_prefabs') ?? '{}')))
  );
  const [showGrid, setShowGrid] = useState(true);
  const [gridSnap, setGridSnap] = useState(10);

  // Align selected actors to nearest grid point
  const handleAlignToGrid = useCallback(() => {
    if (selectedKeys.size === 0) return;
    const keys: string[] = [];
    const newX: number[] = [];
    const newY: number[] = [];
    const newZ: number[] = [];
    const snap = gridSnap || 10;
    for (const key of selectedKeys) {
      const actor = actorByKey.get(key);
      if (actor) {
        keys.push(key);
        newX.push(Math.round(actor.x / snap) * snap);
        newY.push(Math.round(actor.y / snap) * snap);
        newZ.push(Math.round(actor.z / snap) * snap);
      }
    }
    if (keys.length > 0) onMoveActors(keys, newX, newY, newZ);
  }, [selectedKeys, actorByKey, gridSnap, onMoveActors]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (document.activeElement?.tagName === 'INPUT' || document.activeElement?.tagName === 'TEXTAREA') return;

      switch (e.key) {
        case 'f':
        case 'F':
          if ((e.ctrlKey || e.metaKey) && e.shiftKey) {
            e.preventDefault();
            onToggleContentBrowser();
          } else if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            setShowSearch(true);
          } else {
            e.preventDefault();
            onFocusSelected();
          }
          break;
        case 'Escape':
          if (placementMode) {
            onTogglePlacement();
          } else {
            onSelectActor(null);
          }
          break;
        case 'a':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            onSelectAll();
          }
          break;
        // Transform mode shortcuts (UnrealEd style)
        case 'w':
        case 'W':
          if (!e.ctrlKey && !e.metaKey) setTransformMode('move');
          break;
        case 'e':
        case 'E':
          if (!e.ctrlKey && !e.metaKey) setTransformMode('rotate');
          break;
        case 'r':
        case 'R':
          if ((e.ctrlKey || e.metaKey) && e.shiftKey) {
            e.preventDefault();
            onCreateRegion();
          } else if (!e.ctrlKey && !e.metaKey) {
            setTransformMode('scale');
          }
          break;
        case 'p':
        case 'P':
          if ((e.ctrlKey || e.metaKey) && e.shiftKey) {
            e.preventDefault();
            onProceduralPlacement();
          } else if (!e.ctrlKey && !e.metaKey && selectedTemplate) {
            onTogglePlacement();
          }
          break;
        case 'g':
        case 'G':
          setShowGrid(g => !g);
          break;
        case 'Delete':
          onDeleteActors();
          break;
        case 'c':
        case 'C':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            clipboardRef.current = [...selectedKeys];
          }
          break;
        case 'v':
        case 'V':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (clipboardRef.current.length > 0) onPasteActors(clipboardRef.current);
          }
          break;
        case 'd':
        case 'D':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            onDuplicateActors();
          }
          break;
        case 'z':
        case 'Z':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (e.shiftKey) {
              onRedo();
            } else {
              onUndo();
            }
          }
          break;
        case 'y':
        case 'Y':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            onRedo();
          }
          break;
        // Hide selected actor classes / show all
        case 'h':
          if (e.shiftKey) {
            // Shift+H: Show all classes
            onToggleAllClasses(true);
          } else if (selectedKeys.size > 0) {
            // H: Hide classes of selected actors
            const classesToHide = new Set<string>();
            for (const key of selectedKeys) {
              const actor = actorByKey.get(key);
              if (actor) classesToHide.add(actor.class_name);
            }
            for (const cls of classesToHide) {
              onToggleClass(cls);
            }
            onSelectActor(null);
          }
          break;
        // Invert selection
        case 'i':
        case 'I':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            handleInvertSelection();
          }
          break;
        // Save
        case 's':
        case 'S':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            onSave();
          }
          break;
        // Lock/unlock selected actors
        case 'l':
        case 'L':
          if (selectedKeys.size > 0) {
            setLockedKeys(prev => {
              const next = new Set(prev);
              const allLocked = [...selectedKeys].every(k => next.has(k));
              if (allLocked) {
                // Unlock all selected
                for (const k of selectedKeys) next.delete(k);
                addLog(`Unlocked ${selectedKeys.size} actor(s)`);
              } else {
                // Lock all selected
                for (const k of selectedKeys) next.add(k);
                addLog(`Locked ${selectedKeys.size} actor(s)`);
              }
              return next;
            });
          }
          break;
        // Group/Prefab (Ctrl+G = group, Ctrl+Shift+G = save prefab)
        case 'g':
          if ((e.ctrlKey || e.metaKey) && e.shiftKey) {
            e.preventDefault();
            if (selectedKeys.size > 0) {
              const name = prompt('Prefab name:', `Prefab_${prefabsRef.current.size + 1}`);
              if (name) {
                prefabsRef.current.set(name, [...selectedKeys]);
                localStorage.setItem('editor_prefabs', JSON.stringify(Object.fromEntries(prefabsRef.current)));
                addLog(`Saved prefab "${name}" with ${selectedKeys.size} actors`);
              }
            }
          } else if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            if (selectedKeys.size > 1) {
              const groupName = `Group_${actorGroups.size + 1}`;
              setActorGroups(prev => {
                const next = new Map(prev);
                next.set(groupName, new Set(selectedKeys));
                return next;
              });
              addLog(`Created group "${groupName}" with ${selectedKeys.size} actors`);
            }
          }
          break;
        // Measurement tool (M key)
        case 'm':
        case 'M':
          if (!e.ctrlKey && !e.metaKey) {
            if (!measureStart) {
              setMeasureStart(mouseWorldPos);
              addLog(`Measure start: (${mouseWorldPos.x.toFixed(0)}, ${mouseWorldPos.y.toFixed(0)}, ${mouseWorldPos.z.toFixed(0)})`);
            } else {
              setMeasureEnd(mouseWorldPos);
              const dx = mouseWorldPos.x - measureStart.x;
              const dy = mouseWorldPos.y - measureStart.y;
              const dz = mouseWorldPos.z - measureStart.z;
              const dist = Math.sqrt(dx*dx + dy*dy + dz*dz);
              addLog(`Measure: ${dist.toFixed(1)} cm (${(dist/100).toFixed(2)} m)`);
              setMeasureStart(null);
              setMeasureEnd(null);
            }
          }
          break;
        // Drop to ground (End key — matches UnrealEd)
        case 'End':
          if (selectedKeys.size > 0) {
            const keys: string[] = [];
            const newX: number[] = [];
            const newY: number[] = [];
            const newZ: number[] = [];
            for (const key of selectedKeys) {
              if (lockedKeys.has(key)) continue;
              const actor = actorByKey.get(key);
              if (actor) {
                keys.push(key);
                newX.push(actor.x);
                newY.push(actor.y);
                newZ.push(0); // Snap Z to ground plane (0)
              }
            }
            if (keys.length > 0) {
              onMoveActors(keys, newX, newY, newZ);
              addLog(`Dropped ${keys.length} actor(s) to ground`);
            }
          }
          break;
        // Toggle log window
        case '`':
          setShowLog(prev => !prev);
          break;
        // Camera bookmarks: Ctrl+1-9 to save, 1-9 to recall
        case '1': case '2': case '3': case '4': case '5':
        case '6': case '7': case '8': case '9':
          if (e.ctrlKey || e.metaKey) {
            e.preventDefault();
            // Save bookmark at current mouse position
            bookmarksRef.current[e.key] = { ...mouseWorldPos };
            localStorage.setItem('editor_bookmarks', JSON.stringify(bookmarksRef.current));
            addLog(`Bookmark ${e.key} saved at (${mouseWorldPos.x.toFixed(0)}, ${mouseWorldPos.y.toFixed(0)}, ${mouseWorldPos.z.toFixed(0)})`);
          } else if (!e.altKey) {
            // Recall bookmark — find nearest actor to bookmark position and focus
            const bm = bookmarksRef.current[e.key];
            if (bm) {
              // Find the actor closest to the bookmark position
              let nearest: string | null = null;
              let nearestDist = Infinity;
              for (const a of actors) {
                const dx = a.x - bm.x;
                const dy = a.y - bm.y;
                const dz = a.z - bm.z;
                const d = dx*dx + dy*dy + dz*dz;
                if (d < nearestDist) {
                  nearestDist = d;
                  nearest = a.key;
                }
              }
              if (nearest) {
                onSelectActor(nearest);
                onFocusSelected();
                addLog(`Recalled bookmark ${e.key}`);
              }
            }
          }
          break;
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onFocusSelected, onSelectActor, onSelectAll, onToggleClass, onToggleAllClasses, onDeleteActors, onDuplicateActors, onPasteActors, onUndo, onRedo, onSave, handleInvertSelection, selectedKeys, actorByKey, onToggleContentBrowser]);

  return (
    <div className="flex h-screen flex-col">
      <MenuBar
        loadedZone={loadedZone}
        classFilter={classFilter}
        hasSelection={selectedKeys.size > 0}
        onOpenZone={onOpenZoneSelector}
        onToggleClass={onToggleClass}
        onToggleAllClasses={onToggleAllClasses}
        onSelectAll={onSelectAll}
        onSelectNone={() => onSelectActor(null)}
        onSelectSameClass={handleSelectSameClass}
        onInvertSelection={handleInvertSelection}
        onSearch={() => setShowSearch(true)}
        onUndo={onUndo}
        onRedo={onRedo}
        onSave={onSave}
        onExportSql={onExportSql}
        onDeleteActors={onDeleteActors}
        onDuplicateActors={onDuplicateActors}
        onAlignToGrid={handleAlignToGrid}
        onCopy={() => { clipboardRef.current = [...selectedKeys]; }}
        onPaste={() => { if (clipboardRef.current.length > 0) onPasteActors(clipboardRef.current); }}
        hasClipboard={clipboardRef.current.length > 0}
        dbConnected={dbStatus?.connected ?? false}
        hasDbEntities={dbEntities.length > 0}
        onOpenDbDialog={onOpenDbDialog}
        onDbDisconnect={onDbDisconnect}
        onLoadDbEntities={onLoadDbEntities}
        onRefreshDbEntities={onRefreshDbEntities}
        onExportDbSql={onExportDbSql}
        onCreateRegion={onCreateRegion}
        onProceduralPlacement={onProceduralPlacement}
        onToggleContentBrowser={onToggleContentBrowser}
      />

      <MainToolbar
        transformMode={transformMode}
        coordSystem={coordSystem}
        showGrid={showGrid}
        onTransformChange={setTransformMode}
        onCoordSystemChange={setCoordSystem}
        gridSnap={gridSnap}
        selectionCount={selectedKeys.size}
        onToggleGrid={() => setShowGrid(g => !g)}
        onGridSnapChange={setGridSnap}
        onOpenZone={onOpenZoneSelector}
      />

      <div className="flex-1 overflow-hidden">
        <Allotment vertical>
          <Allotment.Pane>
            <Allotment>
              <Allotment.Pane preferredSize={240} minSize={180} maxSize={400}>
                <Allotment vertical>
                  <Allotment.Pane>
                    <Outliner
                      actors={allActors}
                      classFilter={classFilter}
                      selectedKeys={selectedKeys}
                      onSelect={onSelectActor}
                      onToggleSelect={onToggleSelect}
                      onAddSelect={onAddSelect}
                      onBoxSelect={onBoxSelect}
                      onFocusSelected={onFocusSelected}
                    />
                  </Allotment.Pane>
                  {dbEntities.length > 0 && (
                    <Allotment.Pane preferredSize={200} minSize={100}>
                      <DbEntityList
                        entities={dbEntities}
                        selectedIds={selectedDbEntityIds}
                        onSelect={onSelectDbEntity}
                        onDelete={onDeleteDbEntity}
                      />
                    </Allotment.Pane>
                  )}
                </Allotment>
              </Allotment.Pane>

              <Allotment.Pane>
                <ViewportContainer
                  actors={actors}
                  bounds={loadedZone?.bounds ?? null}
                  selectedKeys={selectedKeys}
                  showGrid={showGrid}
                  focusRequest={focusRequest}
                  transformMode={transformMode}
                  gridSnap={gridSnap}
                  onSelect={onSelectActor}
                  onToggleSelect={onToggleSelect}
                  onAddSelect={onAddSelect}
                  onBoxSelect={onBoxSelect}
                  onMouseMove={onMouseWorldPosChange}
                  onSelectAll={onSelectAll}
                  onFocusSelected={onFocusSelected}
                  onMoveActors={onMoveActors}
                  onRotateActors={onRotateActors}
                  onScaleActors={onScaleActors}
                  onDeleteActors={onDeleteActors}
                  onDuplicateActors={onDuplicateActors}
                  onSelectSameClass={handleSelectSameClass}
                  onInvertSelection={handleInvertSelection}
                  dbEntities={dbEntities}
                  selectedDbEntityIds={selectedDbEntityIds}
                  placementMode={placementMode}
                  onPlaceSpawn={onPlaceSpawn}
                  selectedTemplateName={selectedTemplate?.template_name ?? null}
                  onSelectDbEntity={onSelectDbEntity}
                  onUpdateSpawnPosition={onUpdateSpawnPosition}
                  navMeshData={navMeshData}
                />
              </Allotment.Pane>

              <Allotment.Pane preferredSize={320} minSize={240} maxSize={500}>
                <Allotment vertical>
                  <Allotment.Pane>
                    <PropertyWindow
                      actor={selectedActorDetail}
                      selectionCount={selectedKeys.size}
                      onMoveActor={onMoveOneActor}
                      onRotateActor={onRotateOneActor}
                      onScaleActor={onScaleOneActor}
                      onPropertyChange={onPropertyChange}
                      isLocked={primarySelectedKey ? lockedKeys.has(primarySelectedKey) : false}
                    />
                  </Allotment.Pane>
                  {selectedDbEntityIds.size === 1 && (
                    <Allotment.Pane preferredSize={180} minSize={100}>
                      <DbEntityDetail
                        entity={dbEntities.find(e => e.id === [...selectedDbEntityIds][0]) ?? null}
                        onUpdatePosition={onUpdateSpawnPosition}
                      />
                    </Allotment.Pane>
                  )}
                  <Allotment.Pane preferredSize={200}>
                    <EntityPalette
                      templates={entityTemplates}
                      selectedTemplate={selectedTemplate}
                      onSelectTemplate={onSelectTemplate}
                      connected={dbStatus?.connected ?? false}
                      placementMode={placementMode}
                      onTogglePlacement={onTogglePlacement}
                    />
                  </Allotment.Pane>
                </Allotment>
              </Allotment.Pane>
            </Allotment>
          </Allotment.Pane>
          {showContentBrowser && (
            <Allotment.Pane preferredSize={300} minSize={200}>
              <ContentBrowser
                visible={showContentBrowser}
                cookedPcPath={cookedPcPath}
              />
            </Allotment.Pane>
          )}
        </Allotment>
      </div>

      <StatusBar
        zone={loadedZone}
        mousePos={mouseWorldPos}
        selectedActor={selectedActorDetail}
        gridSnap={gridSnap}
        actorCount={actors.length}
        totalActorCount={allActors.length}
        selectionCount={selectedKeys.size}
        meshProgress={meshProgress}
        transformMode={transformMode}
        dbStatus={dbStatus}
        dbEntityCount={dbEntities.length}
        placementMode={placementMode}
        placementTemplateName={selectedTemplate?.template_name ?? null}
      />

      {showSearch && (
        <ActorSearch
          actors={allActors}
          onSelect={key => {
            onSelectActor(key);
            onFocusSelected();
          }}
          onClose={() => setShowSearch(false)}
        />
      )}

      {/* Log window (toggle with backtick key) */}
      {showLog && (
        <div className="absolute bottom-8 left-0 right-0 z-30 border-t bg-card/95 backdrop-blur">
          <div className="flex items-center justify-between border-b px-3 py-1">
            <span className="text-[10px] font-medium text-foreground">Output Log</span>
            <div className="flex items-center gap-2">
              <span className="text-[9px] text-muted-foreground">{logMessages.length} messages</span>
              <button
                onClick={() => setLogMessages([])}
                className="text-[10px] text-muted-foreground hover:text-foreground"
              >
                Clear
              </button>
              <button
                onClick={() => setShowLog(false)}
                className="text-[10px] text-muted-foreground hover:text-foreground"
              >
                Close
              </button>
            </div>
          </div>
          <div className="h-32 overflow-y-auto px-3 py-1 font-mono text-[10px] text-muted-foreground subtle-scrollbar">
            {logMessages.length === 0 ? (
              <div className="py-2 text-center text-muted-foreground/40">No messages</div>
            ) : (
              logMessages.map((msg, i) => (
                <div key={i} className="py-0.5 leading-tight">{msg}</div>
              ))
            )}
          </div>
        </div>
      )}

      {/* Measurement display */}
      {measureStart && !measureEnd && (
        <div className="pointer-events-none absolute bottom-10 left-1/2 z-40 -translate-x-1/2 rounded bg-black/70 px-3 py-1 text-xs font-mono text-cyan-400">
          Measuring... press M again to set end point (Esc to cancel)
        </div>
      )}

      {/* Lock indicator on status bar area */}
      {lockedKeys.size > 0 && (
        <div className="pointer-events-none absolute bottom-0 right-48 z-40 flex items-center gap-1 px-2 py-1.5">
          <span className="text-[10px] text-amber-500">🔒 {lockedKeys.size} locked</span>
        </div>
      )}
    </div>
  );
}
