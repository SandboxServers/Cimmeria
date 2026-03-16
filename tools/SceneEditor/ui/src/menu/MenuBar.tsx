import { useState, useRef, useEffect } from 'react';
import type { ZoneSummary } from '../lib/types';

interface MenuBarProps {
  loadedZone: ZoneSummary | null;
  classFilter: Set<string>;
  hasSelection: boolean;
  onOpenZone: () => void;
  onToggleClass: (className: string) => void;
  onToggleAllClasses: (enable: boolean) => void;
  onSelectAll: () => void;
  onSelectNone: () => void;
  onSelectSameClass: () => void;
  onInvertSelection: () => void;
  onSearch: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onExportSql: () => void;
  onDeleteActors: () => void;
  onDuplicateActors: () => void;
  onAlignToGrid?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  hasClipboard?: boolean;
  // Content Browser
  onToggleContentBrowser?: () => void;
  // DB menu
  dbConnected?: boolean;
  hasDbEntities?: boolean;
  onOpenDbDialog?: () => void;
  onDbDisconnect?: () => void;
  onLoadDbEntities?: () => void;
  onRefreshDbEntities?: () => void;
  onExportDbSql?: () => void;
  onCreateRegion?: () => void;
}

type MenuId = 'file' | 'edit' | 'view' | 'build' | 'database' | 'tools' | 'help' | null;

export function MenuBar({
  loadedZone,
  classFilter,
  hasSelection,
  onOpenZone,
  onToggleClass,
  onToggleAllClasses,
  onSelectAll,
  onSelectNone,
  onSelectSameClass,
  onInvertSelection,
  onSearch,
  onUndo,
  onRedo,
  onSave,
  onExportSql,
  onDeleteActors,
  onDuplicateActors,
  onAlignToGrid,
  onCopy,
  onPaste,
  hasClipboard,
  onToggleContentBrowser,
  dbConnected,
  hasDbEntities,
  onOpenDbDialog,
  onDbDisconnect,
  onLoadDbEntities,
  onRefreshDbEntities,
  onExportDbSql,
  onCreateRegion,
}: MenuBarProps) {
  const [openMenu, setOpenMenu] = useState<MenuId>(null);
  const barRef = useRef<HTMLDivElement>(null);

  // Close menu on outside click
  useEffect(() => {
    if (!openMenu) return;
    const handler = (e: MouseEvent) => {
      if (barRef.current && !barRef.current.contains(e.target as Node)) {
        setOpenMenu(null);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [openMenu]);

  const toggleMenu = (id: MenuId) => {
    setOpenMenu(prev => (prev === id ? null : id));
  };

  const closeAndRun = (fn: () => void) => {
    setOpenMenu(null);
    fn();
  };

  // Class categories for the View menu
  const classCategories: Record<string, string[]> = {
    'Geometry': ['StaticMeshActor', 'InterpActor', 'Brush', 'Terrain', 'SkeletalMeshActor', 'KActor'],
    'Lights': ['PointLight', 'SpotLight', 'DirectionalLight', 'SkyLight', 'PointLightMovable'],
    'Cover': ['SGWSpecCoverNode', 'SGWCoverSet', 'CoverLink'],
    'Triggers': ['Trigger', 'TriggerVolume'],
    'Volumes': ['BlockingVolume', 'PhysicsVolume'],
    'Spawns': ['SGWSpawnRegion', 'SGWSpawnSet', 'SGWMob', 'SGWNpc', 'PlayerStart'],
    'SGW Objects': ['SGWStargate', 'SGWInteractable', 'SGWDoor', 'SGWTeleporter', 'SGWMinigame'],
    'Effects': ['Emitter', 'DecalActor', 'SpeedTreeActor', 'HeightFog', 'SGWSkyDomeActor'],
    'Audio': ['AmbientSoundSGW', 'AmbientSound'],
    'Navigation': ['PathNode'],
    'Other': ['WorldInfo', 'Note', 'CameraActor', 'PrefabInstance', 'MaterialInstanceActor'],
  };

  return (
    <div ref={barRef} className="relative flex h-7 shrink-0 items-center border-b bg-card px-1 text-xs select-none">
      {/* Menu buttons */}
      {(['file', 'edit', 'view', 'build', 'database', 'tools', 'help'] as const).map(id => (
        <button
          key={id}
          className={`rounded px-2.5 py-0.5 hover:bg-muted ${openMenu === id ? 'bg-muted text-foreground' : 'text-muted-foreground'}`}
          onClick={() => toggleMenu(id)}
          onMouseEnter={() => openMenu && setOpenMenu(id)}
        >
          {id.charAt(0).toUpperCase() + id.slice(1)}
        </button>
      ))}

      {/* Dropdown menus */}
      {openMenu === 'file' && (
        <MenuDropdown left={4}>
          <MenuItem label="Open Zone..." onClick={() => closeAndRun(onOpenZone)} />
          <MenuItem label="Save As JSON..." onClick={() => closeAndRun(onSave)} shortcut="Ctrl+S" />
          <MenuItem label="Export SQL..." onClick={() => closeAndRun(onExportSql)} />
          <MenuSeparator />
          <MenuItem label="Exit" onClick={() => closeAndRun(() => window.close())} />
        </MenuDropdown>
      )}

      {openMenu === 'edit' && (
        <MenuDropdown left={52}>
          <MenuItem label="Undo" onClick={() => closeAndRun(onUndo)} shortcut="Ctrl+Z" />
          <MenuItem label="Redo" onClick={() => closeAndRun(onRedo)} shortcut="Ctrl+Y" />
          <MenuSeparator />
          <MenuItem label="Copy" onClick={() => closeAndRun(() => onCopy?.())} shortcut="Ctrl+C" disabled={!hasSelection} />
          <MenuItem label="Paste" onClick={() => closeAndRun(() => onPaste?.())} shortcut="Ctrl+V" disabled={!hasClipboard} />
          <MenuItem label="Duplicate" onClick={() => closeAndRun(onDuplicateActors)} shortcut="Ctrl+D" disabled={!hasSelection} />
          <MenuItem label="Delete" onClick={() => closeAndRun(onDeleteActors)} shortcut="Del" disabled={!hasSelection} />
          {onAlignToGrid && (
            <MenuItem label="Align to Grid" onClick={() => closeAndRun(onAlignToGrid)} disabled={!hasSelection} />
          )}
          <MenuSeparator />
          <MenuItem label="Select All" onClick={() => closeAndRun(onSelectAll)} shortcut="Ctrl+A" />
          <MenuItem label="Select None" onClick={() => closeAndRun(onSelectNone)} shortcut="Esc" />
          <MenuItem label="Select Same Class" onClick={() => closeAndRun(onSelectSameClass)} disabled={!hasSelection} />
          <MenuItem label="Invert Selection" onClick={() => closeAndRun(onInvertSelection)} shortcut="Ctrl+I" />
          <MenuSeparator />
          <MenuItem label="Search Actors..." onClick={() => closeAndRun(onSearch)} shortcut="Ctrl+F" />
        </MenuDropdown>
      )}

      {openMenu === 'view' && (
        <MenuDropdown left={92}>
          <MenuItem
            label="Content Browser"
            onClick={() => closeAndRun(() => onToggleContentBrowser?.())}
            shortcut="Ctrl+Shift+F"
          />
          <MenuSeparator />
          <MenuItem
            label="Show All Classes"
            onClick={() => closeAndRun(() => onToggleAllClasses(true))}
            shortcut="Shift+H"
          />
          <MenuItem
            label="Hide All Classes"
            onClick={() => closeAndRun(() => onToggleAllClasses(false))}
          />
          <MenuItem
            label="Hide Selected Classes"
            onClick={() => closeAndRun(onSelectNone)}
            shortcut="H"
          />
          <MenuSeparator />
          {Object.entries(classCategories).map(([category, classes]) => {
            const availableClasses = classes.filter(
              c => loadedZone?.class_counts[c]
            );
            if (availableClasses.length === 0) return null;
            return (
              <div key={category}>
                <div className="px-3 py-0.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
                  {category}
                </div>
                {availableClasses.map(cls => (
                  <MenuCheckbox
                    key={cls}
                    label={`${cls} (${loadedZone?.class_counts[cls] ?? 0})`}
                    checked={classFilter.has(cls)}
                    onChange={() => onToggleClass(cls)}
                  />
                ))}
              </div>
            );
          })}
        </MenuDropdown>
      )}

      {openMenu === 'build' && (
        <MenuDropdown left={132}>
          <MenuItem label="Build All" disabled />
          <MenuItem label="Build Geometry" disabled />
          <MenuItem label="Build Lighting" disabled />
          <MenuItem label="Build Paths" disabled />
        </MenuDropdown>
      )}

      {openMenu === 'database' && (
        <MenuDropdown left={178}>
          <MenuItem
            label={dbConnected ? 'Connection...' : 'Connect...'}
            onClick={() => closeAndRun(() => onOpenDbDialog?.())}
          />
          <MenuItem
            label="Disconnect"
            onClick={() => closeAndRun(() => onDbDisconnect?.())}
            disabled={!dbConnected}
          />
          <MenuSeparator />
          <MenuItem
            label="Load Entities"
            onClick={() => closeAndRun(() => onLoadDbEntities?.())}
            disabled={!dbConnected}
          />
          <MenuItem
            label="Refresh Entities"
            onClick={() => closeAndRun(() => onRefreshDbEntities?.())}
            disabled={!dbConnected || !hasDbEntities}
          />
          <MenuSeparator />
          <MenuItem
            label="Create Region..."
            onClick={() => closeAndRun(() => onCreateRegion?.())}
            disabled={!dbConnected}
          />
          <MenuSeparator />
          <MenuItem
            label="Export to SQL..."
            onClick={() => closeAndRun(() => onExportDbSql?.())}
            disabled={!hasDbEntities}
          />
        </MenuDropdown>
      )}

      {openMenu === 'tools' && (
        <MenuDropdown left={248}>
          <MenuItem label="Search Actors" onClick={() => closeAndRun(onSearch)} shortcut="Ctrl+F" />
        </MenuDropdown>
      )}

      {openMenu === 'help' && (
        <MenuDropdown left={290}>
          <MenuItem
            label="About Scene Editor"
            onClick={() => closeAndRun(() => {
              alert('Cimmeria Scene Editor v0.4.0\nUnrealEd-style editor with 3D viewport, multi-select, move/rotate/scale gizmos, box select, copy/paste, and CME tools');
            })}
          />
        </MenuDropdown>
      )}
    </div>
  );
}

// ---- Sub-components ----

function MenuDropdown({ left, children }: { left: number; children: React.ReactNode }) {
  return (
    <div
      className="absolute top-7 z-50 min-w-[200px] rounded-md border bg-card py-1 shadow-lg"
      style={{ left }}
    >
      {children}
    </div>
  );
}

function MenuItem({
  label,
  onClick,
  disabled,
  shortcut,
}: {
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  shortcut?: string;
}) {
  return (
    <button
      className={`flex w-full items-center justify-between px-3 py-1 text-left text-xs ${
        disabled
          ? 'cursor-default text-muted-foreground/50'
          : 'text-foreground hover:bg-muted'
      }`}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
    >
      <span>{label}</span>
      {shortcut && (
        <span className="ml-6 font-mono text-[10px] text-muted-foreground">{shortcut}</span>
      )}
    </button>
  );
}

function MenuCheckbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <button
      className="flex w-full items-center gap-2 px-3 py-0.5 text-left text-xs text-foreground hover:bg-muted"
      onClick={onChange}
    >
      <span className="w-3 text-center font-mono text-primary">
        {checked ? '\u2713' : ''}
      </span>
      <span>{label}</span>
    </button>
  );
}

function MenuSeparator() {
  return <div className="my-1 border-t border-border" />;
}
