import type { InputHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

type InputProps = InputHTMLAttributes<HTMLInputElement>;

export function Input({ className, ...rest }: InputProps) {
  return (
    <input
      className={cn(
        'flex h-11 w-full border border-[rgba(77,70,53,0.15)] bg-surface-lowest px-4 py-2 text-sm text-on-surface shadow-inner shadow-black/10 transition-colors placeholder:text-on-surface-variant/70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 disabled:cursor-not-allowed disabled:opacity-50',
        className,
      )}
      {...rest}
    />
  );
}
