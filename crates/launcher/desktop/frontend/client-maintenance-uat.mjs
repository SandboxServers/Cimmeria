// Logic UAT (LX-06): the real Settings page with the client-maintenance panel mounted
// against a native side that keeps a client folder in memory, the way the engine does:
// reset cache removes Cache.en-US, reset all removes Firesky and refuses without confirmation.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientMaintenance,clientMaintenanceText} from './.test-build/client-maintenance-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const firesky='C:\\Users\\player\\Documents\\My Games\\Firesky';
const disk={cache:true,state:true};
const requests=[];
const answer=removed=>({schema_version:1,cache:{path:`${firesky}\\SGWGame\\Cache.en-US`,exists:disk.cache},
 client_state:{path:firesky,exists:disk.state},removed});
const native=async(name,{request})=>{
 assert.equal(name,'client_maintenance_command');requests.push(request);
 if(request.command==='inspect')return answer(null);
 if(request.command==='reset_cache'){const removed=disk.cache;disk.cache=false;return answer(removed);}
 if(!request.confirmed)throw 'confirmation_required';
 const removed=disk.state;disk.state=disk.cache=false;return answer(removed);
};
const app=mountClientMaintenance(document,native,{snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}});
const el=id=>document.getElementById(id);
const text=()=>el('client-maintenance-status').textContent;
const press=async id=>{el(id).click();const shown=text();await app.settled();return shown;};
try {
 await app.ready;assert.deepEqual(requests,[],'mounting asks nothing of native code');
 assert.ok(el('client-maintenance-panel').closest('#gear'),'the panel sits in Settings');
 assert.equal(el('client-maintenance-title').textContent,'Client cache and settings');

 assert.equal(await press('client-maintenance-reset-cache'),clientMaintenanceText.cacheBusy,'busy on the first press');
 assert.equal(text(),clientMaintenanceText.cacheDone);assert.equal(disk.cache,false);assert.equal(disk.state,true);
 await press('client-maintenance-reset-cache');assert.equal(text(),clientMaintenanceText.nothing);

 assert.equal(await press('client-maintenance-reset-all'),clientMaintenanceText.confirmOpen);
 assert.equal(el('client-maintenance-confirm').hidden,false);assert.ok(!requests.some(r=>r.command==='reset_all_client_state'),'the confirm step sends no reset');
 assert.ok(el('client-maintenance-confirm-text').textContent.includes(firesky),'the confirm step names the folder');
 await press('client-maintenance-confirm-no');assert.equal(text(),clientMaintenanceText.cancelled);
 assert.ok(!requests.some(r=>r.command==='reset_all_client_state'),'cancel sends no reset');assert.equal(disk.state,true);

 await press('client-maintenance-reset-all');
 assert.equal(await press('client-maintenance-confirm-yes'),clientMaintenanceText.allBusy);
 assert.equal(text(),clientMaintenanceText.allDone);assert.equal(disk.state,false);
 assert.deepEqual(requests.at(-1),{command:'reset_all_client_state',schema_version:1,confirmed:true});
 assert.ok(requests.every(r=>r.command!=='reset_all_client_state'||r.confirmed===true),'no unconfirmed reset was sent');

 console.log('Client cache and settings UAT passed: Reset client cache clears only the cache and says so, a second press says nothing to clear; Reset all client state opens a confirm step that names the folder, Cancel sends nothing, Delete all client state removes the tree.');
 console.log('Excluded: real deletion on disk (engine tests cover it); the packaged window and visual layout.');
}finally{await app.dispose();}
