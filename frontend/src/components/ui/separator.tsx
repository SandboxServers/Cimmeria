import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';

type SeparatorProps = JSX.HTMLAttributes<HTMLDivElement> & {
  orientation?: 'horizontal' | 'vertical';
};

export function Separator(props: SeparatorProps) {
  const [local, others] = splitProps(props, ['class', 'orientation']);

  return (
    <div
      aria-hidden="true"
      class={cn(
        local.orientation === 'vertical' ? 'h-full w-px' : 'h-px w-full',
        'shrink-0 bg-border/70',
        local.class,
      )}
      {...others}
    />
  );
}
