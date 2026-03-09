import type { Edge, Node } from '@xyflow/react';

export type PrimitiveFamily = 'anchor' | 'trigger' | 'condition' | 'action' | 'counter';

export type MissionNodeData = {
  kind: 'mission';
  family: PrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  status: string;
  scenario: string;
  accent: string;
  threadColor?: string;
  threadName?: string;
  manualPosition?: boolean;
  sortOrder?: number;
  inputs?: string[];
  outputs?: string[];
  outputConditions?: string[];
  properties: Array<{ label: string; value: string }>;
};

export type ChainFrameData = {
  kind: 'chain';
  name: string;
  summary: string;
  scopeType: 'mission' | 'space' | 'effect' | 'global';
  scopeId: string;
  enabled: boolean;
  priority: number;
  source: string;
  semantic: string;
  color: string;
  spaceId: string;
  missionId: string | null;
  counts?: Record<PrimitiveFamily, number>;
  sequenceCount?: number;
};

export type EditorNodeData = MissionNodeData | ChainFrameData;

export type ScenarioTemplate = {
  id: string;
  family: PrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  status: string;
  scenario: string;
  accent: string;
  properties: Array<{ label: string; value: string }>;
};

export type SequenceStyle = {
  id: string;
  label: string;
  category: string;
  color: string;
  description: string;
};

export type SequenceEdgeData = {
  sequenceId: string;
  label: string;
  category: string;
  color: string;
};

export type SpaceOption = {
  id: string;
  label: string;
};

export type MissionOption = {
  id: string;
  label: string;
  spaceId: string;
};

export type NodeRecord = Node<EditorNodeData>;

export type EditorSnapshot = {
  edges: Edge<SequenceEdgeData>[];
  nodes: Node<EditorNodeData>[];
};

export type ChainSummary = ChainFrameData & {
  nodeId: string;
  childIds: string[];
  edgeCount: number;
};

export function isMissionData(data: EditorNodeData): data is MissionNodeData {
  return data.kind === 'mission';
}

export function isChainData(data: EditorNodeData): data is ChainFrameData {
  return data.kind === 'chain';
}
