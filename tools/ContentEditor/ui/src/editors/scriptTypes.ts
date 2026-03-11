/** TypeScript types for the Visual Script Editor (Phase 3). */

// Node template from Nodes.xml (loaded from Rust backend)
export type ScriptNodeTemplate = {
  ref_name: string;           // "Var_Bool", "Act_Log", etc.
  display_name: string;       // "Boolean Var", "Log"
  node_type: string;          // "Variable", "Action", "Condition", "Event"
  category: string;
  description: string;
  ports: ScriptPortTemplate[];
  properties: ScriptPropertyTemplate[];
  script_types: string[];     // ["Mission", "Effect"] or empty = all
};

export type ScriptPortTemplate = {
  name: string;
  port_type: string;          // "Boolean", "Integer", "Trigger", etc.
  direction: string;          // "in" or "out"
  default_hide: boolean;
  required: boolean;
};

export type ScriptPropertyTemplate = {
  name: string;
  prop_type: string;
  default_value: string;
  database_ref: string | null;
};

// Enum definition
export type EnumDefinition = {
  name: string;
  data_type: string;
  is_bitfield: boolean;
  tokens: Array<{ name: string; value: string }>;
};

// Script file data (matches Rust DTO)
export type ScriptFileData = {
  version: string;
  dataset_version: string;
  module: string;
  script_type: string;
  next_id: number;
  nodes: ScriptNodeInstance[];
  connections: ScriptConnectionData[];
  comments: ScriptCommentData[];
};

export type ScriptNodeInstance = {
  id: number;
  ref_name: string;
  x: number;
  y: number;
  properties: Array<[string, string]>;
  ports: Array<[string, number]>;
};

export type ScriptConnectionData = {
  out_node: number;
  out_port: string;
  in_node: number;
  in_port: string;
};

export type ScriptCommentData = {
  id: number;
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  color: number;
};

// Script file listing entry
export type ScriptEntry = {
  path: string;
  module: string;
  script_type: string;
  file_name: string;
};

// Compile result
export type CompileResult = {
  output_path: string;
  python_code: string;
  warnings: string[];
};

// Color mapping for node types
export const nodeTypeColors: Record<string, { bg: string; border: string; text: string }> = {
  Variable: { bg: 'rgba(59,130,246,0.14)', border: 'rgba(59,130,246,0.4)', text: '#93c5fd' },
  Action: { bg: 'rgba(34,197,94,0.14)', border: 'rgba(34,197,94,0.4)', text: '#86efac' },
  Condition: { bg: 'rgba(245,170,49,0.14)', border: 'rgba(245,170,49,0.4)', text: '#fcd34d' },
  Event: { bg: 'rgba(168,85,247,0.14)', border: 'rgba(168,85,247,0.4)', text: '#c4b5fd' },
};

// Port type colors for handles
export const portTypeColors: Record<string, string> = {
  Boolean: '#3b82f6',
  Integer: '#22c55e',
  Float: '#f59e0b',
  String: '#ec4899',
  Trigger: '#ef4444',
  Entity: '#8b5cf6',
  Vector3: '#06b6d4',
};

// xyflow node data for script nodes
export type ScriptNodeData = {
  nodeId: number;
  templateRef: string;
  displayName: string;
  nodeType: string;
  category: string;
  description: string;
  inputPorts: Array<{ name: string; portType: string; visible: boolean }>;
  outputPorts: Array<{ name: string; portType: string; visible: boolean }>;
  properties: Record<string, string>;
  enabled: boolean;
  debug: boolean;
  comment: string;
};

// xyflow edge data for script connections
export type ScriptEdgeData = {
  portType: string;
};

// xyflow node data for comment annotations
export type ScriptCommentNodeData = {
  commentId: number;
  text: string;
  color: number;
  width: number;
  height: number;
};
