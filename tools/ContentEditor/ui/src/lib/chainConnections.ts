export type LinkValidationNode = {
  data: {
    family: string;
    stage: string;
    title: string;
    inputs?: string[];
    outputConditions?: string[];
    outputs?: string[];
  };
};

export function getConnectionPortLabels(node: LinkValidationNode) {
  const fallbackInputs = ['In'];
  const fallbackOutputs =
    node.data.family === 'anchor' && node.data.stage === 'End' ? [] : ['Out'];

  return {
    inputs: node.data.inputs ?? fallbackInputs,
    outputs: node.data.outputs ?? fallbackOutputs,
  };
}

export function getNodeHandleSignature(node: LinkValidationNode) {
  const { inputs, outputs } = getConnectionPortLabels(node);
  return JSON.stringify({
    inputs,
    outputs,
  });
}

export function validateConnectionRequest(args: {
  sourceHandle?: string | null;
  sourceId?: string | null;
  sourceNode?: LinkValidationNode;
  targetHandle?: string | null;
  targetId?: string | null;
  targetNode?: LinkValidationNode;
}) {
  const { sourceHandle, sourceId, sourceNode, targetHandle, targetId, targetNode } = args;

  if (!sourceId || !targetId) {
    throw new Error('missing a source or target handle');
  }

  if (!sourceNode || !targetNode) {
    throw new Error('both endpoints must be mission cards');
  }

  if (sourceId === targetId) {
    throw new Error('a card cannot connect to itself');
  }

  const sourcePorts = getConnectionPortLabels(sourceNode);
  const targetPorts = getConnectionPortLabels(targetNode);
  const sourceHandleIndex = Number((sourceHandle ?? 'output-0').replace('output-', ''));
  const targetHandleIndex = Number((targetHandle ?? 'input-0').replace('input-', ''));

  if (
    Number.isNaN(sourceHandleIndex) ||
    sourceHandleIndex < 0 ||
    sourceHandleIndex >= sourcePorts.outputs.length
  ) {
    throw new Error(`invalid output on ${sourceNode.data.title}`);
  }

  if (
    Number.isNaN(targetHandleIndex) ||
    targetHandleIndex < 0 ||
    targetHandleIndex >= targetPorts.inputs.length
  ) {
    throw new Error(`invalid input on ${targetNode.data.title}`);
  }
}
