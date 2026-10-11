import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {makePanelContext} from './panel-context';
import {mountClientMaintenance,clientMaintenanceText,clientMaintenanceFailureText} from './client-maintenance-view';
const html=readFileSync('ui/index.html','utf8');

const status=(removed:boolean|null)=>({schema_version:1,cache:{path:'C:\\Users\\p\\Documents\\My Games\\Firesky\\SGWGame\\Cache.en-US',exists:removed===null},
 client_state:{path:'C:\\Users\\p\\Documents\\My Games\\Firesky',exists:true},removed});
type Request={command:string;confirmed?:boolean};
/** A rejection that is not a native code string. */
class Thrown{constructor(readonly error:unknown){}}
/** A native side that records each request and answers from `answer`. Each reset waits on `gate` so the busy state is observable. */
function mount(answer:(request:Request)=>unknown=request=>status(request.command==='inspect'?null:true)){
 const {document}=parseHTML(html);const requests:Request[]=[];let release=()=>{};
 let gate=Promise.resolve();
 const hold=()=>{gate=new Promise<void>(resolve=>{release=resolve;});};
 const app=mountClientMaintenance(document as unknown as Document,async(name,args)=>{
  assert.equal(name,'client_maintenance_command');const request=(args as {request:Request}).request;requests.push(request);
  if(request.command!=='inspect')await gate;
  const value=answer(request);if(typeof value==='string')throw value;if(value instanceof Thrown)throw value.error;return value;
 },makePanelContext().context);
 const el=(id:string)=>document.getElementById(id) as HTMLButtonElement;
 return {app,requests,hold,release:()=>release(),el,text:()=>el('client-maintenance-status').textContent};
}

test('reset all sends nothing until the confirm step is accepted, and cancel sends nothing',async()=>{
 const ui=mount();await ui.app.ready;
 assert.equal(ui.requests.length,0,'mounting asks nothing of native code');
 assert.equal(ui.el('client-maintenance-confirm').hidden,true);
 ui.el('client-maintenance-reset-all').click();await ui.app.settled();
 assert.equal(ui.el('client-maintenance-confirm').hidden,false);
 assert.equal(ui.text(),clientMaintenanceText.confirmOpen);
 assert.match(ui.el('client-maintenance-confirm-text').textContent!,/My Games\\Firesky/);
 assert.deepEqual(ui.requests.map(r=>r.command),['inspect'],'opening the confirm step only inspects, to name the folder');
 ui.el('client-maintenance-confirm-no').click();await ui.app.settled();
 assert.equal(ui.el('client-maintenance-confirm').hidden,true);assert.equal(ui.text(),clientMaintenanceText.cancelled);
 assert.deepEqual(ui.requests.map(r=>r.command),['inspect'],'cancel sends nothing');
 ui.el('client-maintenance-reset-all').click();ui.hold();ui.el('client-maintenance-confirm-yes').click();
 assert.equal(ui.text(),clientMaintenanceText.allBusy);assert.equal(ui.el('client-maintenance-reset-cache').disabled,true);
 ui.release();await ui.app.settled();
 assert.deepEqual(ui.requests.at(-1),{command:'reset_all_client_state',schema_version:1,confirmed:true});
 assert.equal(ui.text(),clientMaintenanceText.allDone);assert.equal(ui.el('client-maintenance-reset-all').disabled,false);
 await ui.app.dispose();
});

test('reset cache is busy on the first press, ignores a second press, then reports the result',async()=>{
 const ui=mount();await ui.app.ready;ui.hold();
 ui.el('client-maintenance-reset-cache').click();
 assert.equal(ui.text(),clientMaintenanceText.cacheBusy);
 assert.equal(ui.el('client-maintenance-reset-cache').getAttribute('aria-busy'),'true');
 ui.el('client-maintenance-reset-cache').click();
 ui.release();await ui.app.settled();
 assert.deepEqual(ui.requests.map(r=>r.command),['reset_cache']);
 assert.equal(ui.text(),clientMaintenanceText.cacheDone);
 await ui.app.dispose();
});

test('an empty cache says nothing to clear; a refusal shows its line',async()=>{
 let next:unknown=status(false);
 const ui=mount(request=>request.command==='inspect'?status(null):next);await ui.app.ready;
 ui.el('client-maintenance-reset-cache').click();await ui.app.settled();
 assert.equal(ui.text(),clientMaintenanceText.nothing);
 for(const code of ['busy','unsafe_path','io','profile_unavailable'] as const){
  next=code;ui.el('client-maintenance-reset-cache').click();await ui.app.settled();
  assert.equal(ui.text(),clientMaintenanceFailureText(code));
 }
 next={schema_version:1};ui.el('client-maintenance-reset-cache').click();await ui.app.settled();
 assert.equal(ui.text(),clientMaintenanceFailureText('schema'));
 next=new Thrown({storage:'busy'});ui.el('client-maintenance-reset-cache').click();await ui.app.settled();
 assert.equal(ui.text(),clientMaintenanceFailureText('transport'));
 await ui.app.dispose();assert.equal(ui.el('client-maintenance-title'),null);
});
