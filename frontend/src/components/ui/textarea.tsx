import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';

type TextareaProps = JSX.TextareaHTMLAttributes<HTMLTextAreaElement>;

export function Textarea(props: TextareaProps) {
  const [local, others] = splitProps(props, ['class']);

  return (
    <textarea
      class={cn(
        'flex min-h-28 w-full rounded-[24px] border border-input bg-white/4 px-4 py-3 text-sm text-foreground shadow-inner shadow-black/10 transition-colors placeholder:text-muted-foreground/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/70 disabled:cursor-not-allowed disabled:opacity-50',
        local.class,
      )}
      {...others}
    />
  );
}
