import test from 'node:test';
import assert from 'node:assert/strict';
import {reportUiEvent} from './ui-event';

/** Installs a fake Tauri `invoke` on the global window for one test, and returns the calls it saw. */
function fakeTauri(answer:() => Promise<unknown>):Array<[string,Record<string,unknown>|undefined]> {
 const calls:Array<[string,Record<string,unknown>|undefined]>=[];
 const invoke=async(command:string,args?:Record<string,unknown>)=>{calls.push([command,args]);return answer();};
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke}}}});
 return calls;
}

test('reportUiEvent invokes ui_event with the arguments', async () => {
 const calls=fakeTauri(async () => undefined);
 reportUiEvent('install','step_failed','disk_full');
 reportUiEvent('telemetry','ping','ok','info');
 await new Promise(resolve => setImmediate(resolve));
 assert.deepEqual(calls,[
  ['ui_event',{area:'install',event:'step_failed',code:'disk_full',level:'warn'}],
  ['ui_event',{area:'telemetry',event:'ping',code:'ok',level:'info'}],
 ]);
 Reflect.deleteProperty(globalThis,'window');
});

test('reportUiEvent ignores a rejected invoke', async () => {
 fakeTauri(async () => {throw 'invalid ui event';});
 assert.doesNotThrow(() => reportUiEvent('launch','play_failed','timeout'));
 await new Promise(resolve => setImmediate(resolve));
 Reflect.deleteProperty(globalThis,'window');
});
