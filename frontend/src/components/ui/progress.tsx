import type { HTMLAttributes } from 'react';
import { cn } from '../../lib/utils';
import { clampProgressValue } from '../../lib/view-models';

type ProgressProps = HTMLAttributes<HTMLDivElement> & {
  value: number;
};

export function Progress({ className, value, ...rest }: ProgressProps) {
  const safeValue = clampProgressValue(value);

  return (
    <div
      className={cn('h-2.5 w-full overflow-hidden bg-surface-high', className)}
      {...rest}
    >
      <div
        className="h-full bg-gradient-to-r from-primary via-primary/70 to-tertiary transition-all duration-300"
        style={{ width: `${safeValue}%` }}
      />
    </div>
  );
}
