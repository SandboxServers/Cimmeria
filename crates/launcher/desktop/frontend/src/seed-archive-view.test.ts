import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountSeedArchive, seedArchiveText } from './seed-archive-view';
import { mountInstall } from './install-view';
import { makePanelContext } from './panel-context';
import type { SeedArchiveStatus } from './contract';
import type { InstallStatus } from './install-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const MB=1024*1024;
const id='d539d049-61b7-4c82-b3d7-cb9b7a991adc';
const native:SeedArchiveStatus['native']={schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},
  operation:{schema_version:1,revision:0,operation:null},
  preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}};
const idle=():SeedArchiveStatus=>({schema_version:1,native,chosen:null,offer:null,verifying:null,last_error:null});
const chosen=():SeedArchiveStatus=>({...idle(),chosen:{file_name:'Stargate.rar',size:40*MB}});
const offered=():SeedArchiveStatus=>({...idle(),offer:{file_name:'Stargate.rar',size:40*MB}});
const checking=(current:number):SeedArchiveStatus=>({...idle(),verifying:{phase:'download',current,total:400*MB}});
const installIdle=():InstallStatus=>({schema_version:1,install_supported:true,can_resume:true,can_reconcile:true,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null,native:{schema_version:1,
  requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},operation:{schema_version:1,revision:0,operation:null},preferences:{schema_version:2,revision:1,
    install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}}});
const using='Using Stargate.rar (verified). The launcher copies it when it needs the game files and never changes or deletes your file.';
const panelContext=(refresh?:()=>Promise<unknown>)=>makePanelContext(refresh).context;
const requestOf=(args?:Record<string,unknown>)=>(args!.request as {command:string}).command;
function dom(){const {document,window}=parseHTML(html);return {document:document as unknown as Document,
  get:(id:string)=>document.getElementById(id)! as unknown as HTMLButtonElement,
  text:(id:string)=>document.getElementById(id)!.textContent!,
  click:(id:string)=>document.getElementById(id)!.dispatchEvent(new window.Event('click'))};}
async function until(condition:()=>boolean) {
  for(let i=0;i<400&&!condition();i++) await new Promise(resolve=>setTimeout(resolve,5));
  assert.ok(condition(),'the launcher never reached the expected state');
}

test('ready content hides main archive discovery while Settings keeps the offer',{timeout:5000},async()=>{
 const ui=dom();const invoke=async(command:string)=>command==='install_command'?installIdle():offered();
 const settings=mountSeedArchive(ui.document,invoke,panelContext(),1);
 const install=mountInstall(ui.document,invoke,()=>id);
 try{
  await settings.ready;await install.ready;
  await until(()=>!ui.get('install-seed-archive-accept').hidden);
  install.setPlayable(true);
  for(const name of ['install-seed-archive','install-seed-archive-offer','install-seed-archive-accept','install-seed-archive-status'])
   assert.equal(ui.get(name).hidden,true,name);
  assert.equal(ui.get('seed-archive-accept').hidden,false,'Settings retains explicit archive access');
  install.setPlayable(false);assert.equal(ui.get('install-seed-archive-accept').hidden,false);
 }finally{await install.dispose();await settings.dispose();}
});

test('every archive error code has its own text',()=>{
  const expected:[string,RegExp][]=[
    ['not_a_file',/not a file the launcher can read/],
    ['size_mismatch',/size does not match the signed release/],
    ['hash_mismatch',/contents do not match the signed release/],
    ['changed',/changed after it was checked/],
    ['cancelled',/The check was cancelled/],
    ['busy',/Another install, repair, update or archive check is running/],
    ['io',/could not read that file/],
    ['release_unavailable',/could not reach the signed release/],
    ['transport',/could not confirm the archive check/],
    ['schema',/could not confirm the archive check/],
  ];
  for(const [error,text] of expected) assert.match(seedArchiveText({status:null,phase:null,cancelling:false,error}),text,error);
});

test('the first press shows busy before native answers',{timeout:5000},async()=>{
  const ui=dom();let open!:(s:SeedArchiveStatus)=>void;
  const gate=new Promise<SeedArchiveStatus>(resolve=>{open=resolve;});
  const app=mountSeedArchive(ui.document,async command=>command==='choose_seed_archive'?gate:idle(),panelContext(),1);
  try{await app.ready;ui.click('seed-archive-choose');
    assert.equal(ui.get('seed-archive-choose').disabled,true);assert.equal(ui.get('seed-archive-choose').textContent,'Choosing…');
    assert.equal(ui.get('seed-archive-choose').getAttribute('aria-busy'),'true');
    assert.equal(ui.text('seed-archive-status'),'Choose your archive in the file window…');
  }finally{open(idle());await app.dispose();}
});

test('verifying shows progress until the check ends in a chosen file',{timeout:5000},async()=>{
  const ui=dom();const seen:string[]=[];let checks=0;let started=false;
  const app=mountSeedArchive(ui.document,async(command,args)=>{
    if(command==='choose_seed_archive'){started=true;return checking(100*MB);}
    if(requestOf(args)!=='inspect')return idle();
    // The mount-time inspect runs before any choose, so it is not a verification poll.
    if(!started)return idle();
    // The screen catches up with the previous reply a moment after it arrives.
    await new Promise(resolve=>setTimeout(resolve,20));
    seen.push(ui.text('seed-archive-status'));checks++;
    return checks===1?checking(200*MB):chosen();
  },panelContext(),1);
  try{await app.ready;ui.click('seed-archive-choose');await app.settled();
    assert.deepEqual(seen.slice(0,2),['Checking your archive: 100 of 400 MB.','Checking your archive: 200 of 400 MB.']);
    assert.equal(ui.text('seed-archive-status'),using);assert.equal(ui.text('seed-archive-file'),'Stargate.rar, 40 MB');
  }finally{await app.dispose();}
});

test('a check whose first reply has no progress yet is followed to its error',{timeout:5000},async()=>{
  const ui=dom();let started=false;let polls=0;
  const app=mountSeedArchive(ui.document,async(command,args)=>{
    if(command==='choose_seed_archive'){started=true;return idle();}
    if(requestOf(args)!=='inspect'||!started)return idle();
    polls++;
    // The reply to choose has no report yet; the next poll shows the check failed.
    return polls===1?{...idle(),last_error:'hash_mismatch'}:idle();
  },panelContext(),1);
  try{await app.ready;ui.click('seed-archive-choose');await app.settled();
    assert.equal(polls,1);
    assert.match(ui.text('seed-archive-status'),/contents do not match the signed release/);
  }finally{await app.dispose();}
});

test('cancel asks native once and shows that nothing changed',{timeout:5000},async()=>{
  const ui=dom();const calls:string[]=[];let cancelled=false;let started=false;
  const app=mountSeedArchive(ui.document,async(command,args)=>{
    if(command==='choose_seed_archive'){started=true;return checking(MB);}
    const request=requestOf(args);
    // The mount-time inspect runs before any choose, so it is not a verification poll.
    if(!started)return idle();
    calls.push(request);
    if(request==='cancel'){cancelled=true;return checking(MB);}
    return cancelled?{...idle(),last_error:'cancelled'}:checking(MB);
  },panelContext(),1);
  try{await app.ready;ui.click('seed-archive-choose');const choosing=app.settled();
    await until(()=>!ui.get('seed-archive-cancel').hidden);
    ui.click('seed-archive-cancel');await choosing;await app.settled();
    assert.deepEqual(calls.filter(c=>c==='cancel'),['cancel']);
    assert.equal(ui.text('seed-archive-status'),'The check was cancelled. Nothing was changed.');
    assert.equal(ui.get('seed-archive-cancel').hidden,true);
  }finally{await app.dispose();}
});

test('forget clears the chosen file and refreshes Settings once',{timeout:5000},async()=>{
  const ui=dom();const calls:string[]=[];let forgotten=false;let refreshes=0;
  const app=mountSeedArchive(ui.document,async(command,args)=>{
    if(command!=='seed_archive_command')return idle();
    const request=requestOf(args);calls.push(request);
    if(request==='forget')forgotten=true;
    return forgotten?idle():chosen();
  },panelContext(async()=>{refreshes++;}),1);
  try{await app.ready;
    assert.equal(ui.get('seed-archive-forget').hidden,false);assert.equal(ui.text('seed-archive-file'),'Stargate.rar, 40 MB');
    ui.click('seed-archive-forget');await app.settled();
    assert.deepEqual(calls,['inspect','forget']);assert.equal(refreshes,1);
    assert.equal(ui.get('seed-archive-forget').hidden,true);assert.equal(ui.text('seed-archive-file'),'');
    assert.match(ui.text('seed-archive-status'),/^Already have the game archive/);
  }finally{await app.dispose();}
});

test('hash_mismatch says the file was not used and shows no chosen file',{timeout:5000},async()=>{
  const ui=dom();
  const app=mountSeedArchive(ui.document,async command=>command==='choose_seed_archive'?{...idle(),last_error:'hash_mismatch'}:idle(),panelContext(),1);
  try{await app.ready;ui.click('seed-archive-choose');await app.settled();
    assert.match(ui.text('seed-archive-status'),/contents do not match the signed release, so it was not used/);
    assert.equal(ui.text('seed-archive-file'),'');assert.doesNotMatch(ui.text('seed-archive-status'),/^Using/);
  }finally{await app.dispose();}
});

test('a Downloads offer asks first; Yes sends accept_offer and shows busy at once',{timeout:5000},async()=>{
  const ui=dom();const sent:string[]=[];let done:SeedArchiveStatus|null=null;let open!:(s:SeedArchiveStatus)=>void;
  const gate=new Promise<SeedArchiveStatus>(resolve=>{open=resolve;});
  const invoke=async(command:string,args?:Record<string,unknown>)=>{
    if(command!=='seed_archive_command')return idle();
    sent.push(requestOf(args));
    return requestOf(args)==='accept_offer'?gate:(done??offered());
  };
  const app=mountSeedArchive(ui.document,invoke,panelContext(),1);
  try{await app.ready;
    assert.equal(ui.text('seed-archive-offer'),'Found Stargate.rar in Downloads. Use it?');
    assert.equal(ui.get('seed-archive-accept').hidden,false);
    ui.click('seed-archive-accept');
    await until(()=>ui.get('seed-archive-accept').disabled);
    assert.equal(ui.get('seed-archive-accept').textContent,'Checking…');
    assert.equal(sent[sent.length-1],'accept_offer');
    // The gate first reports a running check; the next poll reads the final chosen state.
    done=chosen();open(checking(300*MB));await app.settled();
    assert.equal(ui.get('seed-archive-accept').hidden,true);
    assert.equal(ui.text('seed-archive-status'),using);
  }finally{open(idle());await app.dispose();}
});

test('a choice made in the install area shows in Settings at once',{timeout:5000},async()=>{
  const ui=dom();let done:SeedArchiveStatus|null=null;let open!:(s:SeedArchiveStatus)=>void;
  const gate=new Promise<SeedArchiveStatus>(resolve=>{open=resolve;});
  const invoke=async(command:string)=>command==='choose_seed_archive'?gate:command==='install_command'?installIdle():(done??idle());
  const settings=mountSeedArchive(ui.document,invoke,panelContext(),1);
  const install=mountInstall(ui.document,invoke,()=>id);
  try{await settings.ready;await install.ready;
    ui.click('install-seed-archive');
    await until(()=>ui.get('seed-archive-choose').disabled);
    assert.equal(ui.text('seed-archive-status'),'Choose your archive in the file window…','Settings sees the press from the install area');
    done=chosen();open(checking(300*MB));
    await until(()=>ui.text('install-seed-archive-status')===using);
    assert.equal(ui.text('seed-archive-status'),using);assert.equal(ui.text('seed-archive-file'),'Stargate.rar, 40 MB');
    assert.equal(ui.get('seed-archive-forget').hidden,false);
  }finally{open(idle());await install.dispose();await settings.dispose();}
});

test('the install area asks the same Downloads question, and its Yes accepts the offer',{timeout:5000},async()=>{
  const ui=dom();const sent:string[]=[];let done:SeedArchiveStatus|null=null;let open!:(s:SeedArchiveStatus)=>void;
  const gate=new Promise<SeedArchiveStatus>(resolve=>{open=resolve;});
  const invoke=async(command:string,args?:Record<string,unknown>)=>{
    if(command==='install_command')return installIdle();
    if(command!=='seed_archive_command')return idle();
    sent.push(requestOf(args));
    return requestOf(args)==='accept_offer'?gate:(done??offered());
  };
  // Settings starts the shared status check, as it does in the app; the install area shows its result.
  const settings=mountSeedArchive(ui.document,invoke,panelContext(),1);
  const install=mountInstall(ui.document,invoke,()=>id);
  try{await settings.ready;await install.ready;
    assert.equal(ui.text('install-seed-archive-offer'),'Found Stargate.rar in Downloads. Use it?');
    assert.equal(ui.get('install-seed-archive-accept').hidden,false);
    ui.click('install-seed-archive-accept');
    await until(()=>ui.get('install-seed-archive-accept').disabled);
    assert.equal(sent[sent.length-1],'accept_offer');
    done=chosen();open(checking(300*MB));
    await until(()=>ui.text('install-seed-archive-status')===using);
    assert.equal(ui.get('install-seed-archive-accept').hidden,true);
  }finally{open(idle());await install.dispose();await settings.dispose();}
});
