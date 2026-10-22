// LT-25: the launcher-telemetry logic UAT. The production native bridge emits the five
// launcher events through the real forwarder layer and launcher_batch; this script checks
// the rows it prints against the allowed targets and the privacy rules. Temporary state
// only; nothing is sent.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import net from 'node:net';

const binary = process.env.LAUNCHER_TELEMETRY_UAT_BINARY;
if (!binary) {
  console.log('FAIL LAUNCHER_TELEMETRY_UAT_BINARY is not set');
  process.exit(1);
}

const PREFIX = 'LAUNCHER_TELEMETRY_UAT ';
const REQUIRED = [
  ['client.launcher.startup', 'launcher_started'],
  ['client.launcher.ui', 'install_failed'],
  ['client.launcher.settings', 'client_options_saved'],
  ['client.launcher.telemetry', 'pipeline_recovered'],
  ['client.launcher.launch', 'launch_not_started'],
];
const TARGETS = new URL('../../../launcher-core/src/telemetry/launcher_targets.txt', import.meta.url);

// A port of the LT-40 token rule (ip_scrub.rs): a token that is an IP, or an IP with a port.
const hasIp = (s) => {
  for (const token of s.split(/[^0-9A-Fa-f:.\[\]%]+/)) {
    if (token === '') continue;
    const tries = [token];
    const port = /^(.*):\d+$/.exec(token);
    if (port) tries.push(port[1]);
    for (const attempt of tries) {
      const address = attempt.replace(/^\[/, '').replace(/\]$/, '').split('%')[0];
      if (net.isIP(address) !== 0) return true;
    }
  }
  return false;
};

// Every string in a value, at any depth.
const strings = (value, out = []) => {
  if (typeof value === 'string') out.push(value);
  else if (Array.isArray(value)) value.forEach((item) => strings(item, out));
  else if (value && typeof value === 'object') Object.values(value).forEach((item) => strings(item, out));
  return out;
};

const readTargets = async () => (await readFile(TARGETS, 'utf8'))
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter((line) => line !== '' && !line.startsWith('#'));

const runBridge = () => new Promise((resolve, reject) => {
  const child = spawn(binary, ['--ignored', '--exact', 'host::launcher_telemetry_uat::uat_bridge_launcher_telemetry', '--nocapture'], {stdio: ['ignore', 'pipe', 'inherit']});
  const rows = [];
  createInterface({input: child.stdout}).on('line', (line) => {
    if (line.startsWith(PREFIX)) rows.push(JSON.parse(line.slice(PREFIX.length)));
  });
  child.on('error', reject);
  child.on('close', (code) => resolve({code, rows}));
});

const {code, rows} = await runBridge();
const allowed = new Set(await readTargets());
let failed = false;
const check = (name, ok, detail = '') => {
  if (!ok) failed = true;
  console.log(ok ? `PASS ${name}` : `FAIL ${name}: ${detail}`);
};

check('bridge exits 0', code === 0, `exit code ${code}`);
check('rows were read', rows.length > 0, 'no LAUNCHER_TELEMETRY_UAT rows');

const badType = rows.filter((row) => row.type !== 'client_native' || !allowed.has(row.target));
check('every row is client_native with an allowed target', badType.length === 0,
  badType.map((row) => `${row.type}/${row.target}`).join(', '));

const wide = rows.filter((row) => Object.keys(row.fields ?? {}).length > 16);
check('every row has at most 16 fields', wide.length === 0, `${wide.length} rows over 16`);

const leaks = [];
for (const row of rows) {
  for (const s of strings(row.fields ?? {})) {
    if (/[A-Za-z]:\\Users\\/i.test(s) || s.includes('://') || hasIp(s)) leaks.push(`${row.target}: ${s.slice(0, 40)}`);
  }
}
check('no home path, URL or IP in any field', leaks.length === 0, leaks.slice(0, 3).join('; '));

for (const [target, event] of REQUIRED) {
  const found = rows.some((row) => row.target === target && row.fields?.event === event);
  check(`event ${event} on ${target}`, found, 'missing from the rows');
}

if (!failed) {
  console.log(`Launcher telemetry logic UAT passed: ${rows.length} rows, every target allowed, at most 16 fields, no home path, URL or IP, the five events present.`);
} else {
  process.exitCode = 1;
}
console.log('Excluded: the forwarder thread and LauncherSender, session mint, any send, SigNoz, and the launch worker\'s own call sites (pinned by events_tests.rs and LT-18).');
