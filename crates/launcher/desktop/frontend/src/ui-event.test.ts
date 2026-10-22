import test from 'node:test';
import assert from 'node:assert/strict';
import {reportUiEvent,uiCode} from './ui-event';

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

test('reportUiEvent is a no-op without a Tauri window', () => {
 Reflect.deleteProperty(globalThis,'window');
 assert.doesNotThrow(() => reportUiEvent('launch','play_failed','timeout'));
});

test('uiCode passes a valid code through and falls back for anything else', () => {
 assert.equal(uiCode('busy'),'busy');
 for (const bad of ['Bad Code', {}, undefined, 'a'.repeat(41)]) assert.equal(uiCode(bad),'transport', String(bad));
 assert.equal(uiCode(undefined,'schema'),'schema');
});

test('reportUiEvent ignores a rejected invoke', async () => {
 fakeTauri(async () => {throw 'invalid ui event';});
 assert.doesNotThrow(() => reportUiEvent('launch','play_failed','timeout'));
 await new Promise(resolve => setImmediate(resolve));
 Reflect.deleteProperty(globalThis,'window');
});
