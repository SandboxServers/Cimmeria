import { useEffect, useRef } from 'react';
import { Copy, Trash2, Focus, Eye, EyeOff, ClipboardPaste } from 'lucide-react';

export interface ContextMenuAction {
  label: string;
  icon?: React.ReactNode;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
  onClick: () => void;
}

interface ContextMenuProps {
  x: number;
  y: number;
  actions: ContextMenuAction[];
  onClose: () => void;
}

export function ContextMenu({ x, y, actions, onClose }: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('mousedown', handleClick);
    window.addEventListener('keydown', handleKey);
    return () => {
      window.removeEventListener('mousedown', handleClick);
      window.removeEventListener('keydown', handleKey);
    };
  }, [onClose]);

  // Adjust position to stay on screen
  const style: React.CSSProperties = {
    position: 'fixed',
    left: x,
    top: y,
    zIndex: 9999,
  };

  return (
    <div
      ref={menuRef}
      className="min-w-[180px] rounded-md border bg-popover py-1 shadow-xl"
      style={style}
    >
      {actions.map((action, i) => (
        <div key={i}>
          {action.separator && i > 0 && <div className="my-1 h-px bg-border" />}
          <button
            className="flex w-full items-center gap-2 px-3 py-1 text-left text-xs text-foreground hover:bg-muted disabled:opacity-40 disabled:hover:bg-transparent"
            disabled={action.disabled}
            onClick={() => {
              action.onClick();
              onClose();
            }}
          >
            <span className="w-4 shrink-0 text-muted-foreground">{action.icon}</span>
            <span className="flex-1">{action.label}</span>
            {action.shortcut && (
              <span className="ml-4 text-[10px] text-muted-foreground">{action.shortcut}</span>
            )}
          </button>
        </div>
      ))}
    </div>
  );
}

/** Build the standard actor context menu actions. */
export function buildActorContextMenu(opts: {
  hasSelection: boolean;
  selectionCount: number;
  onFocus: () => void;
  onSelectAll: () => void;
  onDeselectAll: () => void;
  onSelectSameClass?: () => void;
  onInvertSelection?: () => void;
  onCopy?: () => void;
  onPaste?: () => void;
  onDelete?: () => void;
}): ContextMenuAction[] {
  const actions: ContextMenuAction[] = [];

  actions.push({
    label: 'Focus Selected',
    icon: <Focus size={12} />,
    shortcut: 'F',
    disabled: !opts.hasSelection,
    onClick: opts.onFocus,
  });

  actions.push({
    label: 'Select All',
    shortcut: 'Ctrl+A',
    onClick: opts.onSelectAll,
    separator: true,
  });

  actions.push({
    label: 'Deselect All',
    shortcut: 'Esc',
    onClick: opts.onDeselectAll,
  });

  if (opts.onSelectSameClass) {
    actions.push({
      label: 'Select Same Class',
      disabled: !opts.hasSelection,
      onClick: opts.onSelectSameClass,
    });
  }

  if (opts.onInvertSelection) {
    actions.push({
      label: 'Invert Selection',
      shortcut: 'Ctrl+I',
      onClick: opts.onInvertSelection,
    });
  }

  if (opts.onCopy) {
    actions.push({
      label: opts.selectionCount > 1 ? `Copy ${opts.selectionCount} Actors` : 'Copy Actor',
      icon: <Copy size={12} />,
      shortcut: 'Ctrl+C',
      disabled: !opts.hasSelection,
      separator: true,
      onClick: opts.onCopy,
    });
  }

  if (opts.onPaste) {
    actions.push({
      label: 'Paste',
      icon: <ClipboardPaste size={12} />,
      shortcut: 'Ctrl+V',
      onClick: opts.onPaste,
    });
  }

  if (opts.onDelete) {
    actions.push({
      label: opts.selectionCount > 1 ? `Delete ${opts.selectionCount} Actors` : 'Delete Actor',
      icon: <Trash2 size={12} />,
      shortcut: 'Del',
      disabled: !opts.hasSelection,
      separator: true,
      onClick: opts.onDelete,
    });
  }

  return actions;
}
