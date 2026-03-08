import type { Edge, Node } from '@xyflow/react';
import { getConnectionPortLabels, validateConnectionRequest } from './chainConnections';

export type ValidationPrimitiveFamily = 'anchor' | 'trigger' | 'condition' | 'action' | 'counter';

export type ValidationMissionNodeData = {
  kind: 'mission';
  family: ValidationPrimitiveFamily;
  stage: string;
  title: string;
  detail: string;
  scenario: string;
  inputs?: string[];
  outputs?: string[];
  outputConditions?: string[];
};

export type ValidationChainFrameData = {
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
};

export type ValidationNodeData = ValidationMissionNodeData | ValidationChainFrameData;

export type ValidationChainSummary = ValidationChainFrameData & {
  nodeId: string;
  childIds: string[];
  edgeCount: number;
  sequenceCount?: number;
};

export type ValidationSeverity = 'error' | 'warning';

export type ValidationIssue = {
  id: string;
  severity: ValidationSeverity;
  scope: 'chain' | 'node' | 'edge' | 'sequence';
  title: string;
  message: string;
  chainId?: string;
  chainName?: string;
  nodeId?: string;
  nodeTitle?: string;
  sequenceId?: string;
};

export type ValidationChainStatus = {
  chainId: string;
  chainName: string;
  errorCount: number;
  warningCount: number;
};

export type ValidationReport = {
  issues: ValidationIssue[];
  errorCount: number;
  warningCount: number;
  chainStatuses: ValidationChainStatus[];
};

type ValidationNodeRecord = Node<ValidationNodeData>;
type ValidationEdgeRecord = Edge<{
  sequenceId?: string;
  label?: string;
  category?: string;
  color?: string;
}>;

function isMissionData(data: ValidationNodeData): data is ValidationMissionNodeData {
  return data.kind === 'mission';
}

function isChainData(data: ValidationNodeData): data is ValidationChainFrameData {
  return data.kind === 'chain';
}

function hasText(value: unknown) {
  return Boolean(stringifyValidationText(value).trim());
}

function countBySeverity(issues: ValidationIssue[], severity: ValidationSeverity) {
  return issues.filter((issue) => issue.severity === severity).length;
}

function createIssue(issue: ValidationIssue): ValidationIssue {
  return {
    ...issue,
    title: stringifyValidationText(issue.title),
    message: stringifyValidationText(issue.message),
    chainName: issue.chainName ? stringifyValidationText(issue.chainName) : issue.chainName,
    nodeTitle: issue.nodeTitle ? stringifyValidationText(issue.nodeTitle) : issue.nodeTitle,
    sequenceId: issue.sequenceId ? stringifyValidationText(issue.sequenceId) : issue.sequenceId,
  };
}

function stringifyValidationText(value: unknown): string {
  if (typeof value === 'string') {
    return value;
  }

  if (value === null || value === undefined) {
    return '';
  }

  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }

  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

export function buildValidationReport(args: {
  chains: ValidationChainSummary[];
  edges: ValidationEdgeRecord[];
  nodes: ValidationNodeRecord[];
}): ValidationReport {
  const issues: ValidationIssue[] = [];
  const nodeMap = new Map(args.nodes.map((node) => [node.id, node] as const));
  const incomingEdgesByNode = new Map<string, ValidationEdgeRecord[]>();
  const outgoingEdgesByNode = new Map<string, ValidationEdgeRecord[]>();

  for (const edge of args.edges) {
    if (!incomingEdgesByNode.has(edge.target)) {
      incomingEdgesByNode.set(edge.target, []);
    }
    if (!outgoingEdgesByNode.has(edge.source)) {
      outgoingEdgesByNode.set(edge.source, []);
    }
    incomingEdgesByNode.get(edge.target)?.push(edge);
    outgoingEdgesByNode.get(edge.source)?.push(edge);

    const sourceNode = nodeMap.get(edge.source);
    const targetNode = nodeMap.get(edge.target);

    if (!sourceNode || !targetNode) {
      issues.push(
        createIssue({
          id: `edge-missing-${edge.id}`,
          severity: 'error',
          scope: 'edge',
          title: 'Broken link endpoint',
          message: 'This link points at a card that is no longer visible in the current scope.',
        }),
      );
      continue;
    }

    if (!isMissionData(sourceNode.data) || !isMissionData(targetNode.data)) {
      issues.push(
        createIssue({
          id: `edge-endpoint-type-${edge.id}`,
          severity: 'error',
          scope: 'edge',
          title: 'Invalid link type',
          message: 'Links must connect mission cards, not chain frames.',
          chainId: sourceNode.parentId ?? targetNode.parentId,
        }),
      );
      continue;
    }

    try {
      validateConnectionRequest({
        sourceHandle: edge.sourceHandle,
        sourceId: edge.source,
        sourceNode,
        targetHandle: edge.targetHandle,
        targetId: edge.target,
        targetNode,
      });
    } catch (error) {
      issues.push(
        createIssue({
          id: `edge-invalid-${edge.id}`,
          severity: 'error',
          scope: 'edge',
          title: 'Invalid port wiring',
          message: error instanceof Error ? error.message : 'This link points at a missing input or output.',
          chainId: sourceNode.parentId ?? targetNode.parentId,
          nodeId: sourceNode.id,
          nodeTitle: sourceNode.data.title,
        }),
      );
    }

    if (edge.type === 'sequenceThread' && !hasText(edge.data?.label)) {
      issues.push(
        createIssue({
          id: `sequence-label-${edge.id}`,
          severity: 'warning',
          scope: 'sequence',
          title: 'Sequence yarn has no visible label',
          message: 'Name this yarn so designers can read the mission beat on the canvas.',
          chainId: sourceNode.parentId ?? targetNode.parentId,
          sequenceId: edge.data?.sequenceId,
        }),
      );
    }
  }

  for (const chain of args.chains) {
    const childNodes = args.nodes.filter(
      (node) => node.parentId === chain.nodeId && isMissionData(node.data),
    );
    const startAnchors = childNodes.filter(
      (node) => node.data.family === 'anchor' && node.data.stage.toLowerCase() === 'start',
    );
    const endAnchors = childNodes.filter(
      (node) => node.data.family === 'anchor' && node.data.stage.toLowerCase() === 'end',
    );

    if (!hasText(chain.name)) {
      issues.push(
        createIssue({
          id: `chain-name-${chain.nodeId}`,
          severity: 'error',
          scope: 'chain',
          title: 'Chain name is missing',
          message: 'Give this content chain a readable name so it can be filtered and reviewed.',
          chainId: chain.nodeId,
          chainName: chain.name,
        }),
      );
    }

    if (!hasText(chain.summary)) {
      issues.push(
        createIssue({
          id: `chain-summary-${chain.nodeId}`,
          severity: 'warning',
          scope: 'chain',
          title: 'Chain summary is empty',
          message: 'Add a short summary so no-code designers know what this chain controls.',
          chainId: chain.nodeId,
          chainName: chain.name,
        }),
      );
    }

    if (childNodes.length === 0) {
      issues.push(
        createIssue({
          id: `chain-empty-${chain.nodeId}`,
          severity: 'error',
          scope: 'chain',
          title: 'Chain has no cards',
          message: 'Add at least one sequence anchor or gameplay card to make this chain authorable.',
          chainId: chain.nodeId,
          chainName: chain.name,
        }),
      );
    }

    if (startAnchors.length === 0) {
      issues.push(
        createIssue({
          id: `chain-start-${chain.nodeId}`,
          severity: 'error',
          scope: 'chain',
          title: 'Missing sequence start',
          message: 'Every content chain should expose a Sequence Start so the playable beat is obvious.',
          chainId: chain.nodeId,
          chainName: chain.name,
        }),
      );
    }

    if (endAnchors.length === 0) {
      issues.push(
        createIssue({
          id: `chain-end-${chain.nodeId}`,
          severity: 'warning',
          scope: 'chain',
          title: 'Missing sequence end',
          message: 'Add a Sequence End so the branch resolves cleanly for designers and reviewers.',
          chainId: chain.nodeId,
          chainName: chain.name,
        }),
      );
    }

    for (const node of childNodes) {
      const ports = getConnectionPortLabels(node);
      const incoming = incomingEdgesByNode.get(node.id) ?? [];
      const outgoing = outgoingEdgesByNode.get(node.id) ?? [];
      const conditions = node.data.outputConditions ?? [];
      const nodeLabel = node.data.title || familyLabel(node.data.family);

      if (!hasText(node.data.title)) {
        issues.push(
          createIssue({
            id: `node-title-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Card title is missing',
            message: 'Name this card so it can be searched, validated, and reviewed.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (!hasText(node.data.scenario)) {
        issues.push(
          createIssue({
            id: `node-scenario-${node.id}`,
            severity: 'warning',
            scope: 'node',
            title: 'Scenario label is empty',
            message: 'Add a scenario label so designers can tell what gameplay system this card belongs to.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (!hasText(node.data.detail)) {
        issues.push(
          createIssue({
            id: `node-detail-${node.id}`,
            severity: 'warning',
            scope: 'node',
            title: 'Card detail is empty',
            message: 'Add a short detail so the runtime intent is readable without opening the backend data.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (ports.inputs.some((label) => !hasText(label))) {
        issues.push(
          createIssue({
            id: `node-input-label-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Input label is blank',
            message: 'Every input port needs a readable label before other cards can wire into it safely.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (ports.outputs.some((label) => !hasText(label))) {
        issues.push(
          createIssue({
            id: `node-output-label-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Output label is blank',
            message: 'Every output port needs a readable label before branch yarns can be reviewed clearly.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (conditions.length !== ports.outputs.length) {
        issues.push(
          createIssue({
            id: `node-branch-count-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Branch rule count is out of sync',
            message: 'The number of branch rules must match the number of output ports.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (ports.outputs.length > 1 && conditions.some((condition) => !hasText(condition))) {
        issues.push(
          createIssue({
            id: `node-branch-rule-${node.id}`,
            severity: 'warning',
            scope: 'node',
            title: 'Branch rule is incomplete',
            message: 'Multi-output cards should describe what decides each branch.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      const isStartAnchor =
        node.data.family === 'anchor' && node.data.stage.toLowerCase() === 'start';
      const isEndAnchor =
        node.data.family === 'anchor' && node.data.stage.toLowerCase() === 'end';

      if (isStartAnchor && outgoing.length === 0) {
        issues.push(
          createIssue({
            id: `node-start-outgoing-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Sequence start does not lead anywhere',
            message: 'Connect this Sequence Start to the first playable card in the chain.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (isEndAnchor && incoming.length === 0) {
        issues.push(
          createIssue({
            id: `node-end-incoming-${node.id}`,
            severity: 'error',
            scope: 'node',
            title: 'Sequence end is never reached',
            message: 'Connect an upstream card into this Sequence End so the branch actually resolves.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (!isStartAnchor && incoming.length === 0) {
        issues.push(
          createIssue({
            id: `node-unreachable-${node.id}`,
            severity: 'warning',
            scope: 'node',
            title: 'Card is unreachable',
            message: 'This card has no incoming links in the current scope.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }

      if (!isEndAnchor && outgoing.length === 0) {
        issues.push(
          createIssue({
            id: `node-unresolved-${node.id}`,
            severity: 'warning',
            scope: 'node',
            title: 'Card has no outgoing path',
            message: 'Connect this card forward or make it an explicit Sequence End.',
            chainId: chain.nodeId,
            chainName: chain.name,
            nodeId: node.id,
            nodeTitle: nodeLabel,
          }),
        );
      }
    }
  }

  return {
    issues,
    errorCount: countBySeverity(issues, 'error'),
    warningCount: countBySeverity(issues, 'warning'),
    chainStatuses: args.chains.map((chain) => ({
      chainId: chain.nodeId,
      chainName: stringifyValidationText(chain.name),
      errorCount: issues.filter(
        (issue) => issue.chainId === chain.nodeId && issue.severity === 'error',
      ).length,
      warningCount: issues.filter(
        (issue) => issue.chainId === chain.nodeId && issue.severity === 'warning',
      ).length,
    })),
  };
}

function familyLabel(family: ValidationPrimitiveFamily) {
  return (
    {
      anchor: 'Sequence card',
      trigger: 'Trigger card',
      condition: 'Condition card',
      action: 'Action card',
      counter: 'Counter card',
    } satisfies Record<ValidationPrimitiveFamily, string>
  )[family];
}
