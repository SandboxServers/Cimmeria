import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '../lib/tauri';
import { X, GitBranch, ZoomIn, ZoomOut, Maximize2 } from 'lucide-react';

interface KismetGraphData {
  nodes: KismetNodeData[];
  edges: KismetEdge[];
  sequences: KismetSequenceData[];
  node_count: number;
  edge_count: number;
}

interface KismetNodeData {
  key: string;
  class_name: string;
  object_name: string;
  parent_sequence: string;
  parent_name: string;
  tile: string;
  comment: string;
  category: string;
  input_pins: string[];
  output_pins: string[];
  variable_pins: string[];
  properties: Record<string, string>;
}

interface KismetEdge {
  source_key: string;
  source_pin: number;
  target_key: string;
  target_pin: number;
  edge_type: string;
}

interface KismetSequenceData {
  key: string;
  name: string;
  child_count: number;
}

interface KismetViewerProps {
  visible: boolean;
  onClose: () => void;
}

const CATEGORY_COLORS: Record<string, string> = {
  event: '#e74c3c',
  action: '#3498db',
  condition: '#f39c12',
  variable: '#2ecc71',
  sequence: '#9b59b6',
  other: '#95a5a6',
};

const NODE_WIDTH = 180;
const NODE_HEIGHT = 60;
const PIN_SPACING = 14;

export function KismetViewer({ visible, onClose }: KismetViewerProps) {
  const [graph, setGraph] = useState<KismetGraphData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedSequence, setSelectedSequence] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<KismetNodeData | null>(null);

  // Canvas state
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [zoom, setZoom] = useState(0.5);
  const [panX, setPanX] = useState(0);
  const [panY, setPanY] = useState(0);
  const [nodePositions, setNodePositions] = useState<Map<string, { x: number; y: number }>>(new Map());

  // Load graph
  const loadGraph = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<KismetGraphData>('extract_kismet_graph');
      setGraph(data);

      // Auto-layout nodes by sequence
      const positions = new Map<string, { x: number; y: number }>();
      const sequenceNodes = new Map<string, KismetNodeData[]>();

      for (const node of data.nodes) {
        const seq = node.parent_sequence || '_root';
        if (!sequenceNodes.has(seq)) sequenceNodes.set(seq, []);
        sequenceNodes.get(seq)!.push(node);
      }

      let globalY = 0;
      for (const [_seq, nodes] of sequenceNodes) {
        // Simple grid layout within each sequence
        const cols = Math.ceil(Math.sqrt(nodes.length));
        for (let i = 0; i < nodes.length; i++) {
          const col = i % cols;
          const row = Math.floor(i / cols);
          positions.set(nodes[i].key, {
            x: col * (NODE_WIDTH + 40) + 20,
            y: globalY + row * (NODE_HEIGHT + 30) + 20,
          });
        }
        const rows = Math.ceil(nodes.length / cols);
        globalY += rows * (NODE_HEIGHT + 30) + 60;
      }

      setNodePositions(positions);

      // Select the first sequence if any
      if (data.sequences.length > 0) {
        setSelectedSequence(data.sequences[0].key);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (visible && !graph) loadGraph();
  }, [visible, graph, loadGraph]);

  // Draw the graph
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !graph) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * window.devicePixelRatio;
    canvas.height = rect.height * window.devicePixelRatio;
    ctx.scale(window.devicePixelRatio, window.devicePixelRatio);

    ctx.clearRect(0, 0, rect.width, rect.height);
    ctx.save();
    ctx.translate(panX, panY);
    ctx.scale(zoom, zoom);

    // Filter nodes by selected sequence
    const visibleNodes = selectedSequence
      ? graph.nodes.filter(n => n.parent_sequence === selectedSequence)
      : graph.nodes;

    const visibleKeys = new Set(visibleNodes.map(n => n.key));

    // Draw edges first (behind nodes)
    ctx.lineWidth = 1.5;
    for (const edge of graph.edges) {
      if (!visibleKeys.has(edge.source_key) || !visibleKeys.has(edge.target_key)) continue;

      const srcPos = nodePositions.get(edge.source_key);
      const tgtPos = nodePositions.get(edge.target_key);
      if (!srcPos || !tgtPos) continue;

      const srcX = srcPos.x + NODE_WIDTH;
      const srcY = srcPos.y + 30 + edge.source_pin * PIN_SPACING;
      const tgtX = tgtPos.x;
      const tgtY = tgtPos.y + 30 + edge.target_pin * PIN_SPACING;

      ctx.strokeStyle = edge.edge_type === 'variable' ? '#2ecc71'
        : edge.edge_type === 'event' ? '#e74c3c'
        : '#ffffff44';

      ctx.beginPath();
      ctx.moveTo(srcX, srcY);
      // Bezier curve for nice routing
      const midX = (srcX + tgtX) / 2;
      ctx.bezierCurveTo(midX, srcY, midX, tgtY, tgtX, tgtY);
      ctx.stroke();
    }

    // Draw nodes
    for (const node of visibleNodes) {
      const pos = nodePositions.get(node.key);
      if (!pos) continue;

      const color = CATEGORY_COLORS[node.category] ?? CATEGORY_COLORS.other;
      const isSelected = selectedNode?.key === node.key;
      const h = NODE_HEIGHT + Math.max(node.input_pins.length, node.output_pins.length) * PIN_SPACING;

      // Node background
      ctx.fillStyle = isSelected ? '#334155' : '#1e293b';
      ctx.strokeStyle = isSelected ? '#60a5fa' : color;
      ctx.lineWidth = isSelected ? 2 : 1;
      ctx.beginPath();
      ctx.roundRect(pos.x, pos.y, NODE_WIDTH, h, 4);
      ctx.fill();
      ctx.stroke();

      // Title bar
      ctx.fillStyle = color + '44';
      ctx.fillRect(pos.x, pos.y, NODE_WIDTH, 22);

      // Title text
      ctx.fillStyle = '#e2e8f0';
      ctx.font = '10px monospace';
      ctx.textBaseline = 'middle';
      const shortClass = node.class_name.replace(/^(SeqAct_|SeqEvent_|SeqCond_|SeqVar_)/, '');
      ctx.fillText(shortClass, pos.x + 4, pos.y + 11, NODE_WIDTH - 8);

      // Comment or object name
      ctx.fillStyle = '#94a3b8';
      ctx.font = '9px sans-serif';
      const label = node.comment || node.object_name;
      ctx.fillText(label, pos.x + 4, pos.y + 34, NODE_WIDTH - 8);

      // Input pins (left side)
      for (let i = 0; i < node.input_pins.length; i++) {
        const py = pos.y + 44 + i * PIN_SPACING;
        ctx.fillStyle = '#ffffff';
        ctx.beginPath();
        ctx.arc(pos.x, py, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#94a3b8';
        ctx.fillText(node.input_pins[i], pos.x + 6, py, 80);
      }

      // Output pins (right side)
      for (let i = 0; i < node.output_pins.length; i++) {
        const py = pos.y + 44 + i * PIN_SPACING;
        ctx.fillStyle = '#ffffff';
        ctx.beginPath();
        ctx.arc(pos.x + NODE_WIDTH, py, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = '#94a3b8';
        ctx.textAlign = 'right';
        ctx.fillText(node.output_pins[i], pos.x + NODE_WIDTH - 6, py, 80);
        ctx.textAlign = 'left';
      }
    }

    ctx.restore();
  }, [graph, nodePositions, zoom, panX, panY, selectedSequence, selectedNode]);

  // Mouse handlers for pan/zoom/select
  const isDragging = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button === 1 || e.button === 2) {
      isDragging.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    } else if (e.button === 0 && graph) {
      // Click to select node
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const mx = (e.clientX - rect.left - panX) / zoom;
      const my = (e.clientY - rect.top - panY) / zoom;

      const visibleNodes = selectedSequence
        ? graph.nodes.filter(n => n.parent_sequence === selectedSequence)
        : graph.nodes;

      for (const node of visibleNodes) {
        const pos = nodePositions.get(node.key);
        if (!pos) continue;
        if (mx >= pos.x && mx <= pos.x + NODE_WIDTH && my >= pos.y && my <= pos.y + NODE_HEIGHT + 20) {
          setSelectedNode(node);
          return;
        }
      }
      setSelectedNode(null);
    }
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (isDragging.current) {
      setPanX(p => p + e.clientX - lastMouse.current.x);
      setPanY(p => p + e.clientY - lastMouse.current.y);
      lastMouse.current = { x: e.clientX, y: e.clientY };
    }
  };

  const handleMouseUp = () => { isDragging.current = false; };

  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    setZoom(z => Math.max(0.1, Math.min(3, z * (e.deltaY > 0 ? 0.9 : 1.1))));
  };

  if (!visible) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-background">
      {/* Header */}
      <div className="flex items-center gap-2 border-b bg-card px-4 py-2">
        <GitBranch size={16} className="text-primary" />
        <span className="text-sm font-medium text-foreground">Kismet Sequence Viewer</span>
        {graph && (
          <span className="ml-2 text-xs text-muted-foreground">
            {graph.node_count} nodes, {graph.edge_count} edges
          </span>
        )}
        <div className="ml-auto flex items-center gap-2">
          <button onClick={() => setZoom(z => Math.min(3, z * 1.2))} className="p-1 text-muted-foreground hover:text-foreground">
            <ZoomIn size={14} />
          </button>
          <button onClick={() => setZoom(z => Math.max(0.1, z * 0.8))} className="p-1 text-muted-foreground hover:text-foreground">
            <ZoomOut size={14} />
          </button>
          <button onClick={() => { setZoom(0.5); setPanX(0); setPanY(0); }} className="p-1 text-muted-foreground hover:text-foreground">
            <Maximize2 size={14} />
          </button>
          <button onClick={onClose} className="p-1 text-muted-foreground hover:text-foreground">
            <X size={16} />
          </button>
        </div>
      </div>

      <div className="flex flex-1 overflow-hidden">
        {/* Sequence selector sidebar */}
        <div className="w-56 shrink-0 overflow-y-auto border-r bg-card subtle-scrollbar">
          <div className="px-3 py-1.5 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
            Sequences
          </div>
          {graph?.sequences.map(seq => (
            <button
              key={seq.key}
              onClick={() => setSelectedSequence(seq.key)}
              className={`flex w-full items-center gap-1.5 px-3 py-1 text-left text-[11px] ${
                selectedSequence === seq.key
                  ? 'bg-primary/20 text-primary'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'
              }`}
            >
              <GitBranch size={10} className="shrink-0" />
              <span className="truncate">{seq.name}</span>
              <span className="ml-auto shrink-0 text-[9px] text-muted-foreground/60">
                {seq.child_count}
              </span>
            </button>
          ))}
          <button
            onClick={() => setSelectedSequence(null)}
            className={`flex w-full items-center gap-1.5 px-3 py-1 text-left text-[11px] ${
              !selectedSequence
                ? 'bg-primary/20 text-primary'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            }`}
          >
            Show All
          </button>
        </div>

        {/* Graph canvas */}
        <div className="relative flex-1">
          {loading && (
            <div className="absolute inset-0 z-10 flex items-center justify-center bg-background/80">
              <span className="text-sm text-muted-foreground">Extracting Kismet graph...</span>
            </div>
          )}
          {error && (
            <div className="absolute inset-0 z-10 flex items-center justify-center bg-background/80">
              <span className="text-sm text-destructive">{error}</span>
            </div>
          )}
          <canvas
            ref={canvasRef}
            className="h-full w-full"
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            onWheel={handleWheel}
            onContextMenu={e => e.preventDefault()}
          />
        </div>

        {/* Node detail sidebar */}
        {selectedNode && (
          <div className="w-64 shrink-0 overflow-y-auto border-l bg-card subtle-scrollbar">
            <div className="border-b px-3 py-1.5">
              <span className="text-xs font-medium text-foreground">{selectedNode.class_name}</span>
            </div>
            <div className="px-3 py-2 text-[11px]">
              <DetailRow label="Object" value={selectedNode.object_name} />
              <DetailRow label="Category" value={selectedNode.category} />
              <DetailRow label="Tile" value={selectedNode.tile} />
              {selectedNode.comment && <DetailRow label="Comment" value={selectedNode.comment} />}
              <DetailRow label="Inputs" value={selectedNode.input_pins.join(', ') || 'none'} />
              <DetailRow label="Outputs" value={selectedNode.output_pins.join(', ') || 'none'} />
              {selectedNode.variable_pins.length > 0 && (
                <DetailRow label="Variables" value={selectedNode.variable_pins.join(', ')} />
              )}
              {Object.keys(selectedNode.properties).length > 0 && (
                <>
                  <div className="mt-2 text-[9px] font-medium uppercase tracking-wider text-muted-foreground/60">Properties</div>
                  {Object.entries(selectedNode.properties).map(([k, v]) => (
                    <DetailRow key={k} label={k} value={v} />
                  ))}
                </>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex gap-2 py-0.5">
      <span className="w-16 shrink-0 text-muted-foreground">{label}</span>
      <span className="truncate text-foreground">{value}</span>
    </div>
  );
}
