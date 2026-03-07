import type { JSX } from 'solid-js';
import { Badge } from './ui/badge';
import { cn } from '../lib/utils';

type PageHeaderProps = {
  eyebrow: string;
  title: string;
  description: string;
  badge?: string;
  actions?: JSX.Element;
  class?: string;
};

export default function PageHeader(props: PageHeaderProps) {
  return (
    <div
      class={cn(
        'flex flex-col gap-5 rounded-[32px] border border-white/8 bg-gradient-to-br from-white/9 via-white/[0.04] to-transparent p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)] lg:flex-row lg:items-end lg:justify-between',
        props.class,
      )}
    >
      <div class="space-y-4">
        <div class="flex flex-wrap items-center gap-3">
          <Badge variant="secondary">{props.eyebrow}</Badge>
          {props.badge && <Badge variant="outline">{props.badge}</Badge>}
        </div>
        <div class="space-y-2">
          <h1 class="text-3xl font-semibold tracking-[-0.04em] text-foreground sm:text-4xl">
            {props.title}
          </h1>
          <p class="max-w-3xl text-sm leading-7 text-muted-foreground sm:text-base">
            {props.description}
          </p>
        </div>
      </div>
      {props.actions && <div class="flex flex-wrap items-center gap-3">{props.actions}</div>}
    </div>
  );
}
