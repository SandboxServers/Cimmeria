import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';

const html = await readFile(new URL('../ui/index.html', import.meta.url), 'utf8');
const root = new URL('../../../../../', import.meta.url);
const summary = 'crates/launcher/desktop/engine/src/storage/launcher_summary/schema.rs';
const events = 'crates/launcher-core/src/telemetry/events.rs';
const schemas: Record<string, string> = {
  SummaryRequest:summary, Summary:summary, DroppedCounts:summary, PhaseDuration:summary,
  MintRequest:'crates/launcher/desktop/engine/src/storage/game_telemetry/session.rs',
  DevSessionRequest:'crates/launcher-core/src/telemetry/auth.rs',
  BundleMetadata:'crates/launcher-core/src/telemetry/bundle.rs',
  ClientLogEvent:events, DebugLogEvent:events, ClientNativeEvent:events, SessionMetaEvent:events,
  Stored:'crates/launcher/desktop/engine/src/sign_in/store.rs',
};

// Controlled Rust DTO declarations, not a general Rust parser. Fail closed if a
// declaration changes shape so this independent disclosure check needs updating.
for (const [name, path] of Object.entries(schemas)) {
  test(`privacy names every collected ${name} field from its native schema`, async () => {
    const source = await readFile(new URL(path, root), 'utf8');
    const body = source.match(new RegExp(`(?:pub )?struct ${name}(?:<[^>]+>)? \\{([\\s\\S]*?)\\r?\\n\\}`))?.[1];
    assert.ok(body, `cannot inspect ${name}; update the schema coverage guard`);
    const native = [...body.matchAll(/^    (?:pub )?(\w+):/gm)].map(m => m[1]).sort();
    assert.ok(native.length > 0, `no fields found in ${name}`);
    const {document} = parseHTML(html);
    const disclosed = Array.from(document.querySelectorAll(`[data-privacy-schema="${name}"] [data-field]`))
      .map(el => {assert.equal(el.textContent, el.getAttribute('data-field')); return el.textContent!;}).sort();
    assert.deepEqual(disclosed, native, `${name}: every wire field must be disclosed, with no duplicates`);
  });
}

test('privacy distinguishes identities, raw logs, consent withdrawal and network visibility', () => {
  const {document} = parseHTML(html);
  const text = (id:string) => document.getElementById(id)!.textContent!;
  assert.match(text('privacy-choices'), /separate, optional choices, both off by default/);
  assert.match(text('privacy-choices'), /even with both choices off/);
  assert.match(text('privacy-choices'), /exit the running game to stop it/);
  assert.match(text('privacy-choices'), /requests already in flight may finish/);
  assert.match(text('game-telemetry-privacy'), /not anonymous/);
  assert.match(text('privacy-game'), /chat, inventory\/trade\/mission data/);
  assert.match(text('privacy-game'), /remote IPv4 peer address and port/);
  assert.match(text('privacy-files'), /sessions\/\*\.txt/);
  assert.match(text('privacy-files'), /no special exclusion for \*-keys\.txt/);
  assert.match(text('privacy-files'), /Raw logs are sent without the manual-upload line scrubber/);
  assert.match(text('privacy-identity'), /SHA256\(computer name\)/);
  assert.match(text('privacy-identity'), /see connection addresses\/metadata/);
  assert.match(text('privacy-identity'), /not unconditionally HTTPS-only/);
  assert.match(text('privacy-identity'), /first effective login server only when it is HTTPS; later entries are not searched/);
  assert.match(text('privacy-identity'), /first server is HTTP, a later HTTPS entry does not enable sending/);
  assert.match(text('privacy-identity'), /omit the receiving HTTP caller's connection address/);
  assert.match(text('privacy-storage'), /not a verified live retention guarantee/);
  assert.match(text('privacy-sign-in'), /operating system's credential store/);
  assert.match(text('privacy-sign-in'), /only for providers the server operator marks as verifying email/);
  assert.match(text('privacy-sign-in'), /none of this goes to diagnostics/);
  assert.match(text('privacy-exclusions'), /No webcam or microphone recording/);
  assert.match(text('privacy-exclusions'), /No browser-history, clipboard, password-manager or personal-document crawler/);
  assert.match(text('privacy-exclusions'), /No lab remote-control bridge in the distributed player module/);
});

test('complete disclosure is offline and collapsed without moving or enabling consent controls', () => {
  const {document} = parseHTML(html);
  const privacy = document.getElementById('diagnostics-privacy')!;
  const sections = ['choices','launcher','game','files','identity','storage','exclusions','sign-in','fields'];
  assert.equal(privacy.hasAttribute('open'), false);
  for (const section of sections) {
    const details = document.getElementById(`privacy-${section}`)!;
    assert.equal(details.parentElement, privacy);
    assert.equal(details.hasAttribute('open'), false);
    assert.ok(details.textContent!.length > 150, `${section} must include its own disclosure`);
  }
  assert.equal(privacy.querySelectorAll('script,iframe,img,object,form,input,button').length, 0);
  assert.equal(privacy.querySelectorAll('a').length, 1, 'source link is supplemental, not the disclosure');
  for (const id of ['telemetry','game-telemetry']) {
    const control = document.getElementById(id)!;
    assert.equal(privacy.contains(control), false);
    assert.equal(control.hasAttribute('checked'), false);
  }
});
