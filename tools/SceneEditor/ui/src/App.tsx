import { useState, useCallback, useMemo } from 'react';
import { invoke } from './lib/tauri';
import type {
  ZoneInfo,
  ZoneSummary,
  ActorEntry,
  ActorListEntry,
  MeshData,
  DbStatus,
  DbEntity,
  EntityTemplate,
  WorldInfo,
  NavMeshData,
  NavMeshInfo,
} from './lib/types';
import { EditorFrame } from './layout/EditorFrame';
import { ZoneSelector } from './panels/ZoneSelector';
import { DbConnectionDialog } from './panels/DbConnectionDialog';
import { CreateRegionDialog } from './panels/CreateRegionDialog';
import { ProceduralPlacementDialog } from './panels/ProceduralPlacementDialog';

export default function App() {
  // ---- Zone state ----
  const [zones, setZones] = useState<ZoneInfo[]>([]);
  const [loadedZone, setLoadedZone] = useState<ZoneSummary | null>(null);
  const [actors, setActors] = useState<ActorListEntry[]>([]);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [selectedActorDetail, setSelectedActorDetail] = useState<ActorEntry | null>(null);
  const [classFilter, setClassFilter] = useState<Set<string>>(new Set());
  const [mouseWorldPos, setMouseWorldPos] = useState({ x: 0, y: 0, z: 0 });

  // ---- Zone selector visibility ----
  const [showZoneSelector, setShowZoneSelector] = useState(true);
  const [loading, setLoading] = useState(false);
  const [loadingMessage, setLoadingMessage] = useState('');

  // ---- Content Browser ----
  const [showContentBrowser, setShowContentBrowser] = useState(false);
  const [cookedPcPath, setCookedPcPath] = useState<string | null>(null);

  // ---- Mesh preloading progress ----
  const [meshProgress, setMeshProgress] = useState<{ loaded: number; total: number } | null>(null);

  // ---- Focus request ----
  const [focusRequest, setFocusRequest] = useState(0);

  // ---- DB state ----
  const [dbStatus, setDbStatus] = useState<DbStatus | null>(null);
  const [dbEntities, setDbEntities] = useState<DbEntity[]>([]);
  const [entityTemplates, setEntityTemplates] = useState<EntityTemplate[]>([]);
  const [selectedDbEntityIds, setSelectedDbEntityIds] = useState<Set<number>>(new Set());
  const [selectedTemplate, setSelectedTemplate] = useState<EntityTemplate | null>(null);
  const [showDbDialog, setShowDbDialog] = useState(false);
  const [worlds, setWorlds] = useState<WorldInfo[]>([]);

  // ---- NavMesh state ----
  const [navMeshData, setNavMeshData] = useState<NavMeshData | null>(null);

  // ---- Placement mode ----
  const [placementMode, setPlacementMode] = useState(false);

  // Place a spawn at the clicked world position (called from viewport)
  const handlePlaceSpawn = useCallback(async (x: number, y: number, z: number) => {
    if (!selectedTemplate || !loadedZone || !dbStatus?.connected) return;

    // Find the world_id for the current zone
    const zoneName = loadedZone.zone_name;
    const matchedWorld = worlds.find(w =>
      w.client_map.toLowerCase() === zoneName.toLowerCase() ||
      w.world_name.toLowerCase() === zoneName.toLowerCase()
    );
    if (!matchedWorld) {
      console.warn(`No world found matching zone "${zoneName}"`);
      return;
    }

    try {
      const entity = await invoke<DbEntity>('create_spawn', {
        worldId: matchedWorld.world_id,
        templateId: selectedTemplate.template_id,
        x, y, z,
        heading: 0,
        tag: null,
      });
      // Add to local DB entity list
      setDbEntities(prev => [...prev, entity]);
      console.log(`Placed ${selectedTemplate.template_name} at (${x.toFixed(0)}, ${y.toFixed(0)}, ${z.toFixed(0)})`);
    } catch (e) {
      console.error('create_spawn failed:', e);
    }
  }, [selectedTemplate, loadedZone, dbStatus, worlds]);

  // Toggle placement mode (Escape to cancel)
  const handleTogglePlacement = useCallback(() => {
    setPlacementMode(prev => !prev);
  }, []);

  // Cancel placement when template is deselected
  const handleSelectTemplate = useCallback((template: EntityTemplate | null) => {
    setSelectedTemplate(template);
    if (!template) setPlacementMode(false);
  }, []);

  // ---- Region creation ----
  const [showRegionDialog, setShowRegionDialog] = useState(false);

  // ---- Procedural placement ----
  const [showProceduralDialog, setShowProceduralDialog] = useState(false);

  const handleProceduralPlaced = useCallback(async (_newActors: ActorListEntry[]) => {
    // Refresh actor list after procedural placement
    try {
      const actorList = await invoke<ActorListEntry[]>('list_actors', { classFilter: null });
      setActors(actorList);
    } catch (e) {
      console.error('list_actors failed:', e);
    }
  }, []);

  const handleCreateRegion = useCallback((entity: DbEntity) => {
    setDbEntities(prev => [...prev, entity]);
  }, []);

  const currentWorldId = useMemo(() => {
    if (!loadedZone || worlds.length === 0) return null;
    const matched = worlds.find(w =>
      w.client_map.toLowerCase() === loadedZone.zone_name.toLowerCase() ||
      w.world_name.toLowerCase() === loadedZone.zone_name.toLowerCase()
    );
    return matched?.world_id ?? null;
  }, [loadedZone, worlds]);

  // ---- Update spawn position (called when DB entity is moved in viewport) ----
  const handleUpdateSpawnPosition = useCallback(async (spawnId: number, x: number, y: number, z: number, heading: number) => {
    try {
      await invoke('update_spawn_position', { spawnId, x, y, z, heading });
      // Update local state to reflect the move
      setDbEntities(prev => prev.map(e =>
        e.id === spawnId && e.source_table === 'spawnlist'
          ? { ...e, x, y, z, heading }
          : e
      ));
      console.log(`Updated spawn ${spawnId} position`);
    } catch (e) {
      console.error('update_spawn_position failed:', e);
    }
  }, []);

  // ---- DB connection handler ----
  const handleDbConnect = useCallback(async (host: string, port: number, dbname: string, username: string, password: string) => {
    const status = await invoke<DbStatus>('connect_db', { host, port, dbname, username, password });
    setDbStatus(status);
    // Load worlds list
    const worldList = await invoke<WorldInfo[]>('list_worlds');
    setWorlds(worldList);
    // Load templates
    const templates = await invoke<EntityTemplate[]>('list_entity_templates', { classFilter: null });
    setEntityTemplates(templates);
  }, []);

  // ---- DB disconnect handler ----
  const handleDbDisconnect = useCallback(async () => {
    try {
      await invoke('disconnect_db');
    } catch { /* ignore */ }
    setDbStatus(null);
    setDbEntities([]);
    setEntityTemplates([]);
    setSelectedDbEntityIds(new Set());
    setSelectedTemplate(null);
    setWorlds([]);
  }, []);

  // ---- Load DB entities for current zone ----
  const handleLoadDbEntities = useCallback(async () => {
    if (!loadedZone || !dbStatus?.connected || worlds.length === 0) return;
    // Match zone_name to a world by comparing with client_map
    const zoneName = loadedZone.zone_name;
    const matchedWorld = worlds.find(w =>
      w.client_map.toLowerCase() === zoneName.toLowerCase() ||
      w.world_name.toLowerCase() === zoneName.toLowerCase()
    );
    if (!matchedWorld) {
      console.warn(`No world found matching zone "${zoneName}"`);
      return;
    }
    try {
      const entities = await invoke<DbEntity[]>('load_db_entities', { worldId: matchedWorld.world_id });
      setDbEntities(entities);
      setSelectedDbEntityIds(new Set());
    } catch (e) {
      console.error('load_db_entities failed:', e);
    }
  }, [loadedZone, dbStatus, worlds]);

  // ---- Refresh DB entities ----
  const handleRefreshDbEntities = useCallback(async () => {
    await handleLoadDbEntities();
  }, [handleLoadDbEntities]);

  // ---- Select DB entity ----
  const handleSelectDbEntity = useCallback((id: number) => {
    setSelectedDbEntityIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  // ---- Delete DB entity ----
  const handleDeleteDbEntity = useCallback(async (id: number) => {
    // Find the entity to get its source_table for table-aware deletion
    const entity = dbEntities.find(e => e.id === id);
    const sourceTable = entity?.source_table ?? 'spawnlist';
    try {
      await invoke('delete_db_entity', { entityId: id, sourceTable });
      setDbEntities(prev => prev.filter(e => e.id !== id));
      setSelectedDbEntityIds(prev => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } catch (e) {
      console.error('delete_db_entity failed:', e);
    }
  }, [dbEntities]);

  // ---- Export DB entities to SQL ----
  const handleExportDbSql = useCallback(async () => {
    const zoneName = loadedZone?.zone_name ?? 'zone';
    const filePath = prompt('Export DB entities SQL to:', `${zoneName}_db.sql`);
    if (!filePath) return;
    try {
      const count = await invoke<number>('export_db_entities_sql', { filePath });
      alert(`Exported ${count} DB entities to ${filePath}`);
    } catch (e) {
      console.error('export_db_entities_sql failed:', e);
      alert(`Export failed: ${e}`);
    }
  }, [loadedZone]);

  // ---- Scan for zones ----
  const handleScan = useCallback(async (scannedPath: string) => {
    setLoading(true);
    setLoadingMessage('Scanning for zones...');
    try {
      const result = await invoke<ZoneInfo[]>('scan_zones', { cookedPcPath: scannedPath });
      setZones(result);
      setCookedPcPath(scannedPath);
    } catch (e) {
      console.error('scan_zones failed:', e);
    } finally {
      setLoading(false);
      setLoadingMessage('');
    }
  }, []);

  // ---- Load a zone ----
  const handleLoadZone = useCallback(async (zonePath: string) => {
    setLoading(true);
    setLoadingMessage('Loading zone...');
    setSelectedKeys(new Set());
    setSelectedActorDetail(null);
    setMeshProgress(null);
    try {
      const summary = await invoke<ZoneSummary>('load_zone', { zonePath });
      setLoadedZone(summary);
      setClassFilter(new Set(Object.keys(summary.class_counts)));

      const actorList = await invoke<ActorListEntry[]>('list_actors', { classFilter: null });
      setActors(actorList);
      setShowZoneSelector(false);

      // Auto-load DB entities if connected
      if (dbStatus?.connected && worlds.length > 0) {
        const matchedWorld = worlds.find(w =>
          w.client_map.toLowerCase() === summary.zone_name.toLowerCase() ||
          w.world_name.toLowerCase() === summary.zone_name.toLowerCase()
        );
        if (matchedWorld) {
          try {
            const entities = await invoke<DbEntity[]>('load_db_entities', { worldId: matchedWorld.world_id });
            setDbEntities(entities);
            setSelectedDbEntityIds(new Set());
          } catch (e) {
            console.warn('Auto-load DB entities failed:', e);
          }
        }
      }

      // Try to load navmesh for this zone
      try {
        const navmeshes = await invoke<NavMeshInfo[]>('list_navmeshes');
        const zoneLower = summary.zone_name.toLowerCase();
        const match = navmeshes.find(n => zoneLower.includes(n.name.toLowerCase()));
        if (match) {
          const nmData = await invoke<NavMeshData>('load_navmesh', { navPath: match.path });
          setNavMeshData(nmData);
          console.log(`Loaded navmesh: ${match.name} (${nmData.tri_count} tris)`);
        } else {
          setNavMeshData(null);
        }
      } catch (e) {
        console.warn('NavMesh load failed:', e);
        setNavMeshData(null);
      }

      // Batch-preload unique meshes
      const uniqueMeshes = [...new Set(
        actorList.map(a => a.static_mesh).filter(m => m && m.length > 0),
      )];
      if (uniqueMeshes.length > 0) {
        setMeshProgress({ loaded: 0, total: uniqueMeshes.length });
        let loaded = 0;
        const batchSize = 8;
        for (let i = 0; i < uniqueMeshes.length; i += batchSize) {
          const batch = uniqueMeshes.slice(i, i + batchSize);
          await Promise.allSettled(
            batch.map(meshName => invoke<MeshData>('load_mesh', { meshName }).catch(() => {})),
          );
          loaded += batch.length;
          setMeshProgress({ loaded: Math.min(loaded, uniqueMeshes.length), total: uniqueMeshes.length });
        }
        setTimeout(() => setMeshProgress(null), 1500);
      }
    } catch (e) {
      console.error('load_zone failed:', e);
    } finally {
      setLoading(false);
      setLoadingMessage('');
    }
  }, []);

  // ---- Selection handlers ----
  // Single select (replaces selection)
  const handleSelectActor = useCallback(async (key: string | null) => {
    if (key) {
      setSelectedKeys(new Set([key]));
      try {
        const detail = await invoke<ActorEntry>('get_actor_details', { key });
        setSelectedActorDetail(detail);
      } catch (e) {
        console.error('get_actor_details failed:', e);
        setSelectedActorDetail(null);
      }
    } else {
      setSelectedKeys(new Set());
      setSelectedActorDetail(null);
    }
  }, []);

  // Toggle selection (Ctrl+click)
  const handleToggleSelect = useCallback(async (key: string) => {
    setSelectedKeys(prev => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
    // Show detail for the toggled actor
    try {
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch { /* ignore */ }
  }, []);

  // Additive selection (Shift+click)
  const handleAddSelect = useCallback(async (key: string) => {
    setSelectedKeys(prev => new Set([...prev, key]));
    try {
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch { /* ignore */ }
  }, []);

  // Box select (replace with set)
  const handleBoxSelect = useCallback((keys: string[]) => {
    setSelectedKeys(new Set(keys));
    setSelectedActorDetail(null);
  }, []);

  // Select all visible
  const handleSelectAll = useCallback(() => {
    setSelectedKeys(new Set(actors.filter(a => classFilter.has(a.class_name)).map(a => a.key)));
  }, [actors, classFilter]);

  // Focus on selected
  const handleFocusSelected = useCallback(() => {
    setFocusRequest(r => r + 1);
  }, []);

  // ---- Mutation handlers ----

  // Helper: refresh the actor list from backend after a mutation
  const refreshActors = useCallback(async () => {
    try {
      const actorList = await invoke<ActorListEntry[]>('list_actors', { classFilter: null });
      setActors(actorList);
    } catch (e) {
      console.error('list_actors failed:', e);
    }
  }, []);

  // Delete selected actors
  const handleDeleteActors = useCallback(async () => {
    if (selectedKeys.size === 0) return;
    const keys = [...selectedKeys];
    try {
      await invoke<number>('delete_actors', { keys });
      setSelectedKeys(new Set());
      setSelectedActorDetail(null);
      await refreshActors();
    } catch (e) {
      console.error('delete_actors failed:', e);
    }
  }, [selectedKeys, refreshActors]);

  // Duplicate selected actors
  const handleDuplicateActors = useCallback(async () => {
    if (selectedKeys.size === 0) return;
    const keys = [...selectedKeys];
    try {
      const newEntries = await invoke<ActorListEntry[]>('duplicate_actors', {
        keys,
        offsetX: 100,
        offsetY: 100,
        offsetZ: 0,
      });
      await refreshActors();
      // Select the newly duplicated actors
      setSelectedKeys(new Set(newEntries.map(e => e.key)));
      setSelectedActorDetail(null);
    } catch (e) {
      console.error('duplicate_actors failed:', e);
    }
  }, [selectedKeys, refreshActors]);

  // Paste actors (duplicate specific keys at same position)
  const handlePasteActors = useCallback(async (keys: string[]) => {
    if (keys.length === 0) return;
    try {
      const newEntries = await invoke<ActorListEntry[]>('duplicate_actors', {
        keys,
        offsetX: 0,
        offsetY: 0,
        offsetZ: 0,
      });
      await refreshActors();
      setSelectedKeys(new Set(newEntries.map(e => e.key)));
      setSelectedActorDetail(null);
    } catch (e) {
      console.error('paste_actors failed:', e);
    }
  }, [refreshActors]);

  // Undo
  const handleUndo = useCallback(async () => {
    try {
      const didUndo = await invoke<boolean>('undo');
      if (didUndo) {
        await refreshActors();
        // Clear selection since actors may have changed
        setSelectedKeys(new Set());
        setSelectedActorDetail(null);
      }
    } catch (e) {
      console.error('undo failed:', e);
    }
  }, [refreshActors]);

  // Redo
  const handleRedo = useCallback(async () => {
    try {
      const didRedo = await invoke<boolean>('redo');
      if (didRedo) {
        await refreshActors();
        setSelectedKeys(new Set());
        setSelectedActorDetail(null);
      }
    } catch (e) {
      console.error('redo failed:', e);
    }
  }, [refreshActors]);

  // Move selected actors (called from gizmo drag)
  const handleMoveActors = useCallback(async (
    keys: string[],
    newX: number[],
    newY: number[],
    newZ: number[],
  ) => {
    try {
      await invoke('move_actors', { keys, newX, newY, newZ });
      await refreshActors();
    } catch (e) {
      console.error('move_actors failed:', e);
    }
  }, [refreshActors]);

  // Rotate selected actors (called from gizmo drag)
  const handleRotateActors = useCallback(async (
    keys: string[],
    newPitch: number[],
    newYaw: number[],
    newRoll: number[],
  ) => {
    try {
      await invoke('rotate_actors', { keys, newPitch, newYaw, newRoll });
      await refreshActors();
    } catch (e) {
      console.error('rotate_actors failed:', e);
    }
  }, [refreshActors]);

  // Scale selected actors (called from gizmo drag)
  const handleScaleActors = useCallback(async (
    keys: string[],
    newDrawScale: number[],
    newDsx: number[],
    newDsy: number[],
    newDsz: number[],
  ) => {
    try {
      await invoke('scale_actors', { keys, newDrawScale, newDsx, newDsy, newDsz });
      await refreshActors();
    } catch (e) {
      console.error('scale_actors failed:', e);
    }
  }, [refreshActors]);

  // Move a single actor (called from property panel edits)
  const handleMoveOneActor = useCallback(async (key: string, x: number, y: number, z: number) => {
    try {
      await invoke('move_actors', { keys: [key], newX: [x], newY: [y], newZ: [z] });
      await refreshActors();
      // Refresh detail for the edited actor
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch (e) {
      console.error('move_actors failed:', e);
    }
  }, [refreshActors]);

  // Rotate a single actor (called from property panel edits)
  const handleRotateOneActor = useCallback(async (key: string, pitch: number, yaw: number, roll: number) => {
    try {
      await invoke('rotate_actors', { keys: [key], newPitch: [pitch], newYaw: [yaw], newRoll: [roll] });
      await refreshActors();
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch (e) {
      console.error('rotate_actors failed:', e);
    }
  }, [refreshActors]);

  // Scale a single actor (called from property panel edits)
  const handleScaleOneActor = useCallback(async (
    key: string, drawScale: number, dsX: number, dsY: number, dsZ: number,
  ) => {
    try {
      await invoke('scale_actors', {
        keys: [key], newDrawScale: [drawScale], newDsx: [dsX], newDsy: [dsY], newDsz: [dsZ],
      });
      await refreshActors();
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch (e) {
      console.error('scale_actors failed:', e);
    }
  }, [refreshActors]);

  // ---- Property change handler ----
  const handlePropertyChange = useCallback(async (key: string, propName: string, newValue: string) => {
    try {
      await invoke('update_actor_property', { key, propertyName: propName, newValue });
      // Refresh actor detail
      const detail = await invoke<ActorEntry>('get_actor_details', { key });
      setSelectedActorDetail(detail);
    } catch (e) {
      console.error('update_actor_property failed:', e);
    }
  }, []);

  // ---- Toggle class filter ----
  const handleToggleClass = useCallback((className: string) => {
    setClassFilter(prev => {
      const next = new Set(prev);
      if (next.has(className)) next.delete(className);
      else next.add(className);
      return next;
    });
  }, []);

  const handleToggleAllClasses = useCallback((enable: boolean) => {
    if (enable && loadedZone) {
      setClassFilter(new Set(Object.keys(loadedZone.class_counts)));
    } else {
      setClassFilter(new Set());
    }
  }, [loadedZone]);

  // Save zone as JSON
  const handleSave = useCallback(async () => {
    const zoneName = loadedZone?.zone_name ?? 'zone';
    const filePath = prompt('Save zone JSON to:', `${zoneName}.json`);
    if (!filePath) return;
    try {
      const count = await invoke<number>('save_zone', { filePath });
      alert(`Saved ${count} actors to ${filePath}`);
    } catch (e) {
      console.error('save_zone failed:', e);
      alert(`Save failed: ${e}`);
    }
  }, [loadedZone]);

  // Export SQL
  const handleExportSql = useCallback(async () => {
    const zoneName = loadedZone?.zone_name ?? 'zone';
    const filePath = prompt('Export SQL to:', `${zoneName}.sql`);
    if (!filePath) return;
    try {
      const count = await invoke<number>('export_sql', { filePath, classFilter: null });
      alert(`Exported ${count} actors to ${filePath}`);
    } catch (e) {
      console.error('export_sql failed:', e);
      alert(`Export failed: ${e}`);
    }
  }, [loadedZone]);

  // ---- Filtered actors ----
  const visibleActors = useMemo(() => {
    if (classFilter.size === 0) return [];
    return actors.filter(a => classFilter.has(a.class_name));
  }, [actors, classFilter]);

  // Primary selected key (last one, for detail panel)
  const primarySelectedKey = useMemo(() => {
    if (selectedKeys.size === 0) return null;
    return [...selectedKeys][selectedKeys.size - 1];
  }, [selectedKeys]);

  // ---- Render ----
  if (showZoneSelector) {
    return (
      <ZoneSelector
        zones={zones}
        loading={loading}
        loadingMessage={loadingMessage}
        onScan={handleScan}
        onLoadZone={handleLoadZone}
      />
    );
  }

  return (
    <>
      <EditorFrame
        loadedZone={loadedZone}
        actors={visibleActors}
        allActors={actors}
        selectedKeys={selectedKeys}
        primarySelectedKey={primarySelectedKey}
        selectedActorDetail={selectedActorDetail}
        classFilter={classFilter}
        mouseWorldPos={mouseWorldPos}
        meshProgress={meshProgress}
        focusRequest={focusRequest}
        onSelectActor={handleSelectActor}
        onToggleSelect={handleToggleSelect}
        onAddSelect={handleAddSelect}
        onBoxSelect={handleBoxSelect}
        onSelectAll={handleSelectAll}
        onFocusSelected={handleFocusSelected}
        onToggleClass={handleToggleClass}
        onToggleAllClasses={handleToggleAllClasses}
        onMouseWorldPosChange={setMouseWorldPos}
        onOpenZoneSelector={() => setShowZoneSelector(true)}
        onDeleteActors={handleDeleteActors}
        onDuplicateActors={handleDuplicateActors}
        onUndo={handleUndo}
        onRedo={handleRedo}
        onMoveActors={handleMoveActors}
        onRotateActors={handleRotateActors}
        onScaleActors={handleScaleActors}
        onMoveOneActor={handleMoveOneActor}
        onRotateOneActor={handleRotateOneActor}
        onScaleOneActor={handleScaleOneActor}
        onPasteActors={handlePasteActors}
        onSave={handleSave}
        onExportSql={handleExportSql}
        dbStatus={dbStatus}
        dbEntities={dbEntities}
        entityTemplates={entityTemplates}
        selectedDbEntityIds={selectedDbEntityIds}
        selectedTemplate={selectedTemplate}
        onOpenDbDialog={() => setShowDbDialog(true)}
        onDbConnect={handleDbConnect}
        onDbDisconnect={handleDbDisconnect}
        onLoadDbEntities={handleLoadDbEntities}
        onRefreshDbEntities={handleRefreshDbEntities}
        onSelectDbEntity={handleSelectDbEntity}
        onDeleteDbEntity={handleDeleteDbEntity}
        onSelectTemplate={handleSelectTemplate}
        placementMode={placementMode}
        onTogglePlacement={handleTogglePlacement}
        onPlaceSpawn={handlePlaceSpawn}
        onUpdateSpawnPosition={handleUpdateSpawnPosition}
        onCreateRegion={() => setShowRegionDialog(true)}
        onPropertyChange={handlePropertyChange}
        onProceduralPlacement={() => setShowProceduralDialog(true)}
        navMeshData={navMeshData}
        onExportDbSql={handleExportDbSql}
        showContentBrowser={showContentBrowser}
        onToggleContentBrowser={() => setShowContentBrowser(prev => !prev)}
        cookedPcPath={cookedPcPath}
      />
      {showDbDialog && (
        <DbConnectionDialog
          status={dbStatus}
          onConnect={handleDbConnect}
          onDisconnect={handleDbDisconnect}
          onClose={() => setShowDbDialog(false)}
        />
      )}
      {showProceduralDialog && (
        <ProceduralPlacementDialog
          sourceKey={selectedKeys.size === 1 ? [...selectedKeys][0] : null}
          onPlaced={handleProceduralPlaced}
          onClose={() => setShowProceduralDialog(false)}
        />
      )}
      {showRegionDialog && (
        <CreateRegionDialog
          position={mouseWorldPos}
          worlds={worlds}
          currentWorldId={currentWorldId}
          onCreated={handleCreateRegion}
          onClose={() => setShowRegionDialog(false)}
        />
      )}
    </>
  );
}
