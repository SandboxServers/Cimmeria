// Exercise the actual composition in a JS REPL-style sequence with durable mock native state.
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {mountDesktop} from './.test-build/composition.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
let revision=0,valid=false,owned=false,calls=0,pick,backend='in_place';
const native=()=>({schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed'],launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed',width:1024,height:768}},operation:{schema_version:1,revision,operation:null}});
const status=()=>({schema_version:1,native:native(),backend,imported:null,modes:backend==='in_place'?['light']:[],activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],has_valid_install:valid,owned,completed:null,last_error:null,last_rejection:null,unsafe_entries:null});
let choose=()=>new Promise(resolve=>{pick=resolve;});
const invoke=async command=>{
 if(command==='adoption_command')return status();
 if(command==='choose_existing_game_folder'){calls++;return choose();}
 if(command==='install_command')return {schema_version:1,native:native(),install_supported:false,can_resume:false,can_reconcile:false,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null};
 return native();
};
const mount=()=>{const {document,window}=parseHTML(html);return {app:mountDesktop(document,invoke),get:id=>document.getElementById(id),press:id=>document.getElementById(id).dispatchEvent(new window.Event('click'))};};
const tick=()=>new Promise(resolve=>setTimeout(resolve,15));
const settle=async ui=>{await ui.app.app.settled();await ui.app.adoption.settled();await tick();await ui.app.adoption.settled();};
let ui=mount();
try{
 await Promise.all([ui.app.app.ready,ui.app.adoption.ready]);await settle(ui);
 assert.equal(ui.get('adopt-install').hidden,false);
 ui.press('adopt-install');assert.equal(ui.get('gear').hidden,false);assert.equal(ui.get('adoption-status').textContent,'Choose SGW.exe in the native dialog; the launcher will check its game folder…');
 ui.press('adopt-install');ui.press('choose-adoption-destination');await tick();assert.equal(calls,1);
 pick(status());await settle(ui);assert.equal(ui.get('adoption-status').textContent,'No SGW.exe chosen.');
 choose=async()=>{throw 'not_a_game_folder';};ui.press('choose-adoption-destination');await settle(ui);assert.match(ui.get('adoption-status').textContent,/not a Cimmeria game folder/);
 choose=async()=>{valid=true;owned=true;revision++;return status();};ui.press('choose-adoption-destination');await settle(ui);assert.equal(ui.get('adopt-install').hidden,true);
 await ui.app.dispose();ui=mount();await Promise.all([ui.app.app.ready,ui.app.adoption.ready]);await settle(ui);assert.equal(ui.get('adopt-install').hidden,true,'fresh composition reads persisted native content validity');
 valid=false;owned=false;revision++;assert.equal(ui.get('gear').hidden,true);await ui.app.app.refresh();await settle(ui);assert.equal(ui.get('adopt-install').hidden,false);
 owned=true;revision++;await ui.app.app.refresh();await settle(ui);assert.equal(ui.get('adopt-install').disabled,true);assert.match(ui.get('adopt-install-status').textContent,/Repair it or uninstall/);
 owned=false;backend='helper_unavailable';revision++;await ui.app.app.refresh();await settle(ui);assert.equal(ui.get('adopt-install').disabled,true);assert.match(ui.get('adopt-install-status').textContent,/Install the complete launcher build/);const before=calls;ui.press('adopt-install');await settle(ui);assert.equal(calls,before,'missing-helper feedback cannot start adoption');
 console.log('PASS: actual composition, immediate visible feedback before deferred picker, one choose per repeated press, cancellation/error, persisted success/reopen and uninstall refresh with Settings closed, invalid-owner refusal.');
 console.log('Not covered: native physical picker, filesystem effects, packaged visual layout, real game launch, macOS runtime.');
}finally{await ui.app.dispose();}
