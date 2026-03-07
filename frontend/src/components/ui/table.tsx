import { type JSX, splitProps } from 'solid-js';
import { cn } from '../../lib/utils';

type TableProps<T extends keyof JSX.IntrinsicElements> = JSX.IntrinsicElements[T];

function withClass<T extends keyof JSX.IntrinsicElements>(
  tag: T,
  baseClass: string,
) {
  return (props: TableProps<T>) => {
    const [local, others] = splitProps(props as { class?: string }, ['class']);

    return tag === 'table' ? (
      <table class={cn(baseClass, local.class)} {...(others as JSX.TableHTMLAttributes<HTMLTableElement>)} />
    ) : tag === 'thead' ? (
      <thead class={cn(baseClass, local.class)} {...(others as JSX.HTMLAttributes<HTMLTableSectionElement>)} />
    ) : tag === 'tbody' ? (
      <tbody class={cn(baseClass, local.class)} {...(others as JSX.HTMLAttributes<HTMLTableSectionElement>)} />
    ) : tag === 'tr' ? (
      <tr class={cn(baseClass, local.class)} {...(others as JSX.HTMLAttributes<HTMLTableRowElement>)} />
    ) : tag === 'th' ? (
      <th class={cn(baseClass, local.class)} {...(others as JSX.ThHTMLAttributes<HTMLTableCellElement>)} />
    ) : (
      <td class={cn(baseClass, local.class)} {...(others as JSX.TdHTMLAttributes<HTMLTableCellElement>)} />
    );
  };
}

export const Table = withClass('table', 'w-full caption-bottom text-sm');
export const TableHeader = withClass('thead', '[&_tr]:border-b [&_tr]:border-white/6');
export const TableBody = withClass('tbody', '[&_tr:last-child]:border-0');
export const TableRow = withClass(
  'tr',
  'border-b border-white/6 transition-colors hover:bg-white/[0.03]',
);
export const TableHead = withClass(
  'th',
  'h-12 px-4 text-left align-middle text-xs font-semibold uppercase tracking-[0.24em] text-muted-foreground',
);
export const TableCell = withClass('td', 'px-4 py-4 align-middle');
