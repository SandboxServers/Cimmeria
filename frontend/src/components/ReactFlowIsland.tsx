import { onCleanup, onMount } from 'solid-js';
import { createRoot, type Root } from 'react-dom/client';
import React from 'react';
import ChainFlowWorkbench from '../react/ChainFlowWorkbench.react';

export default function ReactFlowIsland() {
  let container!: HTMLDivElement;
  let root: Root | undefined;

  onMount(() => {
    root = createRoot(container);
    root.render(React.createElement(ChainFlowWorkbench));
  });

  onCleanup(() => {
    root?.unmount();
  });

  return <div class="min-h-[760px]" ref={container} />;
}
