import { useState, useCallback, useEffect, useRef } from 'react';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import type { Node } from '@xyflow/react';
import { TreeNav } from './TreeNav';
import { Inspector } from './Inspector';
import { Toolbar } from './Toolbar';
import { ChainEditor, type ChainEditorHandle } from '../editors/ChainEditor';
import { ScriptEditor, type ScriptEditorHandle } from '../editors/ScriptEditor';
import MissionCardLibrary from './MissionCardLibrary';
import ValidationPanel from './ValidationPanel';
import ScriptNodePalette from './ScriptNodePalette';
import ScriptPropertyPanel from './ScriptPropertyPanel';
import ScriptBrowser from './ScriptBrowser';
import { buildValidationReport, type ValidationIssue } from '../lib/chainValidation';
import { invoke } from '../lib/tauri';
import type { EditorNodeData } from '../editors/types';
import type {
  CompileResult,
  EnumDefinition,
  ScriptEntry,
  ScriptFileData,
  ScriptNodeData,
  ScriptNodeTemplate,
} from '../editors/scriptTypes';

export interface SpaceEntry {
  space_id: string;
  chain_count: number;
}

export interface ChainData {
  chain_id: number;
  name: string | null;
  description: string | null;
  scope_type: string;
  scope_id: number | null;
  enabled: boolean;
  priority: number;
  editor_data: Record<string, unknown>;
  triggers: unknown[];
  conditions: unknown[];
  actions: unknown[];
  counters: unknown[];
}

type EditorTab = {
  id: string;
  label: string;
  type: 'chains';
  spaceId: string;
  missionId?: string;
};

type EditorMode = 'chains' | 'scripts';
type BottomPanel = 'palette' | 'validation' | null;

export function AppLayout() {
  // ----- Top-level mode -----
  const [mode, setMode] = useState<EditorMode>('chains');

  // ----- Chain editor state -----
  const [spaces, setSpaces] = useState<SpaceEntry[]>([]);
  const [tabs, setTabs] = useState<EditorTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [chains, setChains] = useState<ChainData[]>([]);
  const [selectedNode, setSelectedNode] = useState<Node<EditorNodeData> | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [bottomPanel, setBottomPanel] = useState<BottomPanel>(null);
  const editorRef = useRef<ChainEditorHandle>(null);

  // ----- Script editor state -----
  const [scriptTemplates, setScriptTemplates] = useState<ScriptNodeTemplate[]>([]);
  const [scriptEnums, setScriptEnums] = useState<EnumDefinition[]>([]);
  const [scriptEntries, setScriptEntries] = useState<ScriptEntry[]>([]);
  const [activeScript, setActiveScript] = useState<ScriptFileData | null>(null);
  const [activeScriptPath, setActiveScriptPath] = useState<string | null>(null);
  const [selectedScriptNodeId, setSelectedScriptNodeId] = useState<number | null>(null);
  const [scriptBottomPanel, setScriptBottomPanel] = useState<boolean>(false);
  const scriptEditorRef = useRef<ScriptEditorHandle>(null);
  const [scriptsLoaded, setScriptsLoaded] = useState(false);

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;

  // Selected chain frame ID (for adding nodes to the right parent)
  const selectedFrameId = selectedNode?.type === 'chainFrame'
    ? selectedNode.id
    : selectedNode?.parentId ?? null;

  const selectedChainName = selectedNode?.type === 'chainFrame'
    ? (selectedNode.data as any)?.name ?? 'Unnamed chain'
    : chains.find((c) => selectedNode?.parentId === `chain-${c.chain_id}`)?.name ?? 'No chain selected';

  // ----- Load chain spaces on mount -----
  useEffect(() => {
    invoke<SpaceEntry[]>('list_spaces')
      .then(setSpaces)
      .catch((e) => setStatus(`Failed to load spaces: ${e}`));
  }, []);

  // ----- Load script templates/enums on first switch to Scripts mode -----
  useEffect(() => {
    if (mode !== 'scripts' || scriptsLoaded) return;

    setStatus('Loading script catalog...');
    Promise.all([
      invoke<ScriptNodeTemplate[]>('load_node_templates').catch(() => [] as ScriptNodeTemplate[]),
      invoke<EnumDefinition[]>('load_enumerations').catch(() => [] as EnumDefinition[]),
      invoke<ScriptEntry[]>('list_scripts').catch(() => [] as ScriptEntry[]),
    ])
      .then(([templates, enums, entries]) => {
        setScriptTemplates(templates);
        setScriptEnums(enums);
        setScriptEntries(entries);
        setScriptsLoaded(true);
        setStatus(`Loaded ${templates.length} templates, ${entries.length} scripts`);
      })
      .catch((e) => setStatus(`Failed to load scripts: ${e}`));
  }, [mode, scriptsLoaded]);

  // ----- Chain editor callbacks -----

  const openSpace = useCallback(
    async (spaceId: string) => {
      const existingTab = tabs.find(
        (t) => t.type === 'chains' && t.spaceId === spaceId,
      );
      if (existingTab) {
        setActiveTabId(existingTab.id);
        return;
      }

      const tabId = `chains-${spaceId}`;
      const newTab: EditorTab = {
        id: tabId,
        label: spaceId,
        type: 'chains',
        spaceId,
      };
      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(tabId);

      try {
        const loaded = await invoke<ChainData[]>('load_chains', {
          spaceId,
          missionId: null,
        });
        setChains(loaded);
        setSelectedNode(null);
      } catch (e) {
        setStatus(`Failed to load chains: ${e}`);
      }
    },
    [tabs],
  );

  const closeTab = useCallback(
    (tabId: string) => {
      setTabs((prev) => prev.filter((t) => t.id !== tabId));
      if (activeTabId === tabId) {
        setActiveTabId(tabs.find((t) => t.id !== tabId)?.id ?? null);
        setChains([]);
        setSelectedNode(null);
      }
    },
    [activeTabId, tabs],
  );

  const handleSave = useCallback(async () => {
    if (mode === 'chains') {
      if (!editorRef.current) return;
      const chainData = editorRef.current.getChainData();
      if (chainData.length === 0) return;
      setStatus('Saving...');
      try {
        await invoke('save_chains', { chains: chainData });
        setStatus('Saved to database');
      } catch (e) {
        setStatus(`Save failed: ${e}`);
      }
    } else {
      if (!scriptEditorRef.current || !activeScriptPath) return;
      const scriptData = scriptEditorRef.current.getScriptData();
      setStatus('Saving script...');
      try {
        await invoke('save_script', { path: activeScriptPath, script: scriptData });
        setStatus('Script saved');
      } catch (e) {
        setStatus(`Save failed: ${e}`);
      }
    }
  }, [mode, activeScriptPath]);

  const handleHotReload = useCallback(async () => {
    setStatus('Reloading...');
    try {
      const msg = await invoke<string>('hot_reload');
      setStatus(msg);
    } catch (e) {
      setStatus(`Reload failed: ${e}`);
    }
  }, []);

  const handleExport = useCallback(async () => {
    if (mode === 'chains') {
      if (!activeTab) return;
      setStatus('Exporting...');
      try {
        const path = await invoke<string>('export_to_seed_file', {
          spaceId: activeTab.spaceId,
        });
        setStatus(`Exported to ${path}`);
      } catch (e) {
        setStatus(`Export failed: ${e}`);
      }
    } else {
      if (!activeScriptPath) return;
      setStatus('Compiling script...');
      try {
        const result = await invoke<CompileResult>('compile_script', {
          path: activeScriptPath,
        });
        const warnMsg = result.warnings.length > 0
          ? ` (${result.warnings.length} warnings)`
          : '';
        setStatus(`Compiled to ${result.output_path}${warnMsg}`);
      } catch (e) {
        setStatus(`Compile failed: ${e}`);
      }
    }
  }, [mode, activeTab, activeScriptPath]);

  const handleTogglePalette = useCallback(() => {
    setBottomPanel((p) => (p === 'palette' ? null : 'palette'));
  }, []);

  const handleToggleValidation = useCallback(() => {
    setBottomPanel((p) => (p === 'validation' ? null : 'validation'));
  }, []);

  const handleAddFromPalette = useCallback(
    (template: any) => {
      editorRef.current?.addNodeFromTemplate(template, selectedFrameId ?? undefined);
    },
    [selectedFrameId],
  );

  const handleFocusIssue = useCallback((_issue: ValidationIssue) => {
    // TODO: zoom to issue node on canvas
  }, []);

  // Validation report (recomputed on demand when panel is visible)
  const validationReport = buildValidationReport({
    chains: [],
    edges: [],
    nodes: [],
  });

  // ----- Script editor callbacks -----

  const openScript = useCallback(
    async (path: string) => {
      setStatus('Loading script...');
      try {
        const data = await invoke<ScriptFileData>('load_script', { path });
        setActiveScript(data);
        setActiveScriptPath(path);
        setSelectedScriptNodeId(null);
        setStatus(null);
      } catch (e) {
        setStatus(`Failed to load script: ${e}`);
      }
    },
    [],
  );

  const handleScriptNodeSelect = useCallback((nodeId: number | null) => {
    setSelectedScriptNodeId(nodeId);
  }, []);

  const handleScriptPropertyChange = useCallback(
    (nodeId: number, key: string, value: string) => {
      // Update the active script data so the property panel reflects changes
      if (!activeScript) return;
      setActiveScript((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          nodes: prev.nodes.map((n) => {
            if (n.id !== nodeId) return n;
            const updated = [...n.properties];
            const idx = updated.findIndex(([k]) => k === key);
            if (idx >= 0) {
              updated[idx] = [key, value];
            } else {
              updated.push([key, value]);
            }
            return { ...n, properties: updated };
          }),
        };
      });
    },
    [activeScript],
  );

  const handleScriptTogglePalette = useCallback(() => {
    setScriptBottomPanel((p) => !p);
  }, []);

  const handleAddScriptNode = useCallback(
    (templateRef: string) => {
      scriptEditorRef.current?.addNode(templateRef);
    },
    [],
  );

  // ----- Derive selected script node data for property panel -----
  const selectedScriptNodeData: ScriptNodeData | null = (() => {
    if (selectedScriptNodeId == null || !activeScript) return null;
    const inst = activeScript.nodes.find((n) => n.id === selectedScriptNodeId);
    if (!inst) return null;
    const template = scriptTemplates.find((t) => t.ref_name === inst.ref_name);
    const connectedPorts = new Set<string>();
    for (const conn of activeScript.connections) {
      if (conn.out_node === inst.id) connectedPorts.add(`out-${conn.out_port}`);
      if (conn.in_node === inst.id) connectedPorts.add(`in-${conn.in_port}`);
    }
    const propsMap: Record<string, string> = {};
    for (const [k, v] of inst.properties) {
      propsMap[k] = v;
    }
    return {
      nodeId: inst.id,
      templateRef: inst.ref_name,
      displayName: template?.display_name ?? inst.ref_name,
      nodeType: template?.node_type ?? 'Action',
      category: template?.category ?? '',
      description: template?.description ?? '',
      inputPorts: (template?.ports ?? [])
        .filter((p) => p.direction === 'in')
        .map((p) => ({
          name: p.name,
          portType: p.port_type,
          visible: !p.default_hide || connectedPorts.has(`in-${p.name}`),
        })),
      outputPorts: (template?.ports ?? [])
        .filter((p) => p.direction === 'out')
        .map((p) => ({
          name: p.name,
          portType: p.port_type,
          visible: !p.default_hide || connectedPorts.has(`out-${p.name}`),
        })),
      properties: propsMap,
      enabled: propsMap['_enabled'] !== 'false',
      debug: propsMap['_debug'] === 'true',
      comment: propsMap['_comment'] ?? '',
    };
  })();

  const selectedScriptTemplate = selectedScriptNodeData
    ? scriptTemplates.find((t) => t.ref_name === selectedScriptNodeData.templateRef) ?? null
    : null;

  // ----- Render -----

  return (
    <div className="flex h-screen flex-col">
      {/* Mode tabs + Toolbar */}
      <div className="flex items-center border-b border-border bg-card">
        {/* Mode selector */}
        <div className="flex items-center border-r border-border">
          <ModeTab
            label="Chains"
            active={mode === 'chains'}
            onClick={() => setMode('chains')}
          />
          <ModeTab
            label="Scripts"
            active={mode === 'scripts'}
            onClick={() => setMode('scripts')}
          />
        </div>

        {/* Toolbar (shared) */}
        <div className="flex-1">
          <Toolbar
            tabs={mode === 'chains' ? tabs : []}
            activeTabId={mode === 'chains' ? activeTabId : null}
            onTabSelect={setActiveTabId}
            onTabClose={closeTab}
            onSave={handleSave}
            onHotReload={handleHotReload}
            onExport={handleExport}
            status={status}
          />
        </div>
      </div>

      {/* Main panels */}
      <div className="flex-1 overflow-hidden">
        {mode === 'chains' ? (
          /* ========== CHAIN EDITOR MODE ========== */
          <Allotment>
            {/* Left: Tree navigation */}
            <Allotment.Pane preferredSize={240} minSize={180} maxSize={400}>
              <TreeNav
                spaces={spaces}
                onSelectSpace={openSpace}
                activeSpaceId={activeTab?.spaceId ?? null}
              />
            </Allotment.Pane>

            {/* Center: Editor canvas + optional bottom panel */}
            <Allotment.Pane>
              <Allotment vertical>
                <Allotment.Pane>
                  {activeTab ? (
                    <div className="relative h-full w-full">
                      <ChainEditor
                        key={activeTab.id}
                        ref={editorRef}
                        chains={chains}
                        spaceId={activeTab.spaceId}
                        selectedFrameId={selectedFrameId}
                        onNodeSelect={setSelectedNode}
                      />
                      {/* Floating panel toggles */}
                      <div className="absolute bottom-3 left-3 z-20 flex gap-2">
                        <button
                          className={`rounded-full border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                            bottomPanel === 'palette'
                              ? 'border-[rgba(245,170,49,0.4)] bg-[rgba(245,170,49,0.18)] text-[#ffd38a]'
                              : 'border-[rgba(255,255,255,0.1)] bg-[rgba(9,18,28,0.92)] text-[rgba(224,231,239,0.72)] hover:bg-[rgba(255,255,255,0.06)]'
                          }`}
                          onClick={handleTogglePalette}
                          type="button"
                        >
                          Card Palette
                        </button>
                        <button
                          className={`rounded-full border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                            bottomPanel === 'validation'
                              ? 'border-[rgba(34,197,94,0.4)] bg-[rgba(34,197,94,0.18)] text-[#c7ffd5]'
                              : 'border-[rgba(255,255,255,0.1)] bg-[rgba(9,18,28,0.92)] text-[rgba(224,231,239,0.72)] hover:bg-[rgba(255,255,255,0.06)]'
                          }`}
                          onClick={handleToggleValidation}
                          type="button"
                        >
                          Validation
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="flex h-full items-center justify-center text-muted-foreground">
                      Select a space from the tree to begin editing
                    </div>
                  )}
                </Allotment.Pane>

                {/* Bottom panel: palette or validation */}
                {bottomPanel !== null && (
                  <Allotment.Pane preferredSize={380} minSize={200} maxSize={600}>
                    <div className="subtle-scrollbar h-full overflow-y-auto border-t border-[rgba(255,255,255,0.06)] bg-[rgba(5,12,20,0.95)] p-4">
                      {bottomPanel === 'palette' ? (
                        <MissionCardLibrary
                          onAdd={handleAddFromPalette}
                          selectedChainName={selectedChainName}
                        />
                      ) : (
                        <ValidationPanel
                          report={validationReport}
                          onFocusIssue={handleFocusIssue}
                        />
                      )}
                    </div>
                  </Allotment.Pane>
                )}
              </Allotment>
            </Allotment.Pane>

            {/* Right: Inspector */}
            <Allotment.Pane preferredSize={320} minSize={240} maxSize={500}>
              <Inspector selectedNode={selectedNode} />
            </Allotment.Pane>
          </Allotment>
        ) : (
          /* ========== SCRIPT EDITOR MODE ========== */
          <Allotment>
            {/* Left: Script browser */}
            <Allotment.Pane preferredSize={240} minSize={180} maxSize={400}>
              <ScriptBrowser
                scripts={scriptEntries}
                activeScriptPath={activeScriptPath}
                onSelectScript={openScript}
              />
            </Allotment.Pane>

            {/* Center: Script canvas + optional bottom palette */}
            <Allotment.Pane>
              <Allotment vertical>
                <Allotment.Pane>
                  {activeScript ? (
                    <div className="relative h-full w-full">
                      <ScriptEditor
                        key={activeScriptPath}
                        ref={scriptEditorRef}
                        script={activeScript}
                        templates={scriptTemplates}
                        onNodeSelect={handleScriptNodeSelect}
                      />
                      {/* Floating palette toggle */}
                      <div className="absolute bottom-3 left-3 z-20 flex gap-2">
                        <button
                          className={`rounded-full border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                            scriptBottomPanel
                              ? 'border-[rgba(168,85,247,0.4)] bg-[rgba(168,85,247,0.18)] text-[#c4b5fd]'
                              : 'border-[rgba(255,255,255,0.1)] bg-[rgba(9,18,28,0.92)] text-[rgba(224,231,239,0.72)] hover:bg-[rgba(255,255,255,0.06)]'
                          }`}
                          onClick={handleScriptTogglePalette}
                          type="button"
                        >
                          Node Palette
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div className="flex h-full items-center justify-center text-muted-foreground">
                      Select a script from the browser to begin editing
                    </div>
                  )}
                </Allotment.Pane>

                {/* Bottom panel: Script node palette */}
                {scriptBottomPanel && (
                  <Allotment.Pane preferredSize={380} minSize={200} maxSize={600}>
                    <div className="subtle-scrollbar h-full overflow-y-auto border-t border-[rgba(255,255,255,0.06)] bg-[rgba(5,12,20,0.95)] p-4">
                      <ScriptNodePalette
                        templates={scriptTemplates}
                        scriptType={activeScript?.script_type ?? 'Mission'}
                        onAdd={handleAddScriptNode}
                      />
                    </div>
                  </Allotment.Pane>
                )}
              </Allotment>
            </Allotment.Pane>

            {/* Right: Script property panel */}
            <Allotment.Pane preferredSize={320} minSize={240} maxSize={500}>
              <ScriptPropertyPanel
                nodeData={selectedScriptNodeData}
                template={selectedScriptTemplate}
                enums={scriptEnums}
                onPropertyChange={handleScriptPropertyChange}
              />
            </Allotment.Pane>
          </Allotment>
        )}
      </div>
    </div>
  );
}

/* ----- Mode tab sub-component ----- */

function ModeTab({
  label,
  active,
  onClick,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`px-4 py-2.5 text-[11px] font-semibold uppercase tracking-[0.18em] transition-colors ${
        active
          ? 'bg-background text-foreground'
          : 'text-muted-foreground hover:bg-secondary/30 hover:text-foreground'
      }`}
      onClick={onClick}
      type="button"
    >
      {label}
    </button>
  );
}
