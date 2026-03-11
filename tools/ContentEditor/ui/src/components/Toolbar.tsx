import {
  Save,
  RefreshCw,
  FileDown,
  X,
} from 'lucide-react';

interface Tab {
  id: string;
  label: string;
}

interface ToolbarProps {
  tabs: Tab[];
  activeTabId: string | null;
  onTabSelect: (id: string) => void;
  onTabClose: (id: string) => void;
  onSave: () => void;
  onHotReload: () => void;
  onExport: () => void;
  status: string | null;
}

export function Toolbar({
  tabs,
  activeTabId,
  onTabSelect,
  onTabClose,
  onSave,
  onHotReload,
  onExport,
  status,
}: ToolbarProps) {
  return (
    <div className="flex items-center border-b border-border bg-card">
      {/* Tabs */}
      <div className="flex flex-1 items-center overflow-x-auto">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={`group flex items-center gap-1.5 border-r border-border px-3 py-2 text-sm transition-colors ${
              activeTabId === tab.id
                ? 'bg-background text-foreground'
                : 'text-muted-foreground hover:bg-secondary/30 hover:text-foreground'
            }`}
          >
            <button
              onClick={() => onTabSelect(tab.id)}
              className="truncate"
            >
              {tab.label}
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(tab.id);
              }}
              className="ml-1 rounded p-0.5 opacity-0 transition-opacity hover:bg-destructive/20 group-hover:opacity-100"
            >
              <X className="h-3 w-3" />
            </button>
          </div>
        ))}

        {tabs.length === 0 && (
          <div className="px-3 py-2 text-sm text-muted-foreground">
            No editors open
          </div>
        )}
      </div>

      {/* Action buttons */}
      <div className="flex items-center gap-1 px-2">
        {status && (
          <span className="mr-2 text-xs text-muted-foreground">{status}</span>
        )}
        <ToolbarButton
          icon={<Save className="h-4 w-4" />}
          label="Save to DB"
          onClick={onSave}
          shortcut="Ctrl+S"
        />
        <ToolbarButton
          icon={<RefreshCw className="h-4 w-4" />}
          label="Hot Reload"
          onClick={onHotReload}
          shortcut="Ctrl+R"
        />
        <ToolbarButton
          icon={<FileDown className="h-4 w-4" />}
          label="Export SQL"
          onClick={onExport}
          shortcut="Ctrl+E"
        />
      </div>
    </div>
  );
}

function ToolbarButton({
  icon,
  label,
  onClick,
  shortcut,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  shortcut?: string;
}) {
  return (
    <button
      onClick={onClick}
      title={shortcut ? `${label} (${shortcut})` : label}
      className="flex items-center gap-1.5 rounded px-2.5 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
    >
      {icon}
      <span className="hidden lg:inline">{label}</span>
    </button>
  );
}
