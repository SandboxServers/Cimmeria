import {
  useState,
  useCallback,
  useEffect,
  useRef,
  forwardRef,
  useImperativeHandle,
} from 'react';
import {
  Users,
  Package,
  Map,
  Dice5,
  Swords,
  Plus,
  Save,
  Trash2,
} from 'lucide-react';
import { invoke } from '../lib/tauri';
import DataGrid from '../components/DataGrid';
import EntityTemplateForm from '../components/EntityTemplateForm';
import ItemForm from '../components/ItemForm';
import MissionForm from '../components/MissionForm';
import LootTableForm from '../components/LootTableForm';
import AbilityForm from '../components/AbilityForm';
import type {
  DataCategory,
  EntityTemplateSummary,
  EntityTemplate,
  ItemSummary,
  Item,
  MissionSummary,
  Mission,
  LootTableSummary,
  LootTable,
  AbilitySummary,
  Ability,
} from './dataTypes';

/* ---- Public handle for parent component ---- */
export interface DataEditorHandle {
  save: () => Promise<void>;
}

/* ---- Category metadata ---- */
const CATEGORIES: {
  key: DataCategory;
  label: string;
  icon: typeof Users;
}[] = [
  { key: 'entities', label: 'Entities', icon: Users },
  { key: 'items', label: 'Items', icon: Package },
  { key: 'missions', label: 'Missions', icon: Map },
  { key: 'loot', label: 'Loot Tables', icon: Dice5 },
  { key: 'abilities', label: 'Abilities', icon: Swords },
];

/* ---- Column configs ---- */
const ENTITY_COLUMNS: { key: keyof EntityTemplateSummary; label: string; width?: string }[] = [
  { key: 'template_id', label: 'ID', width: '60px' },
  { key: 'name', label: 'Name' },
  { key: 'class', label: 'Class', width: '100px' },
  { key: 'level', label: 'Lvl', width: '50px' },
];

const ITEM_COLUMNS: { key: keyof ItemSummary; label: string; width?: string }[] = [
  { key: 'item_id', label: 'ID', width: '60px' },
  { key: 'name', label: 'Name' },
  { key: 'quality_id', label: 'Quality', width: '60px' },
  { key: 'tier', label: 'Tier', width: '50px' },
];

const MISSION_COLUMNS: { key: keyof MissionSummary; label: string; width?: string }[] = [
  { key: 'mission_id', label: 'ID', width: '60px' },
  { key: 'mission_label', label: 'Label' },
  { key: 'level', label: 'Lvl', width: '50px' },
  { key: 'is_enabled', label: 'On', width: '40px' },
];

const LOOT_COLUMNS: { key: keyof LootTableSummary; label: string; width?: string }[] = [
  { key: 'loot_table_id', label: 'ID', width: '60px' },
  { key: 'description', label: 'Description' },
  { key: 'entry_count', label: 'Entries', width: '60px' },
];

const ABILITY_COLUMNS: { key: keyof AbilitySummary; label: string; width?: string }[] = [
  { key: 'ability_id', label: 'ID', width: '60px' },
  { key: 'name', label: 'Name' },
  { key: 'type_id', label: 'Type', width: '60px' },
  { key: 'cooldown', label: 'CD', width: '60px' },
];

/* ---- Main component ---- */

const DataEditor = forwardRef<DataEditorHandle, { onStatus?: (msg: string | null) => void }>(
  function DataEditor({ onStatus }, ref) {
    const [category, setCategory] = useState<DataCategory>('entities');
    const [searchValue, setSearchValue] = useState('');
    const [dirty, setDirty] = useState(false);

    // List data per category (lazy loaded)
    const [entityList, setEntityList] = useState<EntityTemplateSummary[]>([]);
    const [itemList, setItemList] = useState<ItemSummary[]>([]);
    const [missionList, setMissionList] = useState<MissionSummary[]>([]);
    const [lootList, setLootList] = useState<LootTableSummary[]>([]);
    const [abilityList, setAbilityList] = useState<AbilitySummary[]>([]);
    const loadedCategories = useRef(new Set<DataCategory>());

    // Selected record (full detail)
    const [selectedId, setSelectedId] = useState<number | null>(null);
    const [entityDetail, setEntityDetail] = useState<EntityTemplate | null>(null);
    const [itemDetail, setItemDetail] = useState<Item | null>(null);
    const [missionDetail, setMissionDetail] = useState<Mission | null>(null);
    const [lootDetail, setLootDetail] = useState<LootTable | null>(null);
    const [abilityDetail, setAbilityDetail] = useState<Ability | null>(null);

    const status = useCallback(
      (msg: string | null) => { onStatus?.(msg); },
      [onStatus],
    );

    // ---- Load list data on category change ----
    useEffect(() => {
      if (loadedCategories.current.has(category)) return;

      status(`Loading ${category}...`);

      const commands: Record<DataCategory, string> = {
        entities: 'list_entity_templates',
        items: 'list_items',
        missions: 'list_missions',
        loot: 'list_loot_tables',
        abilities: 'list_abilities',
      };

      invoke<unknown[]>(commands[category])
        .then((data) => {
          loadedCategories.current.add(category);
          switch (category) {
            case 'entities': setEntityList(data as EntityTemplateSummary[]); break;
            case 'items': setItemList(data as ItemSummary[]); break;
            case 'missions': setMissionList(data as MissionSummary[]); break;
            case 'loot': setLootList(data as LootTableSummary[]); break;
            case 'abilities': setAbilityList(data as AbilitySummary[]); break;
          }
          status(`Loaded ${data.length} ${category}`);
        })
        .catch((e) => status(`Failed to load ${category}: ${e}`));
    }, [category, status]);

    // ---- Select record (load detail) ----
    const selectRecord = useCallback(
      async (id: number) => {
        setSelectedId(id);
        setDirty(false);

        const commands: Record<DataCategory, string> = {
          entities: 'get_entity_template',
          items: 'get_item',
          missions: 'get_mission',
          loot: 'get_loot_table',
          abilities: 'get_ability',
        };

        const idKeys: Record<DataCategory, string> = {
          entities: 'templateId',
          items: 'itemId',
          missions: 'missionId',
          loot: 'lootTableId',
          abilities: 'abilityId',
        };

        try {
          const data = await invoke<unknown>(commands[category], { [idKeys[category]]: id });
          switch (category) {
            case 'entities': setEntityDetail(data as EntityTemplate); break;
            case 'items': setItemDetail(data as Item); break;
            case 'missions': setMissionDetail(data as Mission); break;
            case 'loot': setLootDetail(data as LootTable); break;
            case 'abilities': setAbilityDetail(data as Ability); break;
          }
        } catch (e) {
          status(`Failed to load record: ${e}`);
        }
      },
      [category, status],
    );

    // ---- Category change resets selection ----
    const switchCategory = useCallback((cat: DataCategory) => {
      setCategory(cat);
      setSelectedId(null);
      setEntityDetail(null);
      setItemDetail(null);
      setMissionDetail(null);
      setLootDetail(null);
      setAbilityDetail(null);
      setSearchValue('');
      setDirty(false);
    }, []);

    // ---- Save handler ----
    const handleSave = useCallback(async () => {
      const commands: Record<DataCategory, string> = {
        entities: 'save_entity_template',
        items: 'save_item',
        missions: 'save_mission',
        loot: 'save_loot_table',
        abilities: 'save_ability',
      };

      const details: Record<DataCategory, unknown> = {
        entities: entityDetail,
        items: itemDetail,
        missions: missionDetail,
        loot: lootDetail,
        abilities: abilityDetail,
      };

      const detail = details[category];
      if (!detail) return;

      status('Saving...');
      try {
        await invoke(commands[category], { data: detail });
        setDirty(false);
        // Refresh list
        loadedCategories.current.delete(category);
        status('Saved');
      } catch (e) {
        status(`Save failed: ${e}`);
      }
    }, [category, entityDetail, itemDetail, missionDetail, lootDetail, abilityDetail, status]);

    // ---- Delete handler ----
    const handleDelete = useCallback(async () => {
      if (selectedId == null) return;

      const commands: Record<DataCategory, string> = {
        entities: 'delete_entity_template',
        items: 'delete_item',
        missions: 'delete_mission',
        loot: 'delete_loot_table',
        abilities: 'delete_ability',
      };

      const idKeys: Record<DataCategory, string> = {
        entities: 'templateId',
        items: 'itemId',
        missions: 'missionId',
        loot: 'lootTableId',
        abilities: 'abilityId',
      };

      status('Deleting...');
      try {
        await invoke(commands[category], { [idKeys[category]]: selectedId });
        setSelectedId(null);
        setEntityDetail(null);
        setItemDetail(null);
        setMissionDetail(null);
        setLootDetail(null);
        setAbilityDetail(null);
        setDirty(false);
        // Refresh list
        loadedCategories.current.delete(category);
        status('Deleted');
      } catch (e) {
        status(`Delete failed: ${e}`);
      }
    }, [category, selectedId, status]);

    // ---- New record handler ----
    const handleNew = useCallback(async () => {
      const commands: Record<DataCategory, string> = {
        entities: 'new_entity_template',
        items: 'new_item',
        missions: 'new_mission',
        loot: 'new_loot_table',
        abilities: 'new_ability',
      };

      status('Creating new record...');
      try {
        const data = await invoke<unknown>(commands[category]);
        switch (category) {
          case 'entities': {
            const d = data as EntityTemplate;
            setEntityDetail(d);
            setSelectedId(d.template_id);
            break;
          }
          case 'items': {
            const d = data as Item;
            setItemDetail(d);
            setSelectedId(d.item_id);
            break;
          }
          case 'missions': {
            const d = data as Mission;
            setMissionDetail(d);
            setSelectedId(d.mission_id);
            break;
          }
          case 'loot': {
            const d = data as LootTable;
            setLootDetail(d);
            setSelectedId(d.loot_table_id);
            break;
          }
          case 'abilities': {
            const d = data as Ability;
            setAbilityDetail(d);
            setSelectedId(d.ability_id);
            break;
          }
        }
        setDirty(true);
        loadedCategories.current.delete(category);
        status('New record created');
      } catch (e) {
        status(`Failed to create: ${e}`);
      }
    }, [category, status]);

    // ---- Expose save to parent ----
    useImperativeHandle(ref, () => ({ save: handleSave }), [handleSave]);

    // ---- Detail change handlers (mark dirty) ----
    const onEntityChange = useCallback((d: EntityTemplate) => { setEntityDetail(d); setDirty(true); }, []);
    const onItemChange = useCallback((d: Item) => { setItemDetail(d); setDirty(true); }, []);
    const onMissionChange = useCallback((d: Mission) => { setMissionDetail(d); setDirty(true); }, []);
    const onLootChange = useCallback((d: LootTable) => { setLootDetail(d); setDirty(true); }, []);
    const onAbilityChange = useCallback((d: Ability) => { setAbilityDetail(d); setDirty(true); }, []);

    // ---- Render helpers ----
    const renderGrid = () => {
      switch (category) {
        case 'entities':
          return (
            <DataGrid
              data={entityList}
              columns={ENTITY_COLUMNS}
              onSelect={(r) => selectRecord(r.template_id)}
              selectedId={selectedId}
              idKey="template_id"
              searchValue={searchValue}
              onSearchChange={setSearchValue}
              searchPlaceholder="Search entities..."
            />
          );
        case 'items':
          return (
            <DataGrid
              data={itemList}
              columns={ITEM_COLUMNS}
              onSelect={(r) => selectRecord(r.item_id)}
              selectedId={selectedId}
              idKey="item_id"
              searchValue={searchValue}
              onSearchChange={setSearchValue}
              searchPlaceholder="Search items..."
            />
          );
        case 'missions':
          return (
            <DataGrid
              data={missionList}
              columns={MISSION_COLUMNS}
              onSelect={(r) => selectRecord(r.mission_id)}
              selectedId={selectedId}
              idKey="mission_id"
              searchValue={searchValue}
              onSearchChange={setSearchValue}
              searchPlaceholder="Search missions..."
            />
          );
        case 'loot':
          return (
            <DataGrid
              data={lootList}
              columns={LOOT_COLUMNS}
              onSelect={(r) => selectRecord(r.loot_table_id)}
              selectedId={selectedId}
              idKey="loot_table_id"
              searchValue={searchValue}
              onSearchChange={setSearchValue}
              searchPlaceholder="Search loot tables..."
            />
          );
        case 'abilities':
          return (
            <DataGrid
              data={abilityList}
              columns={ABILITY_COLUMNS}
              onSelect={(r) => selectRecord(r.ability_id)}
              selectedId={selectedId}
              idKey="ability_id"
              searchValue={searchValue}
              onSearchChange={setSearchValue}
              searchPlaceholder="Search abilities..."
            />
          );
      }
    };

    const renderDetail = () => {
      switch (category) {
        case 'entities':
          return entityDetail ? (
            <EntityTemplateForm data={entityDetail} onChange={onEntityChange} />
          ) : null;
        case 'items':
          return itemDetail ? (
            <ItemForm data={itemDetail} onChange={onItemChange} />
          ) : null;
        case 'missions':
          return missionDetail ? (
            <MissionForm data={missionDetail} onChange={onMissionChange} />
          ) : null;
        case 'loot':
          return lootDetail ? (
            <LootTableForm data={lootDetail} onChange={onLootChange} />
          ) : null;
        case 'abilities':
          return abilityDetail ? (
            <AbilityForm data={abilityDetail} onChange={onAbilityChange} />
          ) : null;
      }
    };

    const hasDetail = (() => {
      switch (category) {
        case 'entities': return entityDetail != null;
        case 'items': return itemDetail != null;
        case 'missions': return missionDetail != null;
        case 'loot': return lootDetail != null;
        case 'abilities': return abilityDetail != null;
      }
    })();

    return (
      <div className="flex h-full flex-col">
        {/* Category tabs */}
        <div className="flex items-center gap-0 border-b border-border bg-card">
          {CATEGORIES.map(({ key, label, icon: Icon }) => (
            <button
              key={key}
              type="button"
              onClick={() => switchCategory(key)}
              className={`flex items-center gap-1.5 px-3 py-2 text-[11px] font-semibold uppercase tracking-[0.14em] transition-colors ${
                category === key
                  ? 'border-b-2 border-primary bg-background text-foreground'
                  : 'text-muted-foreground hover:bg-secondary/30 hover:text-foreground'
              }`}
            >
              <Icon className="h-3.5 w-3.5" />
              {label}
            </button>
          ))}

          {/* CRUD toolbar */}
          <div className="ml-auto flex items-center gap-1 pr-2">
            <button
              type="button"
              onClick={handleNew}
              className="flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium text-primary hover:bg-primary/10"
              title="New record"
            >
              <Plus className="h-3.5 w-3.5" />
              New
            </button>
            <button
              type="button"
              onClick={handleSave}
              disabled={!dirty}
              className={`flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium transition-colors ${
                dirty
                  ? 'bg-primary/15 text-primary hover:bg-primary/25'
                  : 'text-muted-foreground/50 cursor-not-allowed'
              }`}
              title="Save record"
            >
              <Save className="h-3.5 w-3.5" />
              Save
            </button>
            <button
              type="button"
              onClick={handleDelete}
              disabled={selectedId == null}
              className={`flex items-center gap-1 rounded px-2 py-1 text-[10px] font-medium transition-colors ${
                selectedId != null
                  ? 'text-destructive hover:bg-destructive/10'
                  : 'text-muted-foreground/50 cursor-not-allowed'
              }`}
              title="Delete record"
            >
              <Trash2 className="h-3.5 w-3.5" />
              Delete
            </button>
          </div>
        </div>

        {/* Main content: list + detail */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left: data grid */}
          <div className="w-[340px] shrink-0 border-r border-border bg-card">
            {renderGrid()}
          </div>

          {/* Right: detail form */}
          <div className="flex-1 overflow-hidden bg-background">
            {hasDetail ? (
              renderDetail()
            ) : (
              <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                {selectedId != null
                  ? 'Loading...'
                  : 'Select a record from the list to edit'}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  },
);

export default DataEditor;
