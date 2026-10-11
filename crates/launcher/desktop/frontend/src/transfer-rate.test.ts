import test from 'node:test';
import assert from 'node:assert/strict';
import { formatBytes, formatDuration, makeRateTracker } from './transfer-rate';

const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;

test('formatBytes uses base 1024 with unit boundaries', () => {
  assert.equal(formatBytes(0), '0 B');
  assert.equal(formatBytes(1023), '1023 B');
  assert.equal(formatBytes(KB), '1 KB');
  assert.equal(formatBytes(512 * KB), '512 KB');
  assert.equal(formatBytes(MB), '1 MB');
  assert.equal(formatBytes(1.2 * MB), '1.2 MB');
  assert.equal(formatBytes(GB), '1 GB');
  assert.equal(formatBytes(3.85 * GB), '3.85 GB');
  assert.equal(formatBytes(4.13 * GB), '4.13 GB');
});

test('formatBytes promotes a unit instead of printing four digits', () => {
  assert.equal(formatBytes(GB - 1), '1 GB');
  assert.equal(formatBytes(MB - KB / 2), '1 MB');
  assert.equal(formatBytes(1023.6), '1 KB');
  assert.equal(formatBytes(1023.4), '1023 B');
});

test('formatBytes treats invalid input as zero', () => {
  assert.equal(formatBytes(-5), '0 B');
  assert.equal(formatBytes(Number.NaN), '0 B');
});

test('formatDuration covers seconds, minutes and hours', () => {
  assert.equal(formatDuration(0), '0 s');
  assert.equal(formatDuration(45), '45 s');
  assert.equal(formatDuration(59.4), '59 s');
  assert.equal(formatDuration(59.6), '1 min');
  assert.equal(formatDuration(180), '3 min');
  assert.equal(formatDuration(192), '3 min 12 s');
  assert.equal(formatDuration(3600), '1 h');
  assert.equal(formatDuration(3900), '1 h 05 min');
  assert.equal(formatDuration(-10), '0 s');
});

test('rate stays null until two samples span at least one second', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  assert.deepEqual(tracker.sample(0, 1000), { percent: 0, rateBytesPerSec: null, etaSeconds: null });
  t = 500;
  assert.deepEqual(tracker.sample(50, 1000), { percent: 5, rateBytesPerSec: null, etaSeconds: null });
  t = 1000;
  const sample = tracker.sample(100, 1000);
  assert.equal(sample.rateBytesPerSec, 100);
  assert.equal(sample.etaSeconds, 9);
});

test('rate and ETA come from the injected clock', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 10_000);
  t = 2000;
  const sample = tracker.sample(1000, 10_000);
  assert.equal(sample.percent, 10);
  assert.equal(sample.rateBytesPerSec, 500);
  assert.equal(sample.etaSeconds, 18);
});

test('rate is a sliding window, so a speed change shows within the window', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1_000_000);
  t = 1000;
  tracker.sample(100, 1_000_000);
  t = 2000;
  tracker.sample(200, 1_000_000);
  // Slow down to 10 per second; the earlier fast readings leave the 8 s window.
  let sample = tracker.sample(200, 1_000_000);
  for (let i = 1; i <= 12; i++) {
    t = 2000 + i * 1000;
    sample = tracker.sample(200 + i * 10, 1_000_000);
  }
  assert.equal(sample.rateBytesPerSec, 10);
});

test('a stalled transfer reports zero rate and no ETA', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1000);
  t = 1000;
  tracker.sample(100, 1000);
  t = 2000;
  tracker.sample(200, 1000);
  for (let s = 3; s <= 12; s++) {
    t = s * 1000;
    tracker.sample(200, 1000);
  }
  assert.deepEqual(tracker.sample(200, 1000), { percent: 20, rateBytesPerSec: 0, etaSeconds: null });
});

test('reset when current goes backwards clears the rate history', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1000);
  t = 1000;
  tracker.sample(100, 1000);
  t = 1500;
  assert.equal(tracker.sample(50, 1000).rateBytesPerSec, null);
  t = 2500;
  assert.equal(tracker.sample(150, 1000).rateBytesPerSec, 100);
});

test('reset when the total changes starts a new blob', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1000);
  t = 1000;
  tracker.sample(500, 1000);
  t = 1500;
  assert.deepEqual(tracker.sample(500, 2000), { percent: 25, rateBytesPerSec: null, etaSeconds: null });
});

test('explicit reset discards readings', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1000);
  t = 1000;
  tracker.sample(100, 1000);
  tracker.reset();
  t = 1500;
  assert.equal(tracker.sample(200, 1000).rateBytesPerSec, null);
});

test('percent is clamped and zero without a total', () => {
  const tracker = makeRateTracker(() => 0);
  assert.equal(tracker.sample(5, 0).percent, 0);
  assert.equal(makeRateTracker(() => 0).sample(12, 10).percent, 100);
});

test('a gap longer than the window discards the stale baseline', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1_000_000);
  t = 1000;
  assert.equal(tracker.sample(100, 1_000_000).rateBytesPerSec, 100);
  // Polling paused for 30 s; the old reading must not average the pause into the rate.
  t = 31_000;
  assert.equal(tracker.sample(5000, 1_000_000).rateBytesPerSec, null);
  t = 32_000;
  assert.equal(tracker.sample(6000, 1_000_000).rateBytesPerSec, 1000);
});

test('no ETA once nothing remains', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1000);
  t = 1000;
  const done = tracker.sample(1000, 1000);
  assert.equal(done.rateBytesPerSec, 1000);
  assert.equal(done.etaSeconds, null);
  t = 2000;
  assert.equal(tracker.sample(1500, 1000).etaSeconds, null);
});

test('an ETA beyond 24 hours is omitted', () => {
  let t = 0;
  const tracker = makeRateTracker(() => t);
  tracker.sample(0, 1_000_000_000);
  t = 1000;
  // 1 B/s with ~1e9 B left is roughly 31 years.
  const slow = tracker.sample(1, 1_000_000_000);
  assert.equal(slow.rateBytesPerSec, 1);
  assert.equal(slow.etaSeconds, null);
  // 100 B/s with exactly 24 h left is still shown; one more second is not.
  const total = 86_400 * 100 + 100;
  const edge = makeRateTracker(() => t);
  t = 0;
  edge.sample(0, total);
  t = 1000;
  assert.equal(edge.sample(100, total).etaSeconds, 86_400);
  const over = makeRateTracker(() => t);
  t = 0;
  over.sample(0, total + 100);
  t = 1000;
  assert.equal(over.sample(100, total + 100).etaSeconds, null);
});