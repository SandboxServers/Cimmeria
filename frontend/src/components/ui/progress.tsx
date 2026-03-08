import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';
import { clampProgressValue } from '../../lib/view-models';

type ProgressProps = JSX.HTMLAttributes<HTMLDivElement> & {
  value: number;
};

export function Progress(props: ProgressProps) {
  const [local, others] = splitProps(props, ['class', 'value']);
  const safeValue = () => clampProgressValue(local.value);

  return (
    <div
      class={cn('h-2.5 w-full overflow-hidden rounded-full bg-white/8', local.class)}
      {...others}
    >
      <div
        class="h-full rounded-full bg-gradient-to-r from-primary via-amber-300 to-accent transition-all duration-300"
        style={{ width: `${safeValue()}%` }}
      />
    </div>
  );
}
