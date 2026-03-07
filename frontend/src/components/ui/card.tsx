import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';

type DivProps = JSX.HTMLAttributes<HTMLDivElement>;

export function Card(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return (
    <div
      class={cn(
        'glass-panel rounded-[28px] border border-white/8 bg-card/82 text-card-foreground',
        local.class,
      )}
      {...others}
    />
  );
}

export function CardHeader(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return <div class={cn('flex flex-col gap-2 p-6', local.class)} {...others} />;
}

export function CardTitle(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return (
    <div
      class={cn('text-lg font-semibold tracking-tight text-foreground', local.class)}
      {...others}
    />
  );
}

export function CardDescription(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return <div class={cn('text-sm text-muted-foreground', local.class)} {...others} />;
}

export function CardContent(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return <div class={cn('px-6 pb-6', local.class)} {...others} />;
}

export function CardFooter(props: DivProps) {
  const [local, others] = splitProps(props, ['class']);

  return <div class={cn('flex items-center gap-3 px-6 pb-6', local.class)} {...others} />;
}
