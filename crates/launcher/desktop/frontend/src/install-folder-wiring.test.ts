import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {mountDesktop} from './composition';

const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const native=(directory:string)=>({schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null},
 preferences:{schema_version:2,revision:1,install_directory:directory,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed' as const,width:1024,height:768}},
 operation:{schema_version:1,revision:0,operation:null}});

test('Change in the install area chooses through the Settings save path',async()=>{
 const {document,window}=parseHTML(html);let directory='/fixture';let chooses=0;const saves:unknown[]=[];
 const app=mountDesktop(document as unknown as Document,async(command:string,args?:any)=>{
  if(command==='choose_install_directory'){chooses++;return '/games/picked';}
  if(command==='launcher_command'){
   if(args?.request?.command==='save_preferences'){saves.push(args.request);directory=args.request.install_directory;}
   return native(directory);
  }
  if(command==='install_command')return {schema_version:1,install_supported:true,can_resume:false,can_reconcile:false,can_retry:false,uninstall:null,
   runtime_setup:null,progress:null,outcome:null,native:native(directory)};
  return native(directory);
 });
 try{
  for(let i=0;i<50&&document.getElementById('install-folder-change')!.hasAttribute('disabled');i++)await new Promise(resolve=>setTimeout(resolve,10));
  document.getElementById('install-folder-change')!.dispatchEvent(new window.Event('click'));
  for(let i=0;i<50&&saves.length===0;i++)await new Promise(resolve=>setTimeout(resolve,10));
  assert.equal(chooses,1,'the native folder dialog was opened');
  assert.equal(saves.length,1,'one save through the Settings path');
  assert.equal((saves[0] as {install_directory:string}).install_directory,'/games/picked');
 }finally{await app.dispose();}
});
