import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';

type InputProps = JSX.InputHTMLAttributes<HTMLInputElement>;

export function Input(props: InputProps) {
  const [local, others] = splitProps(props, ['class']);

  return (
    <input
      class={cn(
        'flex h-11 w-full rounded-2xl border border-input bg-white/4 px-4 py-2 text-sm text-foreground shadow-inner shadow-black/10 transition-colors placeholder:text-muted-foreground/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/70 disabled:cursor-not-allowed disabled:opacity-50',
        local.class,
      )}
      {...others}
    />
  );
}
