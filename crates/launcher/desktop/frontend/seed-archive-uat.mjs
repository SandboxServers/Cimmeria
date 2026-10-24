// Logic UAT (AI-07): the Settings archive panel against a scripted native fake. Each
// step prints the status line the person sees; a wrong line fails the run. Nothing here
// reads a real archive; the engine tests cover the hash check and the file window.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountSeedArchive} from './.test-build/seed-archive-view.mjs';
import {mountInstall} from './.test-build/install-view.mjs';
const page=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const MB=1024*1024;
const native={schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed'],launcher:{version:'0.1.0',tag:null},
 operation:{schema_version:1,revision:0,operation:null},
 preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed',width:1024,height:768}}};
const idle=()=>({schema_version:1,native,chosen:null,offer:null,verifying:null,last_error:null});
const chosen=()=>({...idle(),chosen:{file_name:'Stargate.rar',size:40*MB}});
const offered=()=>({...idle(),offer:{file_name:'Stargate.rar',size:40*MB}});
const checking=megabytes=>({...idle(),verifying:{phase:'download',current:megabytes*MB,total:400*MB}});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));

const {document,window}=parseHTML(page);
const line=()=>document.getElementById('seed-archive-status').textContent;
const print=label=>console.log(`${label}: ${line()}`);
let plan=[];let refreshes=0;const calls=[];
// Choose and inspect each take the next scripted reply; forget clears the scripted file.
async function invoke(command,args){
 const request=args?.request?.command;
 if(command==='choose_seed_archive'){calls.push('choose');const next=plan.shift()??idle();if(typeof next==='string')throw next;return next;}
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
let active=app;
const settle=async()=>{await app.settled();await sleep(20);};
try{
 await app.ready;await sleep(20);print('start');
 // A good file: choose (the check has no report yet), verify with progress, succeed.
 plan=[idle(),checking(300),chosen()];
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
 // A development build cannot authenticate the public game catalog. The RAR
 // remains unselected; the UI must identify the key rather than the network.
 plan=['signing_key_unavailable'];
 press('seed-archive-choose');await settle();print('development key');
 assert.match(line(),/published launcher ZIP.*release public verification key.*default test key/);
 assert.match(line(),/archive was not checked or changed/);
 assert.doesNotMatch(line(),/could not reach|Download it again/);
 assert.equal(document.getElementById('seed-archive-forget').hidden,true);
 plan=[{...idle(),last_error:'release_verification_failed'}];
 press('seed-archive-choose');await settle();print('catalog verification');
 assert.match(line(),/signed game release failed verification/);
 assert.doesNotMatch(line(),/could not reach/);
 // A matching file in Downloads is offered on mount; Yes verifies it, then it is used.
 await app.dispose();
 plan=[offered()];
 active=mountSeedArchive(document,invoke,context,1);await active.ready;print('offer');
 assert.equal(document.getElementById('seed-archive-offer').textContent,'Found Stargate.rar in Downloads. Use it?');
 plan=[idle(),checking(300),chosen()];
 press('seed-archive-accept');await active.settled();await sleep(20);print('accepted');
 assert.match(line(),/^Using Stargate\.rar \(verified\)/);
 assert.equal(calls.filter(c=>c==='accept_offer').length,1);
 console.log(`seed-archive UAT passed (Settings refreshed ${refreshes} times)`);
 console.log('Development-key/catalog-error UAT passed: actionable build guidance, separate verification failure, no archive selected or silently accepted. Excluded: real file picker, native networking/key provisioning, real archive hashing and packaged UI.');
}finally{await active.dispose();}

// The real selection and installation views share persisted native archive state.
const handoff=parseHTML(page);let saved=false;let selected=false;let installs=0;
let finishVerification,verificationAsked;
const verificationGate=new Promise(resolve=>{finishVerification=resolve;});
const verificationStarted=new Promise(resolve=>{verificationAsked=resolve;});
const installNative={...native,preferences:{...native.preferences,revision:1,install_directory:'/fixture'}};
let installStatus={schema_version:1,install_supported:true,can_resume:false,can_reconcile:false,can_retry:false,
 uninstall:null,runtime_setup:null,progress:null,outcome:null,native:installNative};
const archiveStatus=()=>({...idle(),native:installNative,chosen:saved?chosen().chosen:null});
const handoffInvoke=async(command,args)=>{
 const request=args?.request;
 if(command==='install_command'){
  if(request.command==='install'){
   assert.equal(saved,true,'installation must read a native-verified archive');installs++;
   installStatus={...installStatus,progress:{phase:'extraction',current:0,total:4000},native:{...installNative,
    operation:{schema_version:1,revision:1,operation:{id:request.operation_id,kind:'install',state:'running',intent_digest:Array(32).fill(0)}}}};
  }
  return installStatus;
 }
 if(command==='choose_seed_archive'){
  selected=true;return {...archiveStatus(),verifying:{phase:'download',current:0,total:4000}};
 }
 assert.equal(command,'seed_archive_command');
 if(selected){verificationAsked();await verificationGate;saved=true;}
 return archiveStatus();
};
let installer;
const handoffSettings=mountSeedArchive(handoff.document,handoffInvoke,context,1,()=>installer.installFromArchive());
installer=mountInstall(handoff.document,handoffInvoke,()=> 'd539d049-61b7-4c82-b3d7-cb9b7a991adc');
try{
 await Promise.all([handoffSettings.ready,installer.ready]);
 const choose=handoff.document.getElementById('install-seed-archive');
 choose.click();choose.click();assert.equal(choose.disabled,true);
 await verificationStarted;assert.equal(installs,0);
 finishVerification();await installer.settled();
 assert.equal(saved,true);assert.equal(installs,1);
 assert.equal(handoff.document.getElementById('install').textContent,'Installing…');
 console.log('Archive-to-install UAT passed: first-press feedback; verification saves native archive state before a single automatic install; no second Install click. Excluded: real picker, real RAR hashing/extraction, native persistence and a physical game installation.');
}finally{finishVerification();await installer.dispose();await handoffSettings.dispose();}
