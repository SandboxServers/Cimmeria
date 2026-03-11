import { useMemo, useState } from 'react';
import {
  ChevronDown,
  ChevronRight,
  FileCode2,
  Folder,
  FolderOpen,
} from 'lucide-react';
import type { ScriptEntry } from '../editors/scriptTypes';

interface ScriptBrowserProps {
  scripts: ScriptEntry[];
  activeScriptPath: string | null;
  onSelectScript: (path: string) => void;
}

/**
 * Build a tree structure from flat script entries.
 * Groups by script_type first, then by module path segments.
 */
type TreeNode = {
  name: string;
  fullPath?: string;
  children: Map<string, TreeNode>;
  isLeaf: boolean;
};

function buildTree(scripts: ScriptEntry[]): TreeNode {
  const root: TreeNode = { name: 'root', children: new Map(), isLeaf: false };

  for (const entry of scripts) {
    // Group: scriptType / module / fileName
    const segments = [entry.script_type];
    if (entry.module) {
      segments.push(...entry.module.split('/').filter(Boolean));
    }
    segments.push(entry.file_name);

    let current = root;
    for (let i = 0; i < segments.length; i++) {
      const seg = segments[i];
      const isLast = i === segments.length - 1;
      if (!current.children.has(seg)) {
        current.children.set(seg, {
          name: seg,
          fullPath: isLast ? entry.path : undefined,
          children: new Map(),
          isLeaf: isLast,
        });
      }
      current = current.children.get(seg)!;
    }
  }

  return root;
}

/**
 * A collapsible tree browser showing all .script files organized by type and module.
 */
export default function ScriptBrowser({
  scripts,
  activeScriptPath,
  onSelectScript,
}: ScriptBrowserProps) {
  const tree = useMemo(() => buildTree(scripts), [scripts]);

  return (
    <div className="subtle-scrollbar flex h-full flex-col overflow-y-auto border-r border-border bg-card">
      <div className="border-b border-border px-3 py-2">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Script Browser
        </h2>
      </div>

      {scripts.length === 0 ? (
        <div className="px-4 py-6 text-center text-xs text-muted-foreground">
          No scripts loaded.
          <br />
          Switch to Scripts mode to load the script catalog.
        </div>
      ) : (
        <div className="pb-2">
          {Array.from(tree.children.entries())
            .sort((a, b) => a[0].localeCompare(b[0]))
            .map(([key, node]) => (
              <TreeBranch
                key={key}
                node={node}
                depth={0}
                activeScriptPath={activeScriptPath}
                onSelectScript={onSelectScript}
              />
            ))}
        </div>
      )}
    </div>
  );
}

/* ----- Recursive tree branch ----- */

function TreeBranch({
  node,
  depth,
  activeScriptPath,
  onSelectScript,
}: {
  node: TreeNode;
  depth: number;
  activeScriptPath: string | null;
  onSelectScript: (path: string) => void;
}) {
  const [expanded, setExpanded] = useState(depth < 1);

  if (node.isLeaf && node.fullPath) {
    const isActive = activeScriptPath === node.fullPath;
    return (
      <button
        className={`flex w-full items-center gap-2 py-1 text-sm transition-colors hover:bg-secondary/50 ${
          isActive
            ? 'bg-secondary/70 text-foreground'
            : 'text-muted-foreground'
        }`}
        onClick={() => onSelectScript(node.fullPath!)}
        style={{ paddingLeft: 12 + depth * 16 }}
      >
        <FileCode2 className="h-3.5 w-3.5 shrink-0 text-[rgba(34,197,94,0.7)]" />
        <span className="truncate">{node.name}</span>
      </button>
    );
  }

  const sortedChildren = Array.from(node.children.entries()).sort((a, b) => {
    // Folders first, then files
    if (a[1].isLeaf !== b[1].isLeaf) return a[1].isLeaf ? 1 : -1;
    return a[0].localeCompare(b[0]);
  });

  return (
    <div>
      <button
        className="flex w-full items-center gap-1.5 py-1 text-sm font-medium text-foreground hover:bg-secondary/50"
        onClick={() => setExpanded((e) => !e)}
        style={{ paddingLeft: 8 + depth * 16 }}
      >
        {expanded ? (
          <ChevronDown className="h-3.5 w-3.5 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
        )}
        {expanded ? (
          <FolderOpen className="h-4 w-4 text-primary" />
        ) : (
          <Folder className="h-4 w-4 text-primary" />
        )}
        <span className="truncate">{node.name}</span>
        <span className="ml-auto pr-3 text-[10px] text-muted-foreground">
          {countLeaves(node)}
        </span>
      </button>

      {expanded && (
        <div>
          {sortedChildren.map(([key, child]) => (
            <TreeBranch
              key={key}
              node={child}
              depth={depth + 1}
              activeScriptPath={activeScriptPath}
              onSelectScript={onSelectScript}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * Count total leaf (file) nodes in a subtree.
 */
function countLeaves(node: TreeNode): number {
  if (node.isLeaf) return 1;
  let count = 0;
  for (const child of node.children.values()) {
    count += countLeaves(child);
  }
  return count;
}
