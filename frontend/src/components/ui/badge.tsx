import type { HTMLAttributes } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const badgeVariants = cva(
  'inline-flex items-center gap-1 border px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]',
  {
    variants: {
      variant: {
        default: 'border-primary/30 bg-primary/14 text-primary',
        secondary: 'border-[rgba(77,70,53,0.15)] bg-surface-high text-on-surface-variant',
        outline: 'border-[rgba(77,70,53,0.15)] bg-transparent text-on-surface-variant',
        success: 'border-tertiary/30 bg-tertiary/12 text-tertiary',
        destructive: 'border-error/30 bg-error/15 text-error',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  },
);

type BadgeProps = HTMLAttributes<HTMLDivElement> & VariantProps<typeof badgeVariants>;

export function Badge({ className, variant, ...rest }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...rest} />;
}
