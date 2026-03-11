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
      className={cn('h-2.5 w-full overflow-hidden rounded-full bg-white/8', className)}
      {...rest}
    >
      <div
        className="h-full rounded-full bg-gradient-to-r from-primary via-amber-300 to-accent transition-all duration-300"
        style={{ width: `${safeValue}%` }}
      />
    </div>
  );
}
