import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountMigration} from './migration-view';
const html=readFileSync('ui/index.html','utf8');
const legacy={source:{launcher_directory:'/legacy',game_directory:'/game'},identity:{schema_version:1,install_id:'identity',machine_id:'machine',first_seen_ms:12,created_by_launcher_version:'old'},config:{schema_version:2,install_path:'C:\\Game',manifest_url:'https://example.invalid',login_servers:[],telemetry:{opted_in:false,prompt_answered:false,auth_url:'https://example.invalid'},client_patches:{enabled:false,dll_override:null}},ledger:{applied_patches:['two','one','two'],seed_sha256:null,seed_adopted:true},confirmation:'digest'};
const initial=()=>({schema_version:1,imported:null as typeof legacy|null,preview:null as {imported:typeof legacy;preferences_revision:number}|null,native:{schema_version:1,requires_reopen:false,preferences:{schema_version:1,revision:0,install_directory:null as string|null,launcher_summary_consent:true},operation:{schema_version:1,revision:0,operation:null}}});
const settle=()=>new Promise(resolve=>setImmediate(resolve));
test('explicit confirmation, duplicate suppression and lost response reconcile without replay',async()=>{
 let status=initial();let commits=0;let finish!:()=>void;const wait=new Promise<void>(resolve=>{finish=resolve;});
 const {document,window}=parseHTML(html);const get=(id:string)=>document.getElementById(id)!;const click=(id:string)=>get(id).dispatchEvent(new window.Event('click'));
 const app=mountMigration(document,async(command,args:any)=>{
  if(command==='choose_legacy_source'){status={...status,preview:{imported:legacy,preferences_revision:0}};return status;}
  const r=args.request;if(r.command==='confirm'){assert.equal('source' in r,false);commits++;status={...status,preview:null,imported:legacy,native:{...status.native,preferences:{...status.native.preferences,revision:1,install_directory:'/game'}}};await wait;throw 'transport';}
  if(r.command==='dismiss')status={...status,preview:null};return status;
 });
 try{await app.ready;await settle();click('choose-legacy');await app.settled();await settle();assert.equal(commits,0);assert.match(get('migration-details').textContent!,/C:\\\\Game/);assert.match(get('migration-consent').textContent!,/Game telemetry consent: off.*diagnostics: on \(unchanged\)/);
 click('confirm-migration');click('confirm-migration');assert.match(get('migration-status').textContent!,/Checking/);finish();await app.settled();await settle();assert.equal(commits,1);assert.equal((get('confirm-migration') as HTMLButtonElement).disabled,true);assert.match(get('migration-status').textContent!,/not retried/);
 await app.refresh();await settle();assert.equal(commits,1);assert.match(get('migration-status').textContent!,/does not enable Play, Repair or Uninstall/);assert.match(get('migration-status').textContent!,/separate empty folder/);assert.equal(get('migration-actions').hidden,true);
 }finally{await app.dispose();}
});
test('cancelled chooser and dismissed preview cannot commit; source errors demand fresh preview',async()=>{
 let status=initial();let picks=0;let commits=0;const {document,window}=parseHTML(html);const get=(id:string)=>document.getElementById(id)!;const click=(id:string)=>get(id).dispatchEvent(new window.Event('click'));
 const app=mountMigration(document,async(command,args:any)=>{
 if(command==='choose_legacy_source'){if(++picks>1)status={...status,preview:{imported:legacy,preferences_revision:0}};return status;}
 if(args.request.command==='dismiss')status={...status,preview:null};
 if(args.request.command==='confirm'){commits++;status={...status,preview:null};throw 'source_changed';}return status;});
 try{await app.ready;await settle();click('choose-legacy');await app.settled();await settle();assert.equal(get('migration-review').hidden,true);
 click('choose-legacy');await app.settled();await settle();click('dismiss-migration');await app.settled();await settle();click('confirm-migration');assert.equal(commits,0);
 click('choose-legacy');await app.settled();await settle();click('confirm-migration');await app.settled();await settle();assert.match(get('migration-status').textContent!,/fresh preview/);click('confirm-migration');assert.equal(commits,1);
 }finally{await app.dispose();}
});
