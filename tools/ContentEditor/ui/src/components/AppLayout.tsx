import { useState, useCallback, useEffect, useRef } from 'react';
import { Allotment } from 'allotment';
import 'allotment/dist/style.css';
import type { Node } from '@xyflow/react';
import {
  Keyboard,
  Wand,
  Users,
  Zap,
  Settings,
  LogOut,
  Save,
  RefreshCw,
  FileDown,
  X,
} from 'lucide-react';
import { TreeNav } from './TreeNav';
import { Inspector } from './Inspector';
import { ChainEditor, type ChainEditorHandle } from '../editors/ChainEditor';
import { ScriptEditor, type ScriptEditorHandle } from '../editors/ScriptEditor';
import DataEditor, { type DataEditorHandle } from '../editors/DataEditor';
import MissionCardLibrary from './MissionCardLibrary';
import ValidationPanel from './ValidationPanel';
import ScriptNodePalette from './ScriptNodePalette';
import ScriptPropertyPanel from './ScriptPropertyPanel';
import ScriptBrowser from './ScriptBrowser';
import ConvertScriptDialog from './ConvertScriptDialog';
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
import type { ConnectionMeta } from '../App';

export interface SpaceEntry {
  space_id: string;
  chain_count: number;
}

export interface ChainData {
  chain_id: number;
  description: string | null;
  scope_type: string;
  scope_id: number | null;
  enabled: boolean;
  priority: number;
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

type Page = 'chains' | 'scripts' | 'data';
type BottomPanel = 'palette' | 'validation' | null;

interface AppLayoutProps {
  connectionMeta: ConnectionMeta | null;
  onDisconnect: () => void;
}

/* ── Relative time helper ── */
function formatRelativeTime(date: Date): string {
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (seconds < 10) return 'just now';
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}

export function AppLayout({ connectionMeta, onDisconnect }: AppLayoutProps) {
  // ----- Top-level page -----
  const [page, setPage] = useState<Page>('chains');

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
  const [showConvertDialog, setShowConvertDialog] = useState(false);

  // ----- Data editor state -----
  const dataEditorRef = useRef<DataEditorHandle>(null);

  // ----- HUD state -----
  const [lastSaveTime, setLastSaveTime] = useState<Date | null>(null);
  const [, setTick] = useState(0); // force re-render for relative time

  const activeTab = tabs.find((t) => t.id === activeTabId) ?? null;
  const totalChains = spaces.reduce((sum, s) => sum + s.chain_count, 0);

  // Selected chain frame ID (for adding nodes to the right parent)
  const selectedFrameId = selectedNode?.type === 'chainFrame'
    ? selectedNode.id
    : selectedNode?.parentId ?? null;

  const selectedChainName = selectedNode?.type === 'chainFrame'
    ? (selectedNode.data as any)?.name ?? 'Unnamed chain'
    : chains.find((c) => selectedNode?.parentId === `chain-${c.chain_id}`)?.description ?? 'No chain selected';

  // ----- Load chain spaces on mount -----
  useEffect(() => {
    invoke<SpaceEntry[]>('list_spaces')
      .then(setSpaces)
      .catch((e) => setStatus(`Failed to load spaces: ${e}`));
  }, []);

  // ----- Tick for relative time in HUD -----
  useEffect(() => {
    if (!lastSaveTime) return;
    const interval = setInterval(() => setTick((t) => t + 1), 30000);
    return () => clearInterval(interval);
  }, [lastSaveTime]);

  // ----- Load script templates/enums on first switch to Scripts page -----
  useEffect(() => {
    if (page !== 'scripts' || scriptsLoaded) return;

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
  }, [page, scriptsLoaded]);

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
    if (page === 'chains') {
      if (!editorRef.current) return;
      const chainData = editorRef.current.getChainData();
      if (chainData.length === 0) return;
      setStatus('Saving...');
      try {
        await invoke('save_chains', { chains: chainData });
        setStatus('Saved to database');
        setLastSaveTime(new Date());
      } catch (e) {
        setStatus(`Save failed: ${e}`);
      }
    } else if (page === 'scripts') {
      if (!scriptEditorRef.current || !activeScriptPath) return;
      const scriptData = scriptEditorRef.current.getScriptData();
      setStatus('Saving script...');
      try {
        await invoke('save_script', { path: activeScriptPath, script: scriptData });
        setStatus('Script saved');
        setLastSaveTime(new Date());
      } catch (e) {
        setStatus(`Save failed: ${e}`);
      }
    } else if (page === 'data') {
      if (!dataEditorRef.current) return;
      await dataEditorRef.current.save();
      setLastSaveTime(new Date());
    }
  }, [page, activeScriptPath]);

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
    if (page === 'chains') {
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
  }, [page, activeTab, activeScriptPath]);

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

  // ----- Nav items -----
  const navItems: { key: Page; label: string; icon: typeof Keyboard }[] = [
    { key: 'chains', label: 'Dialing Computer', icon: Keyboard },
    { key: 'scripts', label: 'Artifact Lab', icon: Wand },
    { key: 'data', label: 'Personnel', icon: Users },
  ];

  // ----- Render -----

  return (
    <div className="flex h-screen flex-col bg-surface">
      {/* ══════ TOP BAR ══════ */}
      <header className="fixed top-0 left-0 z-50 flex h-16 w-full items-center justify-between bg-surface/90 px-6 backdrop-blur-xl">
        <div className="flex items-center gap-8">
          <h1 className="font-headline text-2xl font-bold uppercase tracking-[0.3em] text-primary gold-glow">
            SGC ADMIN
          </h1>
          <nav className="hidden md:flex gap-6">
            <span className="font-headline text-sm uppercase tracking-[0.2em] text-primary border-b-2 border-primary pb-1">SECTOR_ALPHA</span>
            <span className="font-headline text-sm uppercase tracking-[0.2em] text-on-surface-variant">SECTOR_BETA</span>
            <span className="font-headline text-sm uppercase tracking-[0.2em] text-on-surface-variant">SECTOR_GAMMA</span>
          </nav>
        </div>
        <div className="flex items-center gap-1">
          {status && (
            <span className="mr-3 font-dense text-xs text-on-surface-variant">{status}</span>
          )}
          <TopBarButton icon={<Save className="h-4 w-4" />} label="Save" onClick={handleSave} />
          <TopBarButton icon={<RefreshCw className="h-4 w-4" />} label="Reload" onClick={handleHotReload} />
          <TopBarButton icon={<FileDown className="h-4 w-4" />} label="Export" onClick={handleExport} />
          <div className="ml-2 flex h-8 w-8 items-center justify-center bg-primary-container">
            <span className="font-headline text-xs text-on-primary">C</span>
          </div>
        </div>
        {/* Bottom hairline */}
        <div className="absolute bottom-0 left-0 h-[1px] w-full bg-gradient-to-r from-transparent via-[rgba(77,70,53,0.2)] to-transparent" />
      </header>

      {/* ══════ SIDEBAR ══════ */}
      <aside className="fixed left-0 top-0 z-40 flex h-full w-64 flex-col border-r border-[rgba(77,70,53,0.1)] bg-surface-lowest pt-20 pb-4 shadow-[20px_0_60px_rgba(0,0,0,0.5)]">
        {/* Header */}
        <div className="mb-8 px-6">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center border border-primary/20 bg-primary-container/20">
              <Zap className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h2 className="font-headline text-lg leading-tight tracking-tighter text-primary">COMMAND_STRATA</h2>
              <p className="font-dense text-[10px] uppercase tracking-widest text-on-surface-variant/50">V1.0.4_ANCIENT</p>
            </div>
          </div>
        </div>

        {/* Nav */}
        <nav className="flex-1 space-y-1 px-2">
          {navItems.map((item) => (
            <button
              key={item.key}
              onClick={() => setPage(item.key)}
              className={`flex w-full items-center gap-4 px-4 py-3 font-headline text-sm uppercase tracking-widest transition-all duration-200 ${
                page === item.key
                  ? 'bg-secondary-container/30 text-primary shadow-[inset_4px_0_0_0_#F2CA50]'
                  : 'text-on-surface-variant/70 hover:translate-x-1 hover:bg-surface-highest hover:text-on-surface-variant'
              }`}
              type="button"
            >
              <item.icon className="h-4 w-4" />
              {item.label}
            </button>
          ))}
        </nav>

        {/* Bottom */}
        <div className="mt-auto space-y-4 px-4">
          <button
            onClick={onDisconnect}
            className="bevel-stone flex w-full items-center justify-center gap-2 bg-primary-container py-4 font-headline text-xs font-bold uppercase tracking-[0.2em] text-on-primary transition-all active:scale-[0.98]"
            type="button"
          >
            <Zap className="h-4 w-4" />
            DIAL_GATE
          </button>
          <div className="space-y-1 border-t border-[rgba(77,70,53,0.1)] pt-4">
            <button className="flex w-full items-center gap-4 px-4 py-2 font-dense text-xs uppercase text-on-surface-variant/50 transition-colors hover:text-primary" type="button">
              <Settings className="h-3.5 w-3.5" />
              Settings
            </button>
            <button
              onClick={onDisconnect}
              className="flex w-full items-center gap-4 px-4 py-2 font-dense text-xs uppercase text-error/70 transition-colors hover:text-error"
              type="button"
            >
              <LogOut className="h-3.5 w-3.5" />
              Relinquish
            </button>
          </div>
        </div>
      </aside>

      {/* ══════ MAIN CONTENT ══════ */}
      <main className="ml-64 flex-1 overflow-hidden pt-16">
        {/* Chain tabs bar (only in chains page) */}
        {page === 'chains' && (
          <div className="flex items-center border-b border-[rgba(77,70,53,0.15)] bg-surface-low">
            <div className="flex flex-1 items-center overflow-x-auto">
              {tabs.map((tab) => (
                <div
                  key={tab.id}
                  className={`group flex items-center gap-1.5 border-r border-[rgba(77,70,53,0.15)] px-3 py-2 text-sm transition-colors ${
                    activeTabId === tab.id
                      ? 'bg-surface text-on-surface shadow-[inset_0_-2px_0_0_#F2CA50]'
                      : 'text-on-surface-variant hover:bg-surface-highest hover:text-on-surface'
                  }`}
                >
                  <button onClick={() => setActiveTabId(tab.id)} className="truncate font-dense" type="button">
                    {tab.label}
                  </button>
                  <button
                    onClick={(e) => { e.stopPropagation(); closeTab(tab.id); }}
                    className="ml-1 p-0.5 opacity-0 transition-opacity hover:bg-error/20 group-hover:opacity-100"
                    type="button"
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              ))}
              {tabs.length === 0 && (
                <div className="px-3 py-2 font-dense text-sm text-on-surface-variant/50">
                  No editors open
                </div>
              )}
            </div>
          </div>
        )}

        {/* Page content */}
        <div className={`hieroglyph-bg ${page === 'chains' ? 'h-[calc(100vh-64px-41px)]' : 'h-[calc(100vh-64px)]'} overflow-hidden`}>
          {page === 'data' ? (
            /* ========== DATA EDITOR ========== */
            <DataEditor ref={dataEditorRef} onStatus={setStatus} />
          ) : page === 'chains' ? (
            /* ========== CHAIN EDITOR ========== */
            <Allotment>
              <Allotment.Pane preferredSize={240} minSize={180} maxSize={400}>
                <TreeNav
                  spaces={spaces}
                  onSelectSpace={openSpace}
                  activeSpaceId={activeTab?.spaceId ?? null}
                />
              </Allotment.Pane>

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
                            className={`bevel-stone border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                              bottomPanel === 'palette'
                                ? 'border-primary/40 bg-primary/18 text-primary'
                                : 'border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 text-on-surface-variant/70 hover:bg-surface-highest'
                            }`}
                            onClick={handleTogglePalette}
                            type="button"
                          >
                            Card Palette
                          </button>
                          <button
                            className={`bevel-stone border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                              bottomPanel === 'validation'
                                ? 'border-tertiary/40 bg-tertiary/18 text-tertiary'
                                : 'border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 text-on-surface-variant/70 hover:bg-surface-highest'
                            }`}
                            onClick={handleToggleValidation}
                            type="button"
                          >
                            Validation
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div className="flex h-full items-center justify-center font-headline text-on-surface-variant/50 uppercase tracking-widest">
                        Select a space from the tree to begin editing
                      </div>
                    )}
                  </Allotment.Pane>

                  {bottomPanel !== null && (
                    <Allotment.Pane preferredSize={380} minSize={200} maxSize={600}>
                      <div className="subtle-scrollbar h-full overflow-y-auto border-t border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 p-4">
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

              <Allotment.Pane preferredSize={320} minSize={240} maxSize={500}>
                <Inspector selectedNode={selectedNode} />
              </Allotment.Pane>
            </Allotment>
          ) : (
            /* ========== SCRIPT EDITOR ========== */
            <Allotment>
              <Allotment.Pane preferredSize={240} minSize={180} maxSize={400}>
                <ScriptBrowser
                  scripts={scriptEntries}
                  activeScriptPath={activeScriptPath}
                  onSelectScript={openScript}
                />
              </Allotment.Pane>

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
                        <div className="absolute bottom-3 left-3 z-20 flex gap-2">
                          <button
                            className={`bevel-stone border px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] shadow-lg transition-colors ${
                              scriptBottomPanel
                                ? 'border-primary/40 bg-primary/18 text-primary'
                                : 'border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 text-on-surface-variant/70 hover:bg-surface-highest'
                            }`}
                            onClick={handleScriptTogglePalette}
                            type="button"
                          >
                            Node Palette
                          </button>
                          {activeScript?.script_type === 'Level' && (
                            <button
                              className="bevel-stone border border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 px-3 py-1.5 text-[11px] font-semibold uppercase tracking-[0.16em] text-on-surface-variant/70 shadow-lg transition-colors hover:border-secondary/40 hover:bg-secondary-container/18 hover:text-secondary"
                              onClick={() => setShowConvertDialog(true)}
                              type="button"
                            >
                              Convert to Chains
                            </button>
                          )}
                        </div>
                      </div>
                    ) : (
                      <div className="flex h-full items-center justify-center font-headline text-on-surface-variant/50 uppercase tracking-widest">
                        Select a script from the browser to begin editing
                      </div>
                    )}
                  </Allotment.Pane>

                  {scriptBottomPanel && (
                    <Allotment.Pane preferredSize={380} minSize={200} maxSize={600}>
                      <div className="subtle-scrollbar h-full overflow-y-auto border-t border-[rgba(77,70,53,0.15)] bg-surface-lowest/95 p-4">
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
      </main>

      {/* ══════ FLOATING HUD ══════ */}
      <div className="fixed bottom-10 left-[calc(256px+2.5rem)] z-50 hidden border border-[rgba(77,70,53,0.15)] bg-surface-low/95 p-5 shadow-[0_4px_40px_rgba(0,27,60,0.06)] backdrop-blur-md xl:block">
        <div className="flex items-center gap-6">
          <div>
            <div className="font-dense text-[10px] uppercase text-on-surface-variant/50">Connected</div>
            <div className="font-headline text-sm tracking-wider text-primary">
              {connectionMeta?.db ?? 'unknown'}
            </div>
          </div>
          <div className="h-8 w-[1px] bg-primary/20" />
          <div>
            <div className="font-dense text-[10px] uppercase text-on-surface-variant/50">Chains</div>
            <div className="font-headline text-sm tracking-wider text-primary">{totalChains}</div>
          </div>
          <div className="h-8 w-[1px] bg-primary/20" />
          <div>
            <div className="font-dense text-[10px] uppercase text-on-surface-variant/50">Last Save</div>
            <div className="font-headline text-sm tracking-wider text-tertiary">
              {lastSaveTime ? formatRelativeTime(lastSaveTime) : '—'}
            </div>
          </div>
        </div>
      </div>

      {/* Convert script dialog */}
      {showConvertDialog && activeScriptPath && (
        <ConvertScriptDialog
          scriptPath={activeScriptPath}
          onClose={() => setShowConvertDialog(false)}
          onStatus={setStatus}
        />
      )}
    </div>
  );
}

/* ----- Top bar button sub-component ----- */

function TopBarButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      className="flex items-center gap-1.5 px-2.5 py-1.5 text-sm text-on-surface-variant transition-colors hover:bg-surface-highest hover:text-primary"
      type="button"
    >
      {icon}
      <span className="hidden font-dense lg:inline">{label}</span>
    </button>
  );
}
