// Logic UAT for the Linux Proton copy and picker (PP-12). The real launch view and picker run on linkedom
// with a fake IPC: `launch_command` and `proton_command` answer from fixtures, so no Proton is discovered
// on this machine and no path crosses the boundary (R9). Prints one PASS/FAIL line per step; exit 1 on FAIL.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {launchText,mountLaunch} from './.test-build/launch-view.mjs';
import {mountProton} from './.test-build/proton-picker.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
// A launch status the native side would send: the same shape the launch-view tests decode.
const status=(extra={})=>({schema_version:1,setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:0,ops_skipped:0,installation_id:'owner',observation:null,failure:null,...extra,native:{schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed'],launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed',width:1024,height:768}},operation:{schema_version:1,revision:0,operation:null}}});
const protons=[{id:'valve',name:'Proton Experimental',kind:'experimental'},{id:'ge',name:'GE-Proton 9-2',kind:'ge'}];
let launchStatus=status({prerequisites_missing:['proton']});
let found=protons;let saved=null;const protonCalls=[];
// The fake IPC: `proton_command` refuses an id the picker is not offered now, as the native side does.
const invoke=async(command,{request})=>{
 if(command==='launch_command')return structuredClone(launchStatus);
 if(command!=='proton_command')throw new Error(`unexpected ${command}`);
 protonCalls.push(request);
 if(request.op==='select'){
  if(request.id!==null&&!found.some(p=>p.id===request.id))throw 'unknown_proton';
  saved=request.id;
 }
 return {available:found,selected:saved&&found.some(p=>p.id===saved)?saved:found[0]?.id??null,preferred:saved};
};
const {document,window}=parseHTML(html);
const el=id=>document.getElementById(id);
const settle=async pending=>{await pending;await new Promise(resolve=>setImmediate(resolve));};
const results=[];
async function step(label,body){
 try{await body();results.push(true);console.log(`PASS ${label}`);}
 catch(error){results.push(false);console.log(`FAIL ${label}: ${error.message}`);}
}
const launch=mountLaunch(document,invoke,()=>{},()=>'00000000-0000-4000-8000-000000000000');
const picker=mountProton(document,invoke,el('display-panel'));
await settle(launch.ready);await settle(picker.ready);
await step('(a) no Proton: Play disabled, install copy shown, status decodes without a schema failure',async()=>{
 await settle(launch.refresh());
 assert.equal(el('launch').disabled,true);
 assert.match(el('launch-status').textContent,/Install Proton to play\. In Steam, install Proton Experimental, or install GE-Proton with ProtonUp-Qt\. Then reopen the launcher\./);
 assert.doesNotMatch(el('launch-status').textContent,/folder-access|PhysX/);
 assert.equal(el('launch-status').textContent,launchText({status:launchStatus,pending:false,uncertain:false,error:null}));
});
await step('(a) no Proton: the picker is hidden when discovery finds no Proton',async()=>{
 found=[];await settle(picker.refresh());
 assert.equal(el('proton-picker').hidden,true);
 found=protons;await settle(picker.refresh());
});
await step('(b) two Protons: the picker lists Automatic then both in order, and a select says "Saved."',async()=>{
 assert.equal(el('proton-picker').hidden,false);
 const options=[...el('proton-select').querySelectorAll('option')].map(o=>[o.getAttribute('value'),o.textContent]);
 assert.deepEqual(options,[['','Automatic'],['valve','Proton Experimental'],['ge','GE-Proton 9-2']]);
 const option=el('proton-select').querySelector('option[value="ge"]');option.selected=true;
 el('proton-select').dispatchEvent(new window.Event('change'));await settle(picker.settled());
 assert.deepEqual(protonCalls.at(-1),{op:'select',id:'ge'});
 assert.equal(el('proton-status').textContent,'Saved.');
});
await step('(c) unknown id refused: error text shown and the selection is unchanged',async()=>{
 const before=el('proton-select').value;
 found=[protons[0]];
 const option=el('proton-select').querySelector('option[value="ge"]');option.selected=true;
 el('proton-select').dispatchEvent(new window.Event('change'));await settle(picker.settled());
 assert.match(el('proton-status').textContent,/isn't installed any more, so it wasn't chosen/);
 assert.equal(el('proton-select').value,before);
 assert.equal(saved,'ge');
 found=protons;
});
await step('(d) proton_missing renders its C-6 title and fix',async()=>{
 const text=launchText({status:status({failure:'proton_missing'}),pending:false,uncertain:false,error:null});
 assert.match(text,/^Proton isn't installed\.\nInstall Proton Experimental from Steam, or GE-Proton with ProtonUp-Qt, then press Play again\.$/);
});
await step('(d) proton_setup_failed renders its C-6 title and fix',async()=>{
 const text=launchText({status:status({failure:'proton_setup_failed'}),pending:false,uncertain:false,error:null});
 assert.match(text,/^Proton couldn't set up the game\.\nPress Play to try again\./);
});
await launch.dispose();await picker.dispose();
const failed=results.filter(ok=>!ok).length;
console.log(failed===0?`Proton UAT passed: ${results.length} steps. Excluded: real Proton discovery, Steam, game launch and packaged visual checks.`:`Proton UAT failed: ${failed} of ${results.length} steps.`);
process.exit(failed===0?0:1);
