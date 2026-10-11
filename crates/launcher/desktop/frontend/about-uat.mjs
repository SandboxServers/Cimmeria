// Logic UAT (LX-17): the real Settings page reads the build identity from the
// native snapshot, for a tagged release and for an untagged development build.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountLauncher} from './.test-build/view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const snapshot=launcher=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},requires_reopen:false,adopted_login_servers:null,launcher});
for(const [launcher,about,badge] of [
 [{version:'2026.10.11',tag:'launcher-20261011-abc1234'},'Stargate Worlds Launcher 2026.10.11 · release launcher-20261011-abc1234','Version 2026.10.11'],
 [{version:'0.1.0',tag:null},'Stargate Worlds Launcher 0.1.0 · development build','Development build'],
]){
 const {document}=parseHTML(html);
 const app=mountLauncher(document,async command=>command==='launcher_command'?snapshot(launcher):[]);
 try{
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  document.getElementById('settings').dispatchEvent(new document.defaultView.Event('click'));
  assert.equal(document.getElementById('gear').hidden,false);
  assert.equal(document.getElementById('about').textContent,about);
  assert.equal(document.getElementById('build-badge').textContent,badge);
  console.log(`About: ${about} | header: ${badge}`);
 }finally{await app.dispose();}
}
console.log('About UAT passed: Settings shows the compiled version with the release tag, or "development build" when untagged.');
console.log('Excluded: the native window title (set by the shell) and the packaged window.');
