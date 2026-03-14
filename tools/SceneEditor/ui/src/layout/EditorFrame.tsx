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
          if (!e.ctrlKey && !e.metaKey) setTransformMode('scale');
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
                    />
                  </Allotment.Pane>
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
    </div>
  );
}
