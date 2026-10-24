import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {Effect} from 'effect';
import {failureText} from './failure-copy';
import {mountPreflight} from './preflight-view';
import {PreflightFailure,type Check,type CheckId,type PreflightOperation,type PreflightWorkflow,type Report} from './preflight-workflow';
const html=readFileSync('ui/index.html','utf8');
const check=(over:Partial<Check>={}):Check=>({id:'rosetta',status:'missing',action:'auto_install',failure:'rosetta_missing',blocking:true,bytes_needed:null,bytes_free:null,download_bytes:null,...over});
const report=(checks:Check[],over:Partial<Report>={}):Report=>({schema_version:1,operation:'play',checks,ready:!checks.some(c=>c.blocking),...over});
const translocated=check({id:'app_location',status:'warning',action:'manual',failure:'app_translocated',blocking:false});
type Install=(operation:PreflightOperation,check:CheckId)=>Promise<Report>;
function mounted(install:Install=async()=>report([]),inspect:(operation:PreflightOperation)=>Promise<Report>=async()=>report([])) {
 const {document,window}=parseHTML(html);const calls:string[]=[];const continued:PreflightOperation[]=[];
 const workflow:PreflightWorkflow={
  inspectPreflight:operation=>{calls.push(`inspect:${operation}`);return Effect.tryPromise({try:()=>inspect(operation),catch:error=>error as PreflightFailure});},
  installDependency:(operation,id)=>{calls.push(`install:${operation}:${id}`);return Effect.tryPromise({try:()=>install(operation,id),catch:error=>error as PreflightFailure});}};
 const app=mountPreflight(document as unknown as Document,workflow,operation=>continued.push(operation));
 const get=(id:string)=>document.getElementById(id) as unknown as HTMLButtonElement;
 const press=(id:string)=>get(id).dispatchEvent(new window.Event('click'));
 const text=(id:string)=>document.getElementById(id)!.textContent??'';
 return {app,calls,continued,get,press,text};
}

test('the panel sits above the footer and starts hidden',()=>{
 const p=mounted();
 assert.equal(p.get('preflight-panel').hidden,true);
 assert.equal(p.get('preflight-panel').nextElementSibling?.tagName,'FOOTER');
 assert.equal(p.get('preflight-panel').getAttribute('aria-labelledby'),'preflight-title');
});

test('Rosetta missing shows the install line and button; the first press installs once and shows Installing…',async()=>{
 let answer!:(value:Report)=>void;
 const p=mounted(()=>new Promise(resolve=>{answer=resolve;}));
 try{
  p.app.show(report([check()]));
  assert.equal(p.get('preflight-panel').hidden,false);
  assert.match(p.text('preflight-checks'),/Rosetta 2 is not installed/);
  assert.match(p.text('preflight-checks'),/The launcher will install Rosetta 2 from Apple\. macOS will ask for your administrator password\./);
  assert.equal(p.get('preflight-install-rosetta').textContent,'Install Rosetta 2');
  assert.equal(p.get('preflight-continue').hidden,true,'a blocking report offers no Continue');
  p.press('preflight-install-rosetta');p.press('preflight-install-rosetta');
  assert.deepEqual(p.calls,['install:play:rosetta'],'a double press installs once');
  assert.equal(p.get('preflight-install-rosetta').textContent,'Installing…');
  assert.equal(p.get('preflight-install-rosetta').disabled,true);
  assert.equal(p.get('preflight-install-rosetta').getAttribute('aria-busy'),'true');
  assert.equal(p.get('preflight-recheck').disabled,true);
  answer(report([]));await p.app.settled();
  assert.equal(p.get('preflight-install-rosetta'),null,'the returned report has no Rosetta row');
  assert.equal(p.text('preflight-summary'),'Everything needed to play is ready.');
 }finally{await p.app.dispose();}
});

test('a refused install shows the mapped failure copy and re-enables the button',async()=>{
 const p=mounted(async()=>{throw new PreflightFailure({code:'operation_busy'});});
 try{
  p.app.show(report([check()]));p.press('preflight-install-rosetta');await p.app.settled();
  const copy=failureText('operation_busy');
  assert.equal(p.text('preflight-status'),`${copy.message}\n${copy.fix}`);
  assert.equal(p.get('preflight-install-rosetta').disabled,false);
  assert.equal(p.get('preflight-install-rosetta').textContent,'Install Rosetta 2');
 }finally{await p.app.dispose();}
});

test('a ready report with a translocation warning offers Continue, which hides the panel and hands back the operation',async()=>{
 const p=mounted();
 try{
  p.app.show(report([translocated]));
  assert.equal(p.get('preflight-continue').hidden,false);
  assert.match(p.text('preflight-checks'),/Move the launcher to your Applications folder and open it from there\. You can continue\./);
  p.press('preflight-continue');
  assert.equal(p.get('preflight-panel').hidden,true);
  assert.deepEqual(p.continued,['play']);
 }finally{await p.app.dispose();}
});

test('a disk row shows both sizes',async()=>{
 const p=mounted();
 try{
  p.app.show(report([check({id:'disk_space',status:'missing',action:'manual',failure:'disk_space_low',blocking:true,bytes_needed:14e9,bytes_free:3.2e9})],{operation:'install'}));
  assert.match(p.text('preflight-checks'),/about 14\.0 GB is needed and 3\.2 GB is free/);
  assert.equal(p.text('preflight-summary'),'Fix these before you install:');
 }finally{await p.app.dispose();}
});

test('the Wine runtime row: Download now for Play, no button for Install',async()=>{
 const wine=check({id:'wine_runtime',action:'download',failure:'wine_runtime_missing',download_bytes:63_744_856});
 const p=mounted();
 try{
  p.app.show(report([wine]));
  assert.equal(p.get('preflight-install-wine_runtime').textContent,'Download now');
  p.press('preflight-install-wine_runtime');await p.app.settled();
  assert.deepEqual(p.calls,['install:play:wine_runtime']);
  p.app.show(report([{...wine,blocking:false}],{operation:'install'}));
  assert.equal(p.get('preflight-install-wine_runtime'),null);
  assert.match(p.text('preflight-checks'),/during the install/);
 }finally{await p.app.dispose();}
});

test('Choose game folder opens Settings, and Check again re-inspects with visible feedback',async()=>{
 let answer!:(value:Report)=>void;
 const p=mounted(undefined,()=>new Promise(resolve=>{answer=resolve;}));
 try{
  p.app.show(report([check({id:'game_folder',action:'ask_user',failure:'game_folder_not_chosen'}),check({status:'unknown',action:'ask_user',failure:'rosetta_check_timed_out',blocking:false})],{operation:'repair'}));
  assert.match(p.text('preflight-checks'),/Choose a game folder in Settings\./);
  assert.match(p.text('preflight-checks'),/Press Check again\./);
  p.press('preflight-choose-folder');
  assert.equal(p.get('gear').hidden,false);assert.equal(p.get('play').hidden,true);
  assert.equal(p.get('settings').getAttribute('aria-expanded'),'true');
  p.press('preflight-recheck');
  assert.equal(p.get('preflight-recheck').textContent,'Checking…');assert.equal(p.get('preflight-recheck').disabled,true);
  assert.deepEqual(p.calls,['inspect:repair']);
  answer(report([],{operation:'repair'}));await p.app.settled();
  assert.equal(p.get('preflight-recheck').textContent,'Check again');
  assert.equal(p.text('preflight-summary'),'Everything needed to repair is ready.');
 }finally{await p.app.dispose();}
});

test('a ready report whose open checks are all non-blocking offers Continue, even with no warning row',async()=>{
 const p=mounted();
 try{
  p.app.show(report([check({status:'unknown',action:'ask_user',failure:'rosetta_check_timed_out',blocking:false})]));
  assert.equal(p.get('preflight-continue').hidden,false);
  p.app.show(report([check({id:'wine_runtime',action:'download',failure:'wine_runtime_missing',blocking:false})],{operation:'install'}));
  assert.equal(p.get('preflight-continue').hidden,false);
 }finally{await p.app.dispose();}
});

test('a reply for an older report does not overwrite a newer one',async()=>{
 let answer!:(value:Report)=>void;
 const p=mounted(()=>new Promise(resolve=>{answer=resolve;}));
 try{
  p.app.show(report([check()]));p.press('preflight-install-rosetta');
  p.app.show(report([check({id:'game_folder',action:'ask_user',failure:'game_folder_not_chosen'})],{operation:'install'}));
  answer(report([]));await p.app.settled();
  assert.equal(p.text('preflight-summary'),'Fix these before you install:');
  assert.equal(p.get('preflight-choose-folder').disabled,false);
 }finally{await p.app.dispose();}
});
