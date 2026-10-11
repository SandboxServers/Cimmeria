// Logic UAT (LX-12): the real Settings page with the PhysX panel, against a stateful
// native fake that behaves like the engine: detection, the pinned-package check,
// a UAC prompt the player can decline, and the installer's exit code. Nothing here
// runs msiexec; engine tests cover the package hash and the result mapping.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountPhysx} from './.test-build/physx-view.mjs';
const page=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const snapshot={schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,
 preferences:{schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}};

function machine() {
 const m={installed:false,uac:'decline',package:'pinned',exit:0,runs:0,last:null};
 const status=()=>({schema_version:1,supported:true,detection:{core_registered:m.installed,core_present:m.installed},
  installer_available:true,installing:false,last_result:m.last});
 m.invoke=async(command,args)=>{
  assert.equal(command,'physx_command');
  const request=args.request;
  if(request.command==='inspect')return status();
  assert.equal(request.command,'install');
  if(m.package!=='pinned')throw 'package_mismatch';
  if(m.uac==='decline'){m.last={state:'cancelled_at_uac'};return status();}
  m.runs++;
  if(m.exit===0||m.exit===3010)m.installed=true;
  m.last=m.installed?{state:'installed',restart_required:m.exit===3010}:m.exit===1638?{state:'other_version_blocks'}:{state:'failed',exit_code:m.exit};
  return status();
 };
 return m;
}
function open(m) {
 const {document,window}=parseHTML(page);
 let latest=snapshot;const listeners=new Set();
 const context={snapshot:()=>latest,onSnapshot:l=>{listeners.add(l);return()=>listeners.delete(l);},refresh:async()=>{}};
 const app=mountPhysx(document,m.invoke,context);
 const get=id=>document.getElementById(id);
 const press=async()=>{get('install-physx').dispatchEvent(new window.Event('click'));const busy=[get('install-physx').disabled,get('install-physx').textContent];await app.settled();return busy;};
 return {app,get,press,text:id=>get(id).textContent};
}

const m=machine();
let p=open(m);await p.app.ready;
assert.ok(p.get('physx-panel').closest('#gear'),'the panel sits in Settings');
assert.equal(p.get('physx-play-warning').hidden,false,'missing PhysX warns beside Play');
assert.equal(p.get('launch').disabled,true,'Play keeps its own gating; the warning does not change it');

m.package='tampered';
assert.deepEqual(await p.press(),[true,'Installing…']);
assert.match(p.text('physx-status'),/not the expected file, so it was not run/);
assert.equal(m.runs,0,'a mismatched package never reaches the installer');

m.package='pinned';
assert.deepEqual(await p.press(),[true,'Installing…']);
assert.equal(p.text('physx-status'),'Install cancelled at the Windows permission prompt. Nothing was changed.');
assert.equal(m.runs,0);

m.uac='accept';m.exit=1603;
await p.press();
assert.equal(p.text('physx-status'),'The PhysX installer failed (exit code 1603). Try again, or install "NVIDIA PhysX Legacy System Software" yourself.');
assert.equal(p.get('physx-play-warning').hidden,false);

m.exit=1638;
await p.press();
assert.match(p.text('physx-status'),/^Another PhysX version is installed and blocks this installer/);
assert.equal(p.get('physx-play-warning').hidden,false,'a blocking older or newer PhysX still reads as missing');

m.exit=0;
await p.press();
assert.equal(p.text('physx-status'),'PhysX System Software installed.');
assert.equal(p.get('physx-play-warning').hidden,true,'the warning clears once PhysX is detected');
assert.equal(p.get('install-physx').hidden,true);
await p.app.dispose();

p=open(m);await p.app.ready;
assert.equal(p.text('physx-state'),'Installed.','a reopened launcher detects the installed software');
assert.equal(p.text('physx-status'),'PhysX System Software installed.','the last result survives a lost reply');
assert.equal(p.get('physx-play-warning').hidden,true);
await p.app.dispose();

console.log('PhysX UAT passed: missing PhysX warns beside Play; each press shows busy first; a mismatched package is refused unrun; a declined UAC prompt reads as cancelled; an installer failure shows its exit code; another PhysX version blocking the installer gets its own line; success clears the warning and survives a reopen.');
console.log('Excluded: the real engine, registry, UAC prompt and msiexec (engine tests cover detection, the pinned hash and exit-code mapping); the packaged window and visual layout.');
