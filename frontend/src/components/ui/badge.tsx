import type { HTMLAttributes } from 'react';
import { cva, type VariantProps } from 'class-variance-authority';
import { cn } from '../../lib/utils';

const badgeVariants = cva(
  'inline-flex items-center gap-1 rounded-full border px-2.5 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]',
  {
    variants: {
      variant: {
        default: 'border-primary/30 bg-primary/14 text-primary',
        secondary: 'border-secondary/50 bg-secondary/25 text-secondary-foreground',
        outline: 'border-border bg-transparent text-muted-foreground',
        success: 'border-emerald-400/30 bg-emerald-400/12 text-emerald-200',
        destructive: 'border-destructive/30 bg-destructive/15 text-destructive-foreground',
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
