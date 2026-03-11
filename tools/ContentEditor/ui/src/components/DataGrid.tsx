import { useState, useMemo, useCallback } from 'react';
import { Search } from 'lucide-react';

interface Column<T> {
  key: keyof T;
  label: string;
  width?: string;
}

interface DataGridProps<T> {
  data: T[];
  columns: Column<T>[];
  onSelect: (item: T) => void;
  selectedId: number | null;
  idKey: keyof T;
  searchValue: string;
  onSearchChange: (value: string) => void;
  searchPlaceholder?: string;
}

type SortDir = 'asc' | 'desc';

export default function DataGrid<T>({
  data,
  columns,
  onSelect,
  selectedId,
  idKey,
  searchValue,
  onSearchChange,
  searchPlaceholder = 'Search...',
}: DataGridProps<T>) {
  const [sortKey, setSortKey] = useState<keyof T | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>('asc');

  const handleSort = useCallback(
    (key: keyof T) => {
      if (sortKey === key) {
        setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
      } else {
        setSortKey(key);
        setSortDir('asc');
      }
    },
    [sortKey],
  );

  const filtered = useMemo(() => {
    if (!searchValue.trim()) return data;
    const q = searchValue.toLowerCase();
    return data.filter((row) =>
      columns.some((col) => {
        const val = row[col.key];
        if (val == null) return false;
        return String(val).toLowerCase().includes(q);
      }),
    );
  }, [data, searchValue, columns]);

  const sorted = useMemo(() => {
    if (!sortKey) return filtered;
    const copy = [...filtered];
    copy.sort((a, b) => {
      const av = a[sortKey];
      const bv = b[sortKey];
      if (av == null && bv == null) return 0;
      if (av == null) return 1;
      if (bv == null) return -1;
      if (typeof av === 'number' && typeof bv === 'number') {
        return sortDir === 'asc' ? av - bv : bv - av;
      }
      const sa = String(av);
      const sb = String(bv);
      const cmp = sa.localeCompare(sb);
      return sortDir === 'asc' ? cmp : -cmp;
    });
    return copy;
  }, [filtered, sortKey, sortDir]);

  return (
    <div className="flex h-full flex-col">
      {/* Search bar */}
      <div className="border-b border-border px-2 py-1.5">
        <div className="relative">
          <Search className="pointer-events-none absolute left-2 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
          <input
            type="text"
            value={searchValue}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder={searchPlaceholder}
            className="w-full rounded border border-border bg-input py-1 pl-7 pr-2 text-xs text-foreground placeholder:text-muted-foreground focus:border-primary focus:outline-none"
          />
        </div>
      </div>

      {/* Table */}
      <div className="subtle-scrollbar flex-1 overflow-auto">
        <table className="w-full text-xs">
          <thead className="sticky top-0 z-10 bg-card">
            <tr>
              {columns.map((col) => (
                <th
                  key={String(col.key)}
                  className="cursor-pointer select-none border-b border-border px-2 py-1.5 text-left font-medium text-muted-foreground hover:text-foreground"
                  style={col.width ? { width: col.width } : undefined}
                  onClick={() => handleSort(col.key)}
                >
                  <span className="inline-flex items-center gap-1">
                    {col.label}
                    {sortKey === col.key && (
                      <span className="text-[10px]">
                        {sortDir === 'asc' ? '\u25B2' : '\u25BC'}
                      </span>
                    )}
                  </span>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sorted.map((row) => {
              const rowId = row[idKey] as unknown as number;
              const isSelected = rowId === selectedId;
              return (
                <tr
                  key={rowId}
                  className={`cursor-pointer border-b border-border/50 transition-colors ${
                    isSelected
                      ? 'bg-primary/15 text-foreground'
                      : 'text-muted-foreground hover:bg-secondary/30'
                  }`}
                  onClick={() => onSelect(row)}
                >
                  {columns.map((col) => (
                    <td
                      key={String(col.key)}
                      className="truncate px-2 py-1"
                      style={col.width ? { maxWidth: col.width } : undefined}
                    >
                      {formatCell(row[col.key])}
                    </td>
                  ))}
                </tr>
              );
            })}
            {sorted.length === 0 && (
              <tr>
                <td
                  colSpan={columns.length}
                  className="px-4 py-6 text-center text-muted-foreground"
                >
                  {data.length === 0 ? 'No data loaded' : 'No matching records'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {/* Footer count */}
      <div className="border-t border-border px-2 py-1 text-[10px] text-muted-foreground">
        {sorted.length} / {data.length} records
      </div>
    </div>
  );
}

function formatCell(value: unknown): string {
  if (value == null) return '\u2014';
  if (typeof value === 'boolean') return value ? 'Yes' : 'No';
  return String(value);
}
