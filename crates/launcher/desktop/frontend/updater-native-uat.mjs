// Production Effect/view calls the Rust updater's real signed HTTP + disk fixture.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {readFile} from 'node:fs/promises';
import {mountDesktop} from './.test-build/composition.mjs';
const child=spawn(process.env.UPDATER_UAT_BINARY,['--ignored','--nocapture','updater_native_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`native fixture exit ${code}`)));});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('UPDATER_NATIVE_UAT ')){const response=pending.shift(),result=JSON.parse(line.slice(19));result.error?response.reject(result.error):response.resolve(result.ok);}});
const send=request=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('native reply deadline')),10000);pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
let operationRevision=0;
const native=()=>({schema_version:1,requires_reopen:false,operation:{schema_version:1,revision:operationRevision,operation:null},preferences:{schema_version:1,revision:0,install_directory:null,launcher_summary_consent:false}});
const calls=[];
const desktop=mountDesktop(document,async(command,args)=>{
 if(command==='updater_command'){calls.push(args.request);return send(args.request);}
 if(command==='launcher_command')return native();
 if(command==='launch_command')return {schema_version:1,native:native(),installation_id:null,resources_available:false,launcher_update_required:false,observation:null};
 if(command==='migration_command')return {schema_version:1,native:native(),imported:null,preview:null};
 if(command==='install_command')return {schema_version:1,native:native(),install_supported:false,can_resume:false,can_reconcile:false,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null};
 throw Error(`unexpected command ${command}`);
});
const app=desktop.updater;
const get=id=>document.getElementById(id),click=id=>get(id).dispatchEvent(new window.Event('click')),tick=()=>new Promise(resolve=>setImmediate(resolve));
try{
 await app.ready;await tick();assert.equal(get('prepare-updater').disabled,true);
 click('check-updater');click('check-updater');await app.settled();await tick();assert.equal(calls.filter(x=>x.command==='check').length,1);
 assert.match(get('updater-status').textContent,/1.1.0 is available/);assert.equal(get('prepare-updater').disabled,false);
 assert.equal(document.getElementById('updater-notes').querySelectorAll('script').length,0);assert.match(get('updater-notes').textContent,/<script>/);
 const available=await send({command:'inspect'});
 // A separate native journal operation finishes while an offer is displayed.
 const advanced=await send({command:'advance_operation'});operationRevision=advanced.operation_revision;
 await desktop.play.refresh();await tick();await app.settled();await tick();

 click('prepare-updater');click('prepare-updater');await app.settled();await tick();assert.equal(calls.filter(x=>x.command==='prepare').length,1);
 assert.match(get('updater-status').textContent,/verified and saved/);assert.match(get('updater-status').textContent,/has not changed/);
 assert.equal(calls.find(x=>x.command==='prepare').operation_revision,operationRevision);
 const ready=await send({command:'inspect'});assert.equal(ready.phase,'ready');assert.ok(ready.revision>available.revision);
 assert.equal(JSON.stringify(ready).includes('http'),false);assert.equal('signature' in ready.offer,false);
 const reopened=await send({command:'reopen'});assert.equal(reopened.phase,'ready');assert.equal(reopened.revision,ready.revision);
 await app.refresh();await tick();assert.equal(get('prepare-updater').disabled,true);
 await assert.rejects(send(calls.find(x=>x.command==='prepare')),e=>e==='stale_revision');
 const tampered=await send({command:'tamper'});assert.equal(tampered.phase,'failed');assert.equal(tampered.failure,'signature');
 await app.refresh();await tick();assert.match(get('updater-status').textContent,/could not be verified/);assert.equal(get('prepare-updater').disabled,true);
 console.log('Native updater logic UAT passed: real local signed feed and HTTP artifact, one check/prepare per double click, composed operation-revision refresh without manual recheck, literal notes, native-only offer/bytes, verified disk persistence and reopen, stale replay refusal, tampered stage rejection.');
 console.log('Excluded: Tauri IPC/packaged visual UAT, real HTTPS release endpoint, Windows locks/durability, updater installation/relaunch/rollback, production keys or publication.');
}finally{await desktop.dispose();child.stdin.end();await exit;}
