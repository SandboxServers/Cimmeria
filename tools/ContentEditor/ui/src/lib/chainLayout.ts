import { getNodesBounds, type Node } from '@xyflow/react';

type ChainNode = Node<{ kind: string }>;

export type PackedChainSummary = {
  nodeId: string;
};

export type PackedChainLayout = {
  framePosition: { x: number; y: number };
  frameStyle: { width: number; height: number };
};

export type AutoLayoutMissionNode = Node<{
  family?: string;
  kind: string;
  manualPosition?: boolean;
  sortOrder?: number;
}>;

type PackedChainLayoutOptions = {
  cardHeight: number;
  cardWidth: number;
  frameBottomPadding: number;
  frameRightPadding: number;
  minFrameHeight: number;
  minFrameWidth: number;
  topPadding: number;
  verticalGap: number;
};

type ResolveNodePositionsOptions = {
  cardColumnGap: number;
  cardHeight: number;
  cardWidth: number;
  frameLeftPadding: number;
  frameTopPadding: number;
  orderedFamilies: string[];
  rowGap: number;
  rowLabelWidth: number;
};

function getNumericStyleValue(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0;
}

function getActiveFamilies(childNodes: AutoLayoutMissionNode[], orderedFamilies: string[]) {
  const present = new Set(
    childNodes
      .filter((node) => node.data?.kind === 'mission')
      .map((node) => node.data.family)
      .filter(Boolean),
  );

  return orderedFamilies.filter((family) => present.has(family));
}

function getChainChildOrder(childNodes: AutoLayoutMissionNode[], edges: Array<{ source: string; target: string; type?: string }>) {
  const childIds = new Set(childNodes.map((node) => node.id));
  const orderedBySort = [...childNodes].sort((a, b) => {
    const aOrder = a.data?.kind === 'mission' ? (a.data.sortOrder ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
    const bOrder = b.data?.kind === 'mission' ? (b.data.sortOrder ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;

    if (aOrder !== bOrder) {
      return aOrder - bOrder;
    }

    return a.position.x - b.position.x;
  });
  const orderIndex = new Map(orderedBySort.map((node, index) => [node.id, index] as const));
  const incoming = new Map<string, number>();
  const outgoing = new Map<string, string[]>();

  for (const node of childNodes) {
    incoming.set(node.id, 0);
    outgoing.set(node.id, []);
  }

  for (const edge of edges) {
    if (edge.type !== 'sequenceThread' || !childIds.has(edge.source) || !childIds.has(edge.target)) {
      continue;
    }

    outgoing.get(edge.source)?.push(edge.target);
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
  }

  const queue = childNodes
    .filter((node) => (incoming.get(node.id) ?? 0) === 0)
    .sort((a, b) => (orderIndex.get(a.id) ?? 0) - (orderIndex.get(b.id) ?? 0))
    .map((node) => node.id);
  const result: string[] = [];

  while (queue.length) {
    const current = queue.shift();
    if (!current) {
      continue;
    }

    result.push(current);

    const nextIds = [...(outgoing.get(current) ?? [])].sort(
      (a, b) => (orderIndex.get(a) ?? 0) - (orderIndex.get(b) ?? 0),
    );

    for (const nextId of nextIds) {
      const nextIncoming = (incoming.get(nextId) ?? 0) - 1;
      incoming.set(nextId, nextIncoming);
      if (nextIncoming === 0) {
        queue.push(nextId);
      }
    }
  }

  for (const node of childNodes) {
    if (!result.includes(node.id)) {
      result.push(node.id);
    }
  }

  return result;
}

function getLanePosition(
  family: string | undefined,
  order: number,
  activeFamilies: string[],
  options: ResolveNodePositionsOptions,
) {
  const laneIndex = Math.max(activeFamilies.indexOf(family ?? ''), 0);

  return {
    x:
      options.frameLeftPadding +
      options.rowLabelWidth +
      order * (options.cardWidth + options.cardColumnGap),
    y: options.frameTopPadding + laneIndex * (options.cardHeight + options.rowGap),
  };
}

export function resolveAutoLayoutNodePositions(
  chains: PackedChainSummary[],
  nodes: AutoLayoutMissionNode[],
  edges: Array<{ source: string; target: string; type?: string }>,
  options: ResolveNodePositionsOptions,
) {
  const nodesById = new Map(nodes.map((node) => [node.id, node] as const));
  const resolvedNodePositions = new Map<string, { x: number; y: number }>();

  for (const chain of chains) {
    const childNodes = nodes.filter(
      (node) => node.parentId === chain.nodeId && node.data?.kind === 'mission',
    );
    const activeFamilies = getActiveFamilies(childNodes, options.orderedFamilies);
    const orderedChildIds = getChainChildOrder(childNodes, edges);

    orderedChildIds.forEach((nodeId, order) => {
      const childNode = nodesById.get(nodeId);
      if (!childNode || childNode.data?.kind !== 'mission') {
        return;
      }

      if (childNode.data.manualPosition) {
        resolvedNodePositions.set(nodeId, childNode.position);
        return;
      }

      resolvedNodePositions.set(
        nodeId,
        getLanePosition(childNode.data.family, order, activeFamilies, options),
      );
    });
  }

  return resolvedNodePositions;
}

export function computePackedChainLayouts(
  chains: PackedChainSummary[],
  nodes: ChainNode[],
  nodesById: Map<string, ChainNode>,
  options: PackedChainLayoutOptions,
) {
  const layouts = new Map<string, PackedChainLayout>();
  const orderedChains = [...chains].sort((left, right) => {
    const leftFrame = nodesById.get(left.nodeId);
    const rightFrame = nodesById.get(right.nodeId);
    const leftY = leftFrame?.position.y ?? 0;
    const rightY = rightFrame?.position.y ?? 0;

    if (leftY !== rightY) {
      return leftY - rightY;
    }

    return (leftFrame?.position.x ?? 0) - (rightFrame?.position.x ?? 0);
  });

  let nextAvailableY = options.topPadding;

  for (const chain of orderedChains) {
    const frameNode = nodesById.get(chain.nodeId);
    const childNodes = nodes
      .filter((node) => node.parentId === chain.nodeId && node.data?.kind === 'mission')
      .map((node) => ({
        ...node,
        measured: {
          height: node.measured?.height ?? options.cardHeight,
          width: node.measured?.width ?? options.cardWidth,
        },
      }));

    const childBounds = childNodes.length
      ? getNodesBounds(childNodes)
      : {
          height: 0,
          width: 0,
          x: 0,
          y: 0,
        };

    const authoredWidth = getNumericStyleValue(frameNode?.style?.width);
    const authoredHeight = getNumericStyleValue(frameNode?.style?.height);
    const frameWidth = Math.max(
      authoredWidth || options.minFrameWidth,
      childBounds.x + childBounds.width + options.frameRightPadding,
      options.minFrameWidth,
    );
    const frameHeight = Math.max(
      authoredHeight || options.minFrameHeight,
      childBounds.y + childBounds.height + options.frameBottomPadding,
      options.minFrameHeight,
    );
    const frameX = frameNode?.position.x ?? 24;
    const authoredY = frameNode?.position.y ?? options.topPadding;
    const frameY = Math.max(authoredY, nextAvailableY);

    layouts.set(chain.nodeId, {
      framePosition: { x: frameX, y: frameY },
      frameStyle: { width: frameWidth, height: frameHeight },
    });

    nextAvailableY = frameY + frameHeight + options.verticalGap;
  }

  return layouts;
}
