/**
 * Formatting and rate helpers for install progress: byte sizes, durations, and a
 * sliding-window transfer rate with ETA. Pure apart from the injected clock, so
 * the install view and tests can drive it with fake time.
 */

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];
/** Decimal places per unit index: whole KB, one decimal MB, two decimal GB and above. */
const DECIMALS = [0, 0, 1, 2, 2];
/** Samples older than this (ms) fall out of the window, keeping one baseline point. */
const WINDOW_MS = 8000;
/** Rate needs samples spanning at least this long (ms) to be meaningful. */
const MIN_SPAN_MS = 1000;
/** An ETA beyond this many seconds (24 h) is noise from a near-stalled link, so it is omitted. */
const MAX_ETA_SECONDS = 24 * 3600;

/** Formats a byte count with base 1024, e.g. "512 KB", "1.2 MB", "3.85 GB". */
export function formatBytes(bytes: number): string {
  const n = Number.isFinite(bytes) && bytes > 0 ? bytes : 0;
  if (Math.round(n) < 1024) return `${Math.round(n)} B`;
  let unit = 1;
  let value = n / 1024;
  // Promote when rounding would print four digits, so 1023.99 MB reads as 1 GB.
  while (unit < UNITS.length - 1 && Number(value.toFixed(DECIMALS[unit])) >= 1024) {
    value /= 1024;
    unit++;
  }
  return `${Number(value.toFixed(DECIMALS[unit]))} ${UNITS[unit]}`;
}

/** Formats a duration in seconds, e.g. "45 s", "3 min 12 s", "1 h 05 min". */
export function formatDuration(seconds: number): string {
  const s = Number.isFinite(seconds) && seconds > 0 ? Math.round(seconds) : 0;
  if (s < 60) return `${s} s`;
  if (s < 3600) {
    const rest = s % 60;
    return `${Math.floor(s / 60)} min${rest ? ` ${rest} s` : ''}`;
  }
  const minutes = Math.floor((s % 3600) / 60);
  return `${Math.floor(s / 3600)} h${minutes ? ` ${String(minutes).padStart(2, '0')} min` : ''}`;
}

/** One reading of a transfer: whole percent, bytes per second and seconds left, where known. */
export type TransferSample = {
  percent: number;
  rateBytesPerSec: number | null;
  etaSeconds: number | null;
};

/** Stateful tracker fed with `(current, total)` readings over time. */
export type RateTracker = {
  sample: (current: number, total: number) => TransferSample;
  reset: () => void;
};

/**
 * Creates a tracker whose rate is the average over the last ~8 seconds. It resets
 * when the total changes (a new blob or phase), when current goes backwards, or
 * when samples are more than the window apart (a stale baseline).
 * `now` returns milliseconds and defaults to `performance.now()`.
 */
export function makeRateTracker(now: () => number = () => performance.now()): RateTracker {
  let points: { t: number; current: number }[] = [];
  let total: number | null = null;

  const reset = () => {
    points = [];
    total = null;
  };

  const sample = (current: number, totalNow: number): TransferSample => {
    const t = now();
    const last = points[points.length - 1];
    // A gap longer than the window means the baseline is stale (polling paused), so start over.
    if (totalNow !== total || (last && (current < last.current || t - last.t > WINDOW_MS))) {
      points = [];
      total = totalNow;
    }
    points.push({ t, current });
    // Keep the newest point at or before the window start as the baseline.
    const cutoff = t - WINDOW_MS;
    while (points.length > 2 && points[1].t <= cutoff) points.shift();

    const percent = totalNow > 0 ? Math.min(100, Math.max(0, Math.floor((current * 100) / totalNow))) : 0;
    const first = points[0];
    const span = t - first.t;
    if (points.length < 2 || span < MIN_SPAN_MS) return { percent, rateBytesPerSec: null, etaSeconds: null };

    const rate = Math.max(0, (current - first.current) / (span / 1000));
    const remaining = totalNow - current;
    const eta = remaining > 0 && rate > 0 ? remaining / rate : null;
    const etaSeconds = eta !== null && eta <= MAX_ETA_SECONDS ? eta : null;
    return { percent, rateBytesPerSec: rate, etaSeconds };
  };

  return { sample, reset };
}
