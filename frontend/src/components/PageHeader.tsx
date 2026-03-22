import type { ReactNode } from 'react';
import { Badge } from './ui/badge';
import { cn } from '../lib/utils';

type PageHeaderProps = {
  eyebrow: string;
  title: string;
  description: string;
  badge?: ReactNode;
  actions?: ReactNode;
  className?: string;
};

export default function PageHeader(props: PageHeaderProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-5 obsidian-panel border border-[rgba(77,70,53,0.15)] p-6 shadow-[0_24px_80px_rgba(0,0,0,0.22)] lg:flex-row lg:items-end lg:justify-between',
        props.className,
      )}
    >
      <div className="space-y-4">
        <div className="flex flex-wrap items-center gap-3">
          <Badge variant="secondary">{props.eyebrow}</Badge>
          {props.badge && <Badge variant="outline">{props.badge}</Badge>}
        </div>
        <div className="space-y-2">
          <h1 className="text-3xl font-headline uppercase tracking-widest text-on-surface sm:text-4xl">
            {props.title}
          </h1>
          <p className="max-w-3xl text-sm leading-7 text-on-surface-variant sm:text-base">
            {props.description}
          </p>
        </div>
      </div>
      {props.actions && <div className="flex flex-wrap items-center gap-3">{props.actions}</div>}
    </div>
  );
}
