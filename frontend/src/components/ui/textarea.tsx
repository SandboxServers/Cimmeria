import type { TextareaHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement>;

export function Textarea({ className, ...rest }: TextareaProps) {
  return (
    <textarea
      className={cn(
        'flex min-h-28 w-full border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-4 py-3 text-sm text-on-surface shadow-inner shadow-black/10 transition-colors placeholder:text-on-surface-variant/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 disabled:cursor-not-allowed disabled:opacity-50',
        className,
      )}
      {...rest}
    />
  );
}
