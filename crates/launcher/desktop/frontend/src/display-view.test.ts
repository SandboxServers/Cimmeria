import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {DisplayMode,DisplayOptions,NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountDisplay,displayAppliesText,displayMacText,displayWindowsText} from './display-view';
import {failureText} from './failure-copy';
const html=readFileSync('ui/index.html','utf8');
const windowsModes:DisplayMode[]=['windowed','fullscreen'];
const macModes:DisplayMode[]=['windowed'];
const snapshot=(revision:number,display:DisplayOptions={mode:'windowed',width:1024,height:768},display_modes:DisplayMode[]=windowsModes):NativeSnapshot=>({
 schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,display_modes,launcher:{version:'0.1.0',tag:null},
 preferences:{schema_version:2,revision,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display},
});
function mounted(respond:(request:any)=>Promise<unknown>=async()=>null,refresh:()=>Promise<unknown>=async()=>{},first:NativeSnapshot|null=snapshot(3)) {
 const {document,window}=parseHTML(html);const calls:{command:string;request:any}[]=[];
 const feed=makePanelContext(()=>refresh());if(first)feed.publish(first);
 const app=mountDisplay(document as unknown as Document,async(command,args)=>{const request=(args as {request:unknown}).request;calls.push({command,request});return respond(request);},feed.context);
 const el=<T>(id:string)=>document.getElementById(id) as unknown as T;
 const mode=el<HTMLSelectElement>('display-mode');const size=el<HTMLSelectElement>('display-size');const save=el<HTMLButtonElement>('display-save');
 const choose=(select:HTMLSelectElement,value:string)=>{(select.querySelector(`option[value="${value}"]`) as HTMLOptionElement).selected=true;select.dispatchEvent(new window.Event('change'));};
 const press=()=>save.dispatchEvent(new window.Event('click'));
 const text=(id:string)=>document.getElementById(id)!.textContent;
 const hidden=(id:string)=>document.getElementById(id)!.hidden;
 const options=(select:HTMLSelectElement)=>Array.from(select.querySelectorAll('option')).map(o=>[o.getAttribute('value'),o.textContent]);
 return {document,calls,feed,app,mode,size,save,choose,press,text,hidden,options};
}

test('the panel sits in the gear after client maintenance',()=>{
 const {document}=parseHTML(html);
 const panel=document.getElementById('display-panel')!;
 assert.equal(panel.closest('#gear')?.id,'gear');
 assert.equal(panel.previousElementSibling?.id,'client-maintenance-panel');
});

test('a Windows snapshot offers both modes with the saved one selected, and the Windows note',async()=>{
 const m=mounted(undefined,undefined,snapshot(3,{mode:'fullscreen',width:1024,height:768}));
 assert.deepEqual(m.options(m.mode),[['windowed','Windowed'],['fullscreen','Fullscreen (uses the resolution chosen in the game)']]);
 assert.equal(m.mode.value,'fullscreen');
 assert.equal(m.hidden('display-mode'),false);assert.equal(m.hidden('display-mac'),true);
 assert.equal(m.text('display-applies'),displayAppliesText);
 assert.equal(m.hidden('display-windows-note'),false);assert.equal(m.text('display-windows-note'),displayWindowsText);
 assert.equal(m.save.textContent,'Save');assert.equal(m.save.disabled,false);
 await m.app.dispose();assert.equal(m.document.getElementById('display-title'),null);
});

test('the size list has the fixed sizes, and the saved size when it is none of them',async()=>{
 const m=mounted(undefined,undefined,snapshot(3,{mode:'windowed',width:1366,height:768}));
 assert.deepEqual(m.options(m.size),[['1024x768','1024 × 768'],['1280x720','1280 × 720'],['1600x900','1600 × 900'],
  ['1920x1080','1920 × 1080'],['2560x1440','2560 × 1440'],['1366x768','1366 × 768']]);
 assert.equal(m.size.value,'1366x768');
 m.feed.publish(snapshot(4,{mode:'windowed',width:1600,height:900}));
 assert.equal(m.options(m.size).length,5);assert.equal(m.size.value,'1600x900');
 await m.app.dispose();
});

test('choosing fullscreen hides the size select, and windowed shows it again',async()=>{
 const m=mounted();
 assert.equal(m.hidden('display-size'),false);
 m.choose(m.mode,'fullscreen');assert.equal(m.hidden('display-size'),true);
 m.choose(m.mode,'windowed');assert.equal(m.hidden('display-size'),false);
 await m.app.dispose();
});

test('a Mac snapshot shows the Mac line instead of the mode select, and only the size select',async()=>{
 const m=mounted(undefined,undefined,snapshot(3,undefined,macModes));
 assert.equal(m.hidden('display-mode'),true);assert.equal(m.hidden('display-mode-label'),true);
 assert.equal(m.hidden('display-mac'),false);assert.equal(m.text('display-mac'),displayMacText);
 assert.equal(m.hidden('display-size'),false);
 assert.equal(m.hidden('display-windows-note'),true);
 await m.app.dispose();
});

test('Save sends one save_display_options with the chosen mode and size, shows Saving… at once, then Saved',async()=>{
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(4,{mode:'windowed',width:1920,height:1080}));});
 m.choose(m.size,'1920x1080');
 m.press();
 assert.equal(m.text('display-status'),'Saving…');assert.equal(m.save.disabled,true);
 m.press();await m.app.settled();
 assert.equal(m.calls.length,1,'a second press while saving sends nothing');
 assert.equal(m.calls[0]!.command,'launcher_command');
 assert.deepEqual(m.calls[0]!.request,{command:'save_display_options',schema_version:1,expected_revision:3,
  display:{mode:'windowed',width:1920,height:1080}});
 assert.equal(m.text('display-status'),'Saved. Applies the next time you press Play.');
 assert.equal(m.save.disabled,false);assert.equal(m.size.value,'1920x1080');
 await m.app.dispose();
});

test('a Mac save sends windowed',async()=>{
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(4,{mode:'windowed',width:1280,height:720},macModes));},snapshot(3,undefined,macModes));
 m.choose(m.size,'1280x720');m.press();await m.app.settled();
 assert.deepEqual(m.calls[0]!.request.display,{mode:'windowed',width:1280,height:720});
 await m.app.dispose();
});

test('a stale save re-inspects, keeps the choice and says nothing was saved; the next save carries the new revision',async()=>{
 let refreshes=0;let stale=true;
 const m=mounted(async()=>{if(stale)throw 'stale_revision';return null;},async()=>{refreshes++;m.feed.publish(snapshot(5+refreshes));});
 m.choose(m.mode,'fullscreen');m.press();await m.app.settled();
 assert.equal(refreshes,1);
 assert.equal(m.text('display-status'),'Settings changed while saving, so nothing was saved. Press Save again to save this setting.');
 assert.equal(m.mode.value,'fullscreen','the refresh does not overwrite the player\'s choice');
 stale=false;m.press();await m.app.settled();
 assert.equal(m.calls[1]!.request.expected_revision,6);
 assert.equal(m.calls[1]!.request.display.mode,'fullscreen');
 await m.app.dispose();
});

test('an invalid size refusal asks for one from the list',async()=>{
 const m=mounted(async()=>{throw 'invalid_display_options';});
 m.press();await m.app.settled();
 assert.equal(m.text('display-status'),'That size can\'t be used. Pick one from the list.');
 assert.equal(m.save.disabled,false);
 await m.app.dispose();
});

test('any other refusal reads the C-PD1 copy its job_errors code maps to, and an unmapped one reads unknown',async()=>{
 for(const [code,failure] of [['busy','operation_busy'],['io','local_io'],['in_use','unknown']] as const){
  const m=mounted(async()=>{throw code;});
  m.press();await m.app.settled();
  assert.equal(m.text('display-status'),failureText(failure).message,code);
  await m.app.dispose();
 }
});

test('a refresh that brings no newer snapshot says it could not reload',async()=>{
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(3));});
 m.choose(m.size,'2560x1440');m.press();await m.app.settled();
 assert.equal(m.text('display-status'),'Saved, but the launcher could not reload settings. Restart the launcher to see the saved setting.');
 assert.equal(m.size.value,'2560x1440','the old snapshot does not put the previous size back');
 await m.app.dispose();
});

test('Save is disabled until settings load',async()=>{
 const m=mounted(undefined,undefined,null);
 assert.equal(m.save.disabled,true);
 m.feed.publish(snapshot(1));assert.equal(m.save.disabled,false);
 await m.app.dispose();
});
