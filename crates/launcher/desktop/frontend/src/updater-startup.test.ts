import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {mountDesktop} from './composition';
import type {UpdaterRequest,UpdaterStatus} from './updater-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const native={schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'1.0.0',tag:null},preferences:{schema_version:2,revision:0,install_directory:String.raw`\\?\C:\Temp\sgw`,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1,revision:0,operation:null}};
const initial=():UpdaterStatus=>({schema_version:1,revision:0,operation_revision:0,phase:'idle',offer:null,failure:null,requires_reopen:false});
const offer={id:'offer',version:'1.1.0',notes:'A signed update'};
const tick=()=>new Promise<void>(resolve=>setTimeout(resolve,10));
function fixture(reply:(request:UpdaterRequest)=>Promise<UpdaterStatus>){
 const {document,window}=parseHTML(html);const calls:UpdaterRequest[]=[];let plays=0;
 const app=mountDesktop(document,async(command,args)=>{
  if(command==='updater_command'){const request=args!.request as UpdaterRequest;calls.push(request);return reply(request);}
  if(command==='launcher_command')return native;
  if(command==='launch_command'){
   if((args!.request as {command:string}).command==='play')plays++;
   return {schema_version:1,native,installation_id:'owner',setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:0,ops_skipped:0,observation:null};
  }
  return native; // Unrelated panels refuse these fixtures through their strict decoders.
 });
 const get=(id:string)=>document.getElementById(id)! as HTMLButtonElement;
 const settle=async()=>{await app.updater.ready;await app.updater.settled();await app.play.settled();await tick();};
 return {app,calls,get,settle,plays:()=>plays,click:(id:string)=>get(id).dispatchEvent(new window.Event('click'))};
}

test('startup holds Play, checks once, then one main-page click verifies and applies exactly once',async()=>{
 let saved=initial(),finishCheck!:()=>void,finishPrepare!:()=>void;
 const check=new Promise<void>(resolve=>{finishCheck=resolve;}),prepare=new Promise<void>(resolve=>{finishPrepare=resolve;});
 const ui=fixture(async request=>{
  if(request.command==='check'){await check;saved={...saved,revision:1,phase:'available',offer};}
  if(request.command==='prepare'){assert.equal(request.revision,1);await prepare;saved={...saved,revision:2,phase:'ready'};}
  if(request.command==='apply'){assert.equal(request.revision,2);saved={...saved,revision:3,phase:'restart_required'};}
  return saved;
 });
 try{
  await ui.app.play.ready;await tick();assert.equal(ui.get('launch').disabled,true);
  ui.click('launch');assert.equal(ui.plays(),0);finishCheck();await ui.settle();
  assert.equal(ui.calls.filter(r=>r.command==='check').length,1);
  assert.equal(ui.get('update-launcher').hidden,false);assert.equal(ui.get('update-launcher').textContent,'Update and restart');
  assert.match(ui.get('launch-status').textContent!,/launcher update above/);
  assert.equal(ui.get('gear').hidden,true,'action is on the main page');
  assert.equal(ui.get('inspect-launch').hidden,true,'content setup stays complete while the launcher needs updating');
  assert.equal(ui.get('install-seed-archive').hidden,true,'composition shares native Play readiness with installation');
  await ui.app.play.refresh();await tick();assert.equal(ui.get('launch').disabled,true,'Play polling cannot clear updater gate');
  ui.click('update-launcher');assert.equal(ui.get('update-launcher').textContent,'Updating launcher…');
  ui.click('update-launcher');await tick();assert.equal(ui.calls.filter(r=>r.command==='prepare').length,1);
  assert.equal(ui.calls.filter(r=>r.command==='apply').length,0);finishPrepare();await ui.settle();
  assert.deepEqual(ui.calls.filter(r=>r.command!=='inspect').map(r=>r.command),['check','prepare','apply']);
  assert.equal(ui.get('launch').disabled,true);ui.click('launch');assert.equal(ui.plays(),0);
 }finally{finishCheck();finishPrepare();await ui.app.dispose();}
});

for(const mode of ['up_to_date','disabled','offline'] as const)test(`${mode} startup allows Play when no update is known`,async()=>{
 let saved=mode==='disabled'?{...initial(),phase:'disabled' as const,failure:'disabled'}:initial();
 const ui=fixture(async request=>{if(request.command==='check'){if(mode==='offline'){saved={...saved,phase:'failed',failure:'feed'};throw 'feed';}saved={...saved,phase:'up_to_date'};}return saved;});
 try{await ui.settle();assert.equal(ui.get('launch').disabled,false);assert.equal(ui.calls.filter(r=>r.command==='check').length,mode==='disabled'?0:1);
  if(mode==='offline')assert.match(ui.get('launcher-update-status').textContent!,/could not be confirmed/);
 }finally{await ui.app.dispose();}
});

for(const lost of ['prepare','apply'] as const)test(`lost ${lost} reply stops the chain and keeps Play blocked`,async()=>{
 let saved={...initial(),phase:'available' as UpdaterStatus['phase'],offer};
 const ui=fixture(async request=>{
  if(request.command==='prepare'){saved={...saved,phase:'ready'};if(lost==='prepare')throw 'transport';}
  if(request.command==='apply'){saved={...saved,phase:'restart_required'};if(lost==='apply')throw 'transport';}
  return saved;
 });
 try{await ui.settle();ui.click('update-launcher');await ui.settle();
  assert.equal(ui.calls.filter(r=>r.command==='apply').length,lost==='prepare'?0:1);
  assert.equal(ui.get('launch').disabled,true);await ui.app.updater.refresh();await ui.app.play.refresh();await tick();assert.equal(ui.get('launch').disabled,true);
  assert.equal(ui.calls.filter(r=>r.command==='apply').length,lost==='prepare'?0:1);
 }finally{await ui.app.dispose();}
});

test('verified package on reopening applies on one click without downloading or checking again',async()=>{
 let saved={...initial(),phase:'ready' as UpdaterStatus['phase'],offer};
 const ui=fixture(async request=>{if(request.command==='apply')saved={...saved,phase:'restart_required'};return saved;});
 try{await ui.settle();assert.equal(ui.get('launch').disabled,true);ui.click('update-launcher');await ui.settle();
  assert.deepEqual(ui.calls.filter(r=>r.command!=='inspect').map(r=>r.command),['apply']);
 }finally{await ui.app.dispose();}
});

for(const failure of ['feed','signature'] as const)test(`${failure} failure with known offer cannot enable Play or apply an unverified package`,async()=>{
 let saved={...initial(),phase:'available' as UpdaterStatus['phase'],offer};
 const ui=fixture(async request=>{
  if(request.command==='check'&&failure==='feed'){saved={...saved,phase:'failed',failure};throw failure;}
  if(request.command==='prepare'&&failure==='signature'){saved={...saved,phase:'failed',failure};}
  return saved;
 });
 try{await ui.settle();assert.equal(ui.get('launch').disabled,true);ui.click('update-launcher');await ui.settle();
  assert.equal(ui.calls.filter(r=>r.command==='apply').length,0);
  await ui.app.updater.refresh();await ui.app.play.refresh();await tick();assert.equal(ui.get('launch').disabled,true);
  assert.equal(ui.get('update-launcher').textContent,'Update and restart');
 }finally{await ui.app.dispose();}
});
test('failed initial inspection offers an enabled main-page retry and holds Play until it succeeds',async()=>{
 let fail=true,saved=initial();
 const ui=fixture(async request=>{
  if(fail){fail=false;throw 'transport';}
  if(request.command==='check')saved={...saved,phase:'up_to_date'};
  return saved;
 });
 try{await ui.settle();assert.equal(ui.get('launch').disabled,true);assert.equal(ui.get('update-launcher').hidden,false);assert.equal(ui.get('update-launcher').disabled,false);
  ui.click('update-launcher');await ui.settle();assert.equal(ui.calls.filter(r=>r.command==='check').length,1);assert.equal(ui.get('launch').disabled,false);
 }finally{await ui.app.dispose();}
});

for(const failure of ['inspection','check'] as const)test(`retry after ${failure} failure only checks; a discovered offer requires its own Update and restart press`,async()=>{
 let saved=initial(),fail=true;
 const ui=fixture(async request=>{
  if(fail&&request.command===(failure==='inspection'?'inspect':'check')){fail=false;throw 'transport';}
  if(request.command==='check')saved={...saved,phase:'available',offer};
  if(request.command==='prepare')saved={...saved,phase:'ready'};
  if(request.command==='apply')saved={...saved,phase:'restart_required'};
  return saved;
 });
 try{
  await ui.settle();assert.equal(ui.get('update-launcher').textContent,'Check for updates');
  ui.click('update-launcher');await ui.settle();assert.equal(ui.get('update-launcher').textContent,'Update and restart');
  assert.equal(ui.calls.filter(r=>r.command==='prepare'||r.command==='apply').length,0);
  assert.equal(ui.get('launch').disabled,true);
  ui.click('update-launcher');await ui.settle();
  assert.equal(ui.calls.filter(r=>r.command==='prepare').length,1);assert.equal(ui.calls.filter(r=>r.command==='apply').length,1);
 }finally{await ui.app.dispose();}
});
