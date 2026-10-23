// Logic UAT (LX-17): the real Settings page reads the build identity from the
// native snapshot, for a tagged release and for an untagged development build.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountLauncher} from './.test-build/view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const snapshot=launcher=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},requires_reopen:false,adopted_login_servers:null,launcher});
for(const [launcher,about,badge] of [
 [{version:'2026.10.11',tag:'launcher-20261011-abc1234'},'Cimmeria Launcher 2026.10.11 · release launcher-20261011-abc1234','Version 2026.10.11'],
 [{version:'0.1.0',tag:null},'Cimmeria Launcher 0.1.0 · development build','Development build'],
]){
 const {document}=parseHTML(html);
 const calls=[];
 const app=mountLauncher(document,async command=>{calls.push(command);return command==='launcher_command'?snapshot(launcher):[];});
 try{
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  document.getElementById('settings').dispatchEvent(new document.defaultView.Event('click'));
  await app.settled();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(document.getElementById('gear').hidden,false);
  assert.equal(document.getElementById('about').textContent,about);
  assert.equal(document.getElementById('build-badge').textContent,badge);
  // Read the actual offline disclosure while native state stays unchanged.
  const before=JSON.stringify(snapshot(launcher)),beforeCalls=calls.length;
  const privacy=document.getElementById('diagnostics-privacy');
  assert.equal(privacy.hasAttribute('open'),false);
  privacy.setAttribute('open','');privacy.dispatchEvent(new document.defaultView.Event('toggle'));
  for(const section of privacy.querySelectorAll('details')){
   assert.equal(section.hasAttribute('open'),false);
   section.setAttribute('open','');section.dispatchEvent(new document.defaultView.Event('toggle'));
   assert.ok(section.textContent.length>150);
   section.removeAttribute('open');section.dispatchEvent(new document.defaultView.Event('toggle'));
  }
  await new Promise(resolve=>setImmediate(resolve));
  assert.equal(calls.length,beforeCalls,'reading privacy must not call native IPC');
  assert.equal(JSON.stringify(snapshot(launcher)),before,'reading privacy must not save consent');
  for(const id of ['telemetry','game-telemetry'])assert.equal(document.getElementById(id).checked??false,false);
  assert.equal(privacy.querySelectorAll('[data-privacy-schema]').length,11);
  privacy.removeAttribute('open');
  console.log(`About: ${about} | header: ${badge}`);
 }finally{await app.dispose();}
}
console.log('About/privacy UAT passed: release/development identities; full static offline disclosure; expand/read/collapse all eight sections; eleven schema lists; no native IPC, saved consent or enabled diagnostic controls from reading.');
console.log('Excluded: actual native consent persistence, real collectors/uploads, server retention, OS disclosure keyboard behavior, native window title and packaged window.');
