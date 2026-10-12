// Logic UAT (AI-07): the Settings archive panel against a scripted native fake. Each
// step prints the status line the person sees; a wrong line fails the run. Nothing here
// reads a real archive; the engine tests cover the hash check and the file window.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountSeedArchive} from './.test-build/seed-archive-view.mjs';
const page=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const MB=1024*1024;
const native={schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},
 operation:{schema_version:1,revision:0,operation:null},
 preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}};
const idle=()=>({schema_version:1,native,chosen:null,verifying:null,last_error:null});
const chosen=()=>({...idle(),chosen:{file_name:'Stargate.rar',size:40*MB}});
const checking=megabytes=>({...idle(),verifying:{phase:'download',current:megabytes*MB,total:400*MB}});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));

const {document,window}=parseHTML(page);
const line=()=>document.getElementById('seed-archive-status').textContent;
const print=label=>console.log(`${label}: ${line()}`);
let plan=[];let refreshes=0;const calls=[];
// Choose and inspect each take the next scripted reply; forget clears the scripted file.
async function invoke(command,args){
 const request=args?.request?.command;
 if(command==='choose_seed_archive'){calls.push('choose');return plan.shift()??idle();}
 if(command!=='seed_archive_command')throw new Error(`unexpected command ${command}`);
 calls.push(request);
 if(request==='forget'){plan=[];return idle();}
 // The screen still shows the previous reply while the next poll waits.
 await sleep(20);if(plan.length)print('  progress');
 return plan.shift()??idle();
}
const context={snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{refreshes++;}};
const press=id=>document.getElementById(id).dispatchEvent(new window.Event('click'));
const app=mountSeedArchive(document,invoke,context,1);
const settle=async()=>{await app.settled();await sleep(20);};
try{
 await app.ready;await sleep(20);print('start');
 // A good file: choose, verify with progress, succeed.
 plan=[checking(100),checking(300),chosen()];
 press('seed-archive-choose');await settle();print('success');
 assert.match(line(),/^Using Stargate\.rar \(verified\)/);
 // A file that does not match the signed release: the earlier choice stays, the error says so.
 plan=[{...chosen(),last_error:'hash_mismatch'}];
 press('seed-archive-choose');await settle();print('hash_mismatch');
 assert.match(line(),/contents do not match the signed release, so it was not used/);
 // Choose the good file again, then forget it.
 plan=[chosen()];
 press('seed-archive-choose');await settle();print('chosen again');
 press('seed-archive-forget');await settle();print('forget');
 assert.match(line(),/^Already have the game archive/);
 assert.equal(calls.filter(c=>c==='choose').length,3);
 assert.equal(calls.filter(c=>c==='forget').length,1);
 console.log(`seed-archive UAT passed (Settings refreshed ${refreshes} times)`);
}finally{await app.dispose();}
