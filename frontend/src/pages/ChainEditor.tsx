import { GitBranchPlus, Play } from 'lucide-solid';
import PageHeader from '../components/PageHeader';
import ReactFlowIsland from '../components/ReactFlowIsland';
import { Button } from '../components/ui/button';

export default function ChainEditor() {
  return (
    <div class="space-y-6">
      <PageHeader
        actions={
          <>
            <Button variant="secondary">
              <GitBranchPlus class="size-4" />
              Fork chain
            </Button>
            <Button>
              <Play class="size-4" />
              Simulate path
            </Button>
          </>
        }
        badge="Content Engine"
        description="Mission logic authoring for content chains, triggers, conditions, actions, counters, and cross-scope quest handoffs."
        eyebrow="Chains"
        title="Mission and event graph tooling"
      />

      <ReactFlowIsland />
    </div>
  );
}
