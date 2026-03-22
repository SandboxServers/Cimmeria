import { forwardRef, type HTMLAttributes, type TableHTMLAttributes, type TdHTMLAttributes, type ThHTMLAttributes } from 'react';
import { cn } from '../../lib/utils';

export const Table = forwardRef<HTMLTableElement, TableHTMLAttributes<HTMLTableElement>>(
  ({ className, ...rest }, ref) => (
    <table ref={ref} className={cn('w-full caption-bottom text-sm', className)} {...rest} />
  ),
);

export const TableHeader = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...rest }, ref) => (
    <thead ref={ref} className={cn('[&_tr]:border-b [&_tr]:border-[rgba(77,70,53,0.15)]', className)} {...rest} />
  ),
);

export const TableBody = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...rest }, ref) => (
    <tbody ref={ref} className={cn('[&_tr:last-child]:border-0', className)} {...rest} />
  ),
);

export const TableRow = forwardRef<HTMLTableRowElement, HTMLAttributes<HTMLTableRowElement>>(
  ({ className, ...rest }, ref) => (
    <tr ref={ref} className={cn('border-b border-[rgba(77,70,53,0.15)] transition-colors hover:bg-surface-high', className)} {...rest} />
  ),
);

export const TableHead = forwardRef<HTMLTableCellElement, ThHTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...rest }, ref) => (
    <th ref={ref} className={cn('h-12 px-4 text-left align-middle text-xs font-semibold uppercase tracking-[0.24em] text-on-surface-variant', className)} {...rest} />
  ),
);

export const TableCell = forwardRef<HTMLTableCellElement, TdHTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...rest }, ref) => (
    <td ref={ref} className={cn('px-4 py-4 align-middle', className)} {...rest} />
  ),
);
