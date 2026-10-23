import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountLauncher } from './view';
import { Command, NativeSnapshot } from './contract';

const html = await readFile(new URL('../ui/index.html', import.meta.url), 'utf8');
const initial = (): NativeSnapshot => ({schema_version:1, operation:{schema_version:1, revision:0, operation:null},
  preferences:{schema_version:2, revision:0, install_directory:null, launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}, requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null}});
const seedIdle = () => ({schema_version:1, native:initial(), chosen:null, offer:null, verifying:null, last_error:null});
const flush = () => new Promise<void>(resolve => setImmediate(resolve));

function dom() {
  const {document, window} = parseHTML(html);
  const get = <T extends HTMLElement = HTMLElement>(id:string) => document.getElementById(id)! as unknown as T;
  const click = (id:string) => get(id).dispatchEvent(new window.Event('click'));
  return {document: document as unknown as Document, get, click, window};
}

test('Settings displays normal Windows paths while saves and reopening retain native spelling', async () => {
  for (const [directory, shown] of [[String.raw`\\?\C:\Temp\sgw`, String.raw`C:\Temp\sgw`],
    [String.raw`\\?\UNC\server\share\sgw`, String.raw`\\server\share\sgw`]]) {
    let saved = {...initial(), preferences:{...initial().preferences, install_directory:directory}};
    const requests: Command[] = [];
    const invoke = async (command: string, args?: Record<string, unknown>) => {
      if (command === 'seed_archive_command') return seedIdle();
      if (command === 'launcher_command') {
        const request = args!.request as Command; requests.push(request);
        if (request.command === 'save_preferences') {
          assert.equal(request.install_directory, directory);
          saved = {...saved, preferences:{...saved.preferences, launcher_summary_consent:request.launcher_summary_consent, revision:saved.preferences.revision+1}};

        }
      }
      return structuredClone(saved);
    };
    for (const reopen of [false, true]) {
      const ui = dom(), app = mountLauncher(ui.document, invoke);
      try {
        await app.ready; await flush();
        assert.equal(ui.get<HTMLInputElement>('install-path').value, shown);
        assert.equal(saved.preferences.install_directory, directory);
        if (reopen) { assert.equal(saved.preferences.launcher_summary_consent, true); continue; }
        const consent = ui.get<HTMLInputElement>('telemetry'); consent.checked = true;
        consent.dispatchEvent(new ui.window.Event('change')); await app.settled(); await flush();
        assert.equal(requests.filter(r => r.command === 'save_preferences').length, 1);
        ui.click('folder'); await app.settled();
        await app.refresh(); await flush();
        assert.equal(ui.get<HTMLInputElement>('install-path').value, shown);
        assert.equal(saved.preferences.install_directory, directory);
      } finally { await app.dispose(); }
    }
  }
});

test('approved tabs and settings work without inventing installed state', {timeout:5000}, async () => {
  const ui = dom();
  const calls:string[]=[];
  const app = mountLauncher(ui.document, async command => { calls.push(command); return command === 'seed_archive_command' ? seedIdle() : initial(); });
  try {
    await app.ready; await flush();
    assert.equal(ui.get('play').closest('[hidden]'), null, 'Play must not be nested inside hidden settings');
    assert.equal(ui.get('adoption-title').closest('section')!.parentElement, ui.get('gear'), 'adoption and legacy import are sibling settings sections');
    assert.equal(ui.get<HTMLInputElement>('telemetry').checked, false);
    assert.equal(ui.get<HTMLInputElement>('telemetry').disabled, false);
    assert.equal(ui.get<HTMLButtonElement>('install').disabled, true);
    assert.equal(ui.get<HTMLButtonElement>('repair').disabled, true);
    assert.equal(ui.get<HTMLButtonElement>('uninstall').disabled, true);
    assert.equal(ui.document.getElementById('notes'), null);
    assert.equal(ui.document.getElementById('patches'), null);
    for (const id of ['telemetry','game-telemetry','client-patches-enabled','client-changes-panel'])
      assert.equal(ui.get(id).closest('main')?.id, 'play');
    assert.equal(ui.get('client-changes-details').hasAttribute('open'),false);
    assert.equal(ui.get('client-changes-details').querySelector('summary')!.textContent,'View all changes');
    ui.click('settings'); assert.equal(ui.get('gear').hidden, false);
    assert.equal(ui.get('play').hidden, true);
    ui.click('home'); assert.equal(ui.get('play').hidden, false);
    assert.equal(ui.get('gear').hidden, true);
    assert.equal(ui.get('settings').getAttribute('aria-expanded'), 'false');
    await app.settled();
    // The debug-log panel asks once at mount whether an upload can run (LX-05);
    // the changes (LX-07) and PhysX (LX-12) panels each read once, on the first snapshot.
    // Settings' archive panel asks once at mount (AI-07); it is counted apart from the panels above.
    assert.equal(calls.filter(c => c === 'seed_archive_command').length, 1);
    assert.deepEqual(calls.filter(c => c !== 'seed_archive_command'), ['log_upload_command', 'launcher_command', 'client_changes_command', 'physx_command', 'client_changes_command']);
  } finally { await app.dispose(); }
  ui.click('settings'); assert.equal(ui.get('gear').hidden, true, 'dispose removes handlers');
});

test('Cimmeria branding agrees with the native title and macOS app packaging',async()=>{
 const ui=dom();assert.equal(ui.document.title,'Cimmeria Launcher');
 assert.equal(ui.document.querySelector('h1')!.textContent,'Cimmeria');
 assert.match(ui.get('play').textContent!,/Your journey through the gate begins here\./);
 const config=JSON.parse(await readFile(new URL('../../shell/tauri.conf.json',import.meta.url),'utf8'));
 assert.equal(config.productName,'Cimmeria Launcher');assert.equal(config.app.windows[0].title,'Cimmeria Launcher');
 assert.equal(config.identifier,'app.cimmeria.launcher.desktop','persisted app identity stays stable');
 const pack=await readFile('../../../../tools/launcher-release/build-desktop-macos.sh','utf8');
 assert.ok(pack.includes(`APP_NAME="${config.productName}.app"`));
});

test('native adoption dialog targets SGW.exe before catalog access',async()=>{
 const picker=await readFile(new URL('../../shell/src/adoption_picker.rs',import.meta.url),'utf8');
 assert.match(picker,/blocking_pick_file\(\)/);assert.match(picker,/SGW\.exe \(game executable\)/);
 assert.match(picker,/set_file_name\("SGW\.exe"\)/);
 assert.match(picker,/resolve_executable\(&selected\)/);
 const shell=await readFile(new URL('../../shell/src/main.rs',import.meta.url),'utf8');
 const command=shell.slice(shell.indexOf('async fn choose_existing_game_folder('));
 const picked=command.indexOf('adoption_picker::pick('),fetched=command.indexOf('catalog::fetch_release()');
 assert.ok(picked>=0&&fetched>picked);
});

test('Settings shows the compiled version, with the release tag or "development build"', {timeout:5000}, async () => {
  for (const [launcher, about, badge] of [
    [{version:'2026.10.11', tag:'launcher-20261011-abc1234'},
      'Cimmeria Launcher 2026.10.11 · release launcher-20261011-abc1234', 'Version 2026.10.11'],
    [{version:'0.1.0', tag:null}, 'Cimmeria Launcher 0.1.0 · development build', 'Development build'],
  ] as const) {
    const ui = dom();
    assert.equal(ui.get('build-badge').textContent, '', 'no build kind is claimed before the snapshot');
    const app = mountLauncher(ui.document, async () => ({...initial(), launcher}));
    try {
      await app.ready; await flush();
      assert.equal(ui.get('about').textContent, about);
      assert.equal(ui.get('about').closest('section'), ui.get('gear'), 'the About line is in Settings');
      assert.equal(ui.get('build-badge').textContent, badge);
    } finally { await app.dispose(); }
  }
});

test('About stops saying "Reading" once no snapshot is coming', {timeout:5000}, async () => {
  const {launcher: _omitted, ...withoutLauncher} = initial();
  for (const [invoke, status] of [
    [async () => withoutLauncher, /versions do not match/],
    [async () => { throw 'io'; }, /could not read or save/],
  ] as const) {
    const ui = dom();
    const app = mountLauncher(ui.document, invoke);
    try {
      await app.ready; await app.settled(); await flush();
      assert.match(ui.get('status').textContent!, status);
      assert.equal(ui.get('about').textContent, 'Launcher version unavailable');
      assert.equal(ui.get('build-badge').textContent, '');
    } finally { await app.dispose(); }
  }
});

test('consent waits for native acknowledgement and rolls back on failed save', {timeout:5000}, async () => {
  const ui = dom(); let saves=0;
  let rejectSave!: (reason:unknown) => void;
  const app = mountLauncher(ui.document, async (_command,args) => {
    const request = args!.request as Command;
    if(request.command === 'inspect') return initial();
    saves++;
    return new Promise((_resolve,reject) => {rejectSave=reject;});
  });
  try {
    await app.ready; await flush();
    const checkbox=ui.get<HTMLInputElement>('telemetry');
    checkbox.checked=true; checkbox.dispatchEvent(new ui.window.Event('change'));
    await flush();
    assert.equal(checkbox.checked,true); assert.equal(checkbox.disabled,true);
    assert.match(ui.get('status').textContent!, /Saving/);
    rejectSave('io'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(checkbox.checked,false);
    assert.equal(checkbox.disabled,true, 'inspection is required after failed write');
    assert.equal(ui.get('retry').hidden,false);
    assert.match(ui.get('status').textContent!, /could not read or save/);
    ui.click('retry'); await app.settled(); await flush();
    assert.equal(checkbox.disabled,false);
  } finally {await app.dispose();}
});

const withEndpoint = (endpoint: boolean): NativeSnapshot => ({...initial(), setup_diagnostics_endpoint: endpoint});

// Turns consent on with a save that resolves, and returns the status text it leaves.
async function savedStatus(endpoint: boolean): Promise<string> {
  const ui = dom();
  const app = mountLauncher(ui.document, async (_command, args) => {
    const request = args!.request as Command;
    if (request.command === 'inspect') return withEndpoint(endpoint);
    return {...withEndpoint(endpoint), preferences: {...initial().preferences, revision: 1, launcher_summary_consent: true}};
  });
  try {
    await app.ready; await flush();
    const checkbox = ui.get<HTMLInputElement>('telemetry');
    checkbox.checked = true; checkbox.dispatchEvent(new ui.window.Event('change'));
    await app.settled(); await flush();
    return ui.get('status').textContent!;
  } finally {await app.dispose();}
}

test('the saved consent says whether setup diagnostics are sent', {timeout:5000}, async () => {
  assert.match(await savedStatus(false), /saved but not sent yet/);
  assert.match(await savedStatus(true), /sent over https/);
});

test('the diagnostics label says summaries go over https and never that the build sends nothing', {timeout:5000}, () => {
  const ui = dom();
  assert.match(ui.get('diagnostics-privacy').textContent!, /sent over https/);
  assert.doesNotMatch(html, /This build sends nothing/);
});

test('native folder choice saves once, preserves consent, and cancellation changes nothing', {timeout:5000}, async () => {
  const ui=dom(); let state=initial(); let folder:string|null='/selected/game'; let saves=0; let opens=0;
  const app=mountLauncher(ui.document, async (command,args) => {
    if(command === 'choose_install_directory') return folder;
    if(command === 'show_install_directory') { opens++; return null; }
    const request=args!.request as Command;
    if(request.command === 'save_preferences') {
      saves++;
      assert.equal(request.expected_revision,state.preferences.revision);
      state={...state,preferences:{...state.preferences, revision:state.preferences.revision+1,
        install_directory:request.install_directory, launcher_summary_consent:request.launcher_summary_consent}};
    }
    return state;
  });
  try {
    await app.ready; await flush();
    ui.click('choose'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(ui.get<HTMLInputElement>('install-path').value,'/selected/game');
    assert.equal(state.preferences.launcher_summary_consent,false);
    ui.click('folder'); await app.settled(); assert.equal(opens,1);
    folder=null;
    ui.click('choose'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(ui.get<HTMLInputElement>('install-path').value,'/selected/game');
    assert.match(ui.get('status').textContent!,/cancelled/);
  } finally { await app.dispose(); }
});

test('disposing during pending IPC removes controls without claiming a saved result', {timeout:5000}, async () => {
  const ui = dom(); let calls=0;
  const app = mountLauncher(ui.document, async command => { if (command !== 'seed_archive_command') calls++; return new Promise(() => {}); });
  await flush();
  // Launcher inspect plus the debug-log panel's inspect (LX-05); the archive panel's inspect is not counted.
  assert.equal(calls,2);
  await app.dispose();
  ui.click('settings');
  assert.equal(ui.get('gear').hidden,true);
  assert.equal(ui.get<HTMLInputElement>('telemetry').disabled,true);
  assert.equal(calls,2);
});

test('a refresh asked for during an inspect resolves only after a fresh inspect, coalesced', {timeout:5000}, async () => {
  const ui = dom(); const answers:((snapshot:NativeSnapshot)=>void)[] = [];
  const app = mountLauncher(ui.document, async (command,args) => {
    if(command !== 'launcher_command' || (args!.request as Command).command !== 'inspect') return null;
    return new Promise(resolve => answers.push(resolve));
  });
  try {
    await flush(); assert.equal(answers.length,1);
    let firstDone=false; let secondDone=false;
    // Both asked while the opening inspect is in flight: a post-save refresh must not get it.
    void app.refresh().then(() => {firstDone=true;}); void app.refresh().then(() => {secondDone=true;});
    answers[0]!(initial()); await flush(); await flush();
    assert.equal(answers.length,2, 'one queued inspect starts after the in-flight one');
    assert.equal(firstDone,false); assert.equal(secondDone,false);
    answers[1]!({...initial(),preferences:{...initial().preferences,revision:1}}); await flush(); await flush();
    assert.equal(firstDone,true); assert.equal(secondDone,true);
    assert.equal(answers.length,2, 'the two refreshes shared one inspect');
  } finally { await app.dispose(); }
});

test('native Launch ownership gates directory changes until observed exit',async()=>{
 const ui=dom();let status:NativeSnapshot={...initial(),operation:{schema_version:1,revision:1,operation:{id:'launch',kind:'launch',state:'running',intent_digest:Array(32).fill(0)}}};
 const app=mountLauncher(ui.document,async()=>status);
 try{await app.ready;await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,true);
 status={...status,operation:{...status.operation,revision:2,operation:{...status.operation.operation!,state:'reconciliation_required'}}};await app.refresh();await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,true);
 status={...status,operation:{...status.operation,revision:3,operation:{...status.operation.operation!,state:'succeeded'}}};await app.refresh();await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,false);
 }finally{await app.dispose();}
});
