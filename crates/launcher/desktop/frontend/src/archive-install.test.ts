import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountInstall} from './install-view';
import {mountSeedArchive} from './seed-archive-view';
import {makePanelContext} from './panel-context';
import type {SeedArchiveStatus,SeedArchiveCommand} from './contract';
import type {InstallRequest,InstallStatus} from './install-workflow';

const html=readFileSync('ui/index.html','utf8');
const id='d539d049-61b7-4c82-b3d7-cb9b7a991adc';
async function fixture(options:{mode?:'success'|'cancel'|'hash'|'transport';previous?:boolean;offer?:boolean;blocked?:boolean}={}) {
 const {document,window}=parseHTML(html);
 let verified=!!options.previous;let selected=false;let installs=0;let choices=0;let checks=0;
 let finish!:()=>void;let checked!:()=>void;
 const gate=new Promise<void>(resolve=>{finish=resolve;});
 const checking=new Promise<void>(resolve=>{checked=resolve;});
 let status:InstallStatus={schema_version:1,install_supported:true,can_resume:false,can_reconcile:false,can_retry:false,
  uninstall:null,runtime_setup:null,progress:null,outcome:null,native:{schema_version:1,requires_reopen:!!options.blocked,
   adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},operation:{schema_version:1,revision:0,operation:null},
   preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}}};
 const archive=():SeedArchiveStatus=>({schema_version:1,native:status.native,chosen:verified?{file_name:'original.rar',size:4000}:null,
  offer:options.offer&&!verified?{file_name:'original.rar',size:4000}:null,verifying:null,last_error:null});
 const choose=()=>{
  choices++;
  if(options.mode==='transport')throw 'transport';
  if(options.mode==='cancel')return {...archive(),picker_cancelled:true as const};
  if(options.mode==='hash')return {...archive(),last_error:'hash_mismatch' as const};
  selected=true;return {...archive(),verifying:{phase:'download' as const,current:0,total:4000}};
 };
 const invoke=async(command:string,args?:Record<string,unknown>)=>{
  if(command==='choose_seed_archive')return choose();
  const request=args?.request as InstallRequest|SeedArchiveCommand;
  if(command==='install_command'){
   if(request.command==='install'){
    installs++;
    assert.equal(verified,true,'native archive verification must finish before installation');
    assert.equal(request.preferences_revision,2,'Install uses the refreshed native revision');
    status={...status,progress:{phase:'download',current:0,total:4000},native:{...status.native,
     operation:{schema_version:1,revision:1,operation:{id,kind:'install',state:'running',intent_digest:Array(32).fill(0)}}}};
   }
   return status;
  }
  assert.equal(command,'seed_archive_command');
  if(request.command==='accept_offer')return choose();
  if(selected){
   checks++;checked();await gate;
   verified=true;status={...status,native:{...status.native,preferences:{...status.native.preferences,revision:2}}};
  }
  return archive();
 };
 let app!:ReturnType<typeof mountInstall>;
 const settings=mountSeedArchive(document as unknown as Document,invoke,makePanelContext().context,1,()=>app.installFromArchive());
 app=mountInstall(document as unknown as Document,invoke,()=>id);
 await Promise.all([app.ready,settings.ready]);
 const get=(name:string)=>document.getElementById(name)! as unknown as HTMLButtonElement;
 return {app,settings,get,press:(name:string)=>get(name).dispatchEvent(new window.Event('click')),finish,checking,
  counts:()=>({installs,choices,checks}),close:async()=>{finish();await app.dispose();await settings.dispose();}};
}

test('choosing an archive verifies first then starts one install without another press',async()=>{
 const p=await fixture();
 try{
  p.press('install-seed-archive');p.press('install-seed-archive');
  assert.equal(p.get('install-seed-archive').disabled,true,'visible busy state on first press');
  await p.checking;
  assert.equal(p.counts().installs,0,'hash verification is still in progress');
  p.finish();await p.app.settled();
  assert.equal(p.counts().choices,1);assert.equal(p.counts().installs,1);
  assert.equal(p.get('install').textContent,'Installing…');
 }finally{await p.close();}
});

test('Settings selection and accepting a Downloads offer use the same verified handoff',async()=>{
 for(const button of ['seed-archive-choose','seed-archive-accept','install-seed-archive-accept']){
  const p=await fixture({offer:button.includes('accept')});
  try{
   p.press(button);await p.checking;assert.equal(p.counts().installs,0);
   p.finish();await p.settings.settled();await p.app.settled();
   assert.equal(p.counts().installs,1,button);
  }finally{await p.close();}
 }
});

test('cancel, verification failure and lost replies never start installation, including an old selection',async()=>{
 for(const mode of ['cancel','hash','transport'] as const){
  for(const previous of [true,false]){
   const p=await fixture({mode,previous});
   try{
    p.press('install-seed-archive');await p.app.settled();
    assert.equal(p.counts().installs,0,`${mode}, previous=${previous}`);
    assert.equal(p.counts().checks,0,'cancelled picker is not polled into the previous successful choice');
    if(mode==='cancel')assert.match(p.get('install-seed-archive-status').textContent!,/selection cancelled/);
   }finally{await p.close();}
  }
 }
});

test('a remembered archive on reopen does not start installation',async()=>{
 const p=await fixture({previous:true});
 try{await p.app.settled();assert.deepEqual(p.counts(),{installs:0,choices:0,checks:0});}
 finally{await p.close();}
});

test('playable and blocked states do not start an automatic installation',async()=>{
 for(const playable of [true,false]){
  const p=await fixture({blocked:!playable});
  try{
   p.app.setPlayable(playable);p.press('seed-archive-choose');await p.checking;
   p.finish();await p.settings.settled();await p.app.settled();
   assert.equal(p.counts().installs,0);
  }finally{await p.close();}
 }
});

test('a verification reply arriving after disposal cannot start installation',async()=>{
 const p=await fixture();
 try{
  p.press('install-seed-archive');await p.checking;
  await p.app.dispose();p.finish();await p.app.settled();
  assert.equal(p.counts().installs,0);
 }finally{await p.close();}
});
