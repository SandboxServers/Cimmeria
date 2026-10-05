// Production Effect/view and native host, all persistence restricted to temporary fixtures.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountMigration} from './.test-build/migration-view.mjs';
const child=spawn(process.env.MIGRATION_UAT_BINARY,['--ignored','--nocapture','migration_native_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`native fixture exit ${code}`)));});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('MIGRATION_NATIVE_UAT ')){const response=pending.shift();const result=JSON.parse(line.slice(21));result.error?response.reject(result.error):response.resolve(result.ok);}});
const send=request=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('native reply deadline')),10000);pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];const app=mountMigration(document,async(command,args)=>{const request=command==='choose_legacy_source'?{command:'choose'}:args.request;calls.push(request.command);return send(request);});
const get=id=>document.getElementById(id),click=id=>get(id).dispatchEvent(new window.Event('click'));
const tick=()=>new Promise(resolve=>setImmediate(resolve));
try{
 await app.ready;await tick();const before=await send({command:'inspect',schema_version:1});
 click('choose-legacy');await app.settled();await tick();
 assert.equal(calls.filter(x=>x==='confirm').length,0);
 const preview=await send({command:'inspect',schema_version:1});assert.equal(preview.imported,null);assert.deepEqual(preview.native.preferences,before.native.preferences);
 assert.match(get('migration-details').textContent,/12345678-1234-4234-8234-123456789abc/);assert.match(get('migration-consent').textContent,/Game telemetry consent: off/);
 click('confirm-migration');click('confirm-migration');await app.settled();await tick();assert.equal(calls.filter(x=>x==='confirm').length,1);
 const saved=await send({command:'inspect',schema_version:1});assert.deepEqual(saved.imported,preview.preview.imported);assert.equal(saved.native.preferences.launcher_summary_consent,false);assert.equal(saved.native.preferences.revision,before.native.preferences.revision+1);
 const reopened=await send({command:'reopen'});assert.deepEqual(reopened.imported,saved.imported);assert.deepEqual(reopened.native.preferences,saved.native.preferences);
 await app.refresh();await tick();assert.match(get('migration-status').textContent,/does not enable Play, Repair or Uninstall/);assert.equal(get('migration-actions').hidden,true);
 console.log('Native migration logic UAT passed: read-only preview, explicit single confirmation, identity/configuration/ordered adopted ledger/consent retention, disk reopen and truthful unverified next step.');
 console.log('Excluded: real legacy folders, native chooser interaction, Windows lock interoperability, power loss, packaged visual/focus/layout UAT, signed adoption/updater/launch parity.');
}finally{await app.dispose();child.stdin.end();await exit;}
