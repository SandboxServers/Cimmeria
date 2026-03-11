import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Circle, Pause, Play, Trash2 } from 'lucide-react';
import PageHeader from '../components/PageHeader';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../components/ui/card';
import { Input } from '../components/ui/input';
import { Tabs, TabsList, TabsTrigger } from '../components/ui/tabs';
import { connectWs } from '../lib/ws';

type LogEntry = {
  timestamp_ms: number;
  level: string;
  target: string;
  message: string;
  fields: Record<string, unknown>;
  /** Sent when the client falls behind the broadcast buffer. */
  type?: string;
  skipped?: number;
};

type LogSource = 'server' | 'supervisor';

const SOURCE_CONFIG: Record<LogSource, { label: string; wsPath: string; description: string }> = {
  server: {
    label: 'Game Server',
    wsPath: '/ws/logs',
    description: 'Game server process logs (auth, base, cell, database).',
  },
  supervisor: {
    label: 'Supervisor',
    wsPath: '/supervisor/ws/logs',
    description: 'Supervisor process logs (start, stop, rebuild, health proxy).',
  },
};

const MAX_ENTRIES = 2000;

const LEVEL_COLORS: Record<string, string> = {
  ERROR: 'bg-red-400/15 text-red-200',
  WARN: 'bg-amber-300/15 text-amber-100',
  INFO: 'bg-emerald-400/12 text-emerald-100',
  DEBUG: 'bg-sky-400/12 text-sky-200',
  TRACE: 'bg-slate-400/12 text-slate-300',
};

function formatTime(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function formatFields(fields: Record<string, unknown>): string {
  const entries = Object.entries(fields);
  if (entries.length === 0) return '';
  return entries.map(([k, v]) => `${k}=${v}`).join(' ');
}

export default function Logs() {
  const [source, setSource] = useState<LogSource>('server');
  const [level, setLevel] = useState('ALL');
  const [query, setQuery] = useState('');
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [paused, setPaused] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const autoScroll = useRef(true);

  // Track whether user has scrolled up
  const handleScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
    autoScroll.current = atBottom;
  }, []);

  // Auto-scroll on new entries
  useEffect(() => {
    if (autoScroll.current && !paused) {
      scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
    }
  }, [entries, paused]);

  // WebSocket connection — reconnects when source changes
  useEffect(() => {
    setEntries([]);
    setConnected(false);
    setPaused(false);
    autoScroll.current = true;

    const { wsPath } = SOURCE_CONFIG[source];
    const cleanup = connectWs(
      wsPath,
      (data) => {
        const entry = data as LogEntry;
        if (entry.type === 'lagged') return;
        setEntries((prev) => {
          const next = [...prev, entry];
          return next.length > MAX_ENTRIES ? next.slice(-MAX_ENTRIES) : next;
        });
      },
      setConnected,
    );
    return cleanup;
  }, [source]);

  const filteredEntries = useMemo(
    () =>
      entries.filter((entry) => {
        if (paused) return true;
        const matchesLevel = level === 'ALL' || entry.level === level;
        const haystack = `${entry.target} ${entry.message}`.toLowerCase();
        return matchesLevel && haystack.includes(query.trim().toLowerCase());
      }),
    [entries, level, query, paused],
  );

  const displayEntries = paused ? filteredEntries : filteredEntries;

  const levelCounts = useMemo(() => {
    const counts: Record<string, number> = { ERROR: 0, WARN: 0, INFO: 0, DEBUG: 0, TRACE: 0 };
    for (const e of entries) {
      counts[e.level] = (counts[e.level] ?? 0) + 1;
    }
    return counts;
  }, [entries]);

  const { label: sourceLabel, wsPath, description: sourceDescription } = SOURCE_CONFIG[source];

  return (
    <div className="space-y-6">
      <PageHeader
        actions={
          <>
            <Button
              onClick={() => setPaused((p) => !p)}
              variant={paused ? 'default' : 'secondary'}
            >
              {paused ? <Play className="size-4" /> : <Pause className="size-4" />}
              {paused ? 'Resume' : 'Pause'}
            </Button>
            <Button onClick={() => setEntries([])} variant="outline">
              <Trash2 className="size-4" />
              Clear
            </Button>
          </>
        }
        badge={
          <span className="inline-flex items-center gap-2">
            <Circle
              className={`size-2 ${connected ? 'fill-emerald-400 text-emerald-400' : 'fill-amber-400 text-amber-400'}`}
            />
            {connected ? 'Live' : 'Reconnecting...'}
          </span>
        }
        description="Real-time log stream via WebSocket. Entries are buffered client-side (max 2,000)."
        eyebrow="Logs"
        title="Server event stream"
      />

      <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader className="space-y-4">
            <div className="space-y-2">
              <Badge variant="secondary">Tail View</Badge>
              <CardTitle>Live log stream</CardTitle>
              <CardDescription>
                Connected to <code className="text-xs">{wsPath}</code>.
                {entries.length > 0 && ` ${entries.length} entries buffered.`}
              </CardDescription>
            </div>
            <div className="flex flex-col gap-3">
              {/* Source selector */}
              <Tabs defaultValue="server" value={source} onValueChange={(v) => setSource(v as LogSource)}>
                <TabsList className="justify-start">
                  <TabsTrigger value="server">Game Server</TabsTrigger>
                  <TabsTrigger value="supervisor">Supervisor</TabsTrigger>
                </TabsList>
              </Tabs>
              {/* Level filter */}
              <Tabs defaultValue="ALL" onValueChange={setLevel}>
                <TabsList className="justify-start">
                  <TabsTrigger value="ALL">All</TabsTrigger>
                  <TabsTrigger value="ERROR">Error</TabsTrigger>
                  <TabsTrigger value="WARN">Warn</TabsTrigger>
                  <TabsTrigger value="INFO">Info</TabsTrigger>
                  <TabsTrigger value="DEBUG">Debug</TabsTrigger>
                  <TabsTrigger value="TRACE">Trace</TabsTrigger>
                </TabsList>
              </Tabs>
              <div className="max-w-md">
                <Input
                  onChange={(event) => setQuery(event.currentTarget.value)}
                  placeholder="Search by target or message"
                  value={query}
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            <div
              ref={scrollRef}
              onScroll={handleScroll}
              className="subtle-scrollbar h-[600px] overflow-y-auto rounded-[28px] border border-white/8 bg-[#09131d] p-4 font-mono text-xs text-slate-100 shadow-inner shadow-black/35"
            >
              <div className="mb-4 flex items-center gap-3">
                <div
                  className={`size-2 rounded-full ${
                    connected
                      ? 'bg-emerald-300 shadow-[0_0_14px_rgba(110,231,183,0.85)]'
                      : 'bg-amber-300 shadow-[0_0_14px_rgba(252,211,77,0.85)]'
                  }`}
                />
                <span className="uppercase tracking-[0.24em] text-slate-400">
                  {connected ? (paused ? 'Paused' : `Streaming — ${sourceLabel}`) : 'Connecting...'}
                </span>
              </div>
              <div className="space-y-1">
                {displayEntries.length === 0 && (
                  <div className="py-8 text-center text-slate-500">
                    {connected
                      ? 'Waiting for log entries...'
                      : `Connecting to ${sourceLabel.toLowerCase()}...`}
                  </div>
                )}
                {displayEntries.map((entry, i) => (
                  <div key={i} className="flex gap-2 leading-5 hover:bg-white/[0.03] rounded px-1">
                    <span className="shrink-0 text-slate-500">{formatTime(entry.timestamp_ms)}</span>
                    <span
                      className={`shrink-0 w-12 text-center rounded px-1 text-[10px] font-semibold tracking-wider ${LEVEL_COLORS[entry.level] ?? LEVEL_COLORS.INFO}`}
                    >
                      {entry.level}
                    </span>
                    <span className="shrink-0 text-slate-400 max-w-[220px] truncate" title={entry.target}>
                      {entry.target}
                    </span>
                    <span className="text-slate-100 break-all">
                      {entry.message}
                      {Object.keys(entry.fields ?? {}).length > 0 && (
                        <span className="text-slate-500"> {formatFields(entry.fields)}</span>
                      )}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="outline">Summary</Badge>
              <CardTitle>Level breakdown</CardTitle>
              <CardDescription>
                Counts from the current buffer ({entries.length} entries).
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {(['ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE'] as const).map((lvl) => (
                <div
                  key={lvl}
                  className="flex items-center justify-between rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3"
                >
                  <span
                    className={`rounded-full px-2 py-0.5 text-[10px] font-semibold tracking-wider ${LEVEL_COLORS[lvl]}`}
                  >
                    {lvl}
                  </span>
                  <span className="text-sm font-medium text-foreground tabular-nums">
                    {levelCounts[lvl] ?? 0}
                  </span>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="space-y-3">
              <Badge variant="secondary">Connection</Badge>
              <CardTitle>Stream status</CardTitle>
              <CardDescription>{sourceDescription}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex items-center justify-between rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                <span className="text-sm text-muted-foreground">Source</span>
                <Badge variant="outline">{sourceLabel}</Badge>
              </div>
              <div className="flex items-center justify-between rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                <span className="text-sm text-muted-foreground">WebSocket</span>
                <Badge variant={connected ? 'success' : 'outline'}>
                  {connected ? 'Connected' : 'Disconnected'}
                </Badge>
              </div>
              <div className="flex items-center justify-between rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                <span className="text-sm text-muted-foreground">Buffer</span>
                <span className="text-sm font-medium text-foreground tabular-nums">
                  {entries.length} / {MAX_ENTRIES}
                </span>
              </div>
              <div className="flex items-center justify-between rounded-[24px] border border-white/6 bg-white/[0.03] px-4 py-3">
                <span className="text-sm text-muted-foreground">Auto-reconnect</span>
                <Badge variant="success">Enabled</Badge>
              </div>
            </CardContent>
          </Card>
        </div>
      </section>
    </div>
  );
}
