import type { HTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

type DivProps = HTMLAttributes<HTMLDivElement>;

export function Card({ className, ...rest }: DivProps) {
  return (
    <div
      className={cn(
        'glass-panel rounded-[28px] border border-white/8 bg-card/82 text-card-foreground',
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
      className={cn('text-lg font-semibold tracking-tight text-foreground', className)}
      {...rest}
    />
  );
}

export function CardDescription({ className, ...rest }: DivProps) {
  return <div className={cn('text-sm text-muted-foreground', className)} {...rest} />;
}

export function CardContent({ className, ...rest }: DivProps) {
  return <div className={cn('px-6 pb-6', className)} {...rest} />;
}

export function CardFooter({ className, ...rest }: DivProps) {
  return <div className={cn('flex items-center gap-3 px-6 pb-6', className)} {...rest} />;
}
