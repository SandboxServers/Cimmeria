import type { HTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

type SeparatorProps = HTMLAttributes<HTMLDivElement> & {
  orientation?: 'horizontal' | 'vertical';
};

export function Separator({ className, orientation, ...rest }: SeparatorProps) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        orientation === 'vertical' ? 'h-full w-px' : 'h-px w-full',
        'shrink-0 bg-border/70',
        className,
      )}
      {...rest}
    />
  );
}
