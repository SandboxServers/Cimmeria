import type { HTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

type DivProps = HTMLAttributes<HTMLDivElement>;

export function Card({ className, ...rest }: DivProps) {
  return (
    <div
      className={cn(
        'border border-[rgba(77,70,53,0.15)] bg-surface-low text-card-foreground',
        className,
      )}
      {...rest}
    />
  );
}

export function CardHeader({ className, ...rest }: DivProps) {
  return <div className={cn('flex flex-col gap-2 p-6', className)} {...rest} />;
}

export function CardTitle({ className, ...rest }: DivProps) {
  return (
    <div
      className={cn('font-headline text-lg font-semibold uppercase tracking-widest text-on-surface', className)}
      {...rest}
    />
  );
}

export function CardDescription({ className, ...rest }: DivProps) {
  return <div className={cn('text-sm text-on-surface-variant', className)} {...rest} />;
}

export function CardContent({ className, ...rest }: DivProps) {
  return <div className={cn('px-6 pb-6', className)} {...rest} />;
}

export function CardFooter({ className, ...rest }: DivProps) {
  return <div className={cn('flex items-center gap-3 px-6 pb-6', className)} {...rest} />;
}
