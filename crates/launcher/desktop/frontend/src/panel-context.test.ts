import test from 'node:test';
import assert from 'node:assert/strict';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';

const snapshot=(revision:number):NativeSnapshot=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},
 preferences:{schema_version:2,revision,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed' as const,width:1024,height:768}},requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null}});

test('a panel sees the latest snapshot and each later one until it unsubscribes',()=>{
 const {context,publish}=makePanelContext();
 assert.equal(context.snapshot(),undefined);
 const seen:number[]=[];
 const unsubscribe=context.onSnapshot(next=>seen.push(next.preferences.revision));
 publish(snapshot(1));
 assert.equal(context.snapshot()?.preferences.revision,1);
 unsubscribe();
 publish(snapshot(2));
 assert.deepEqual(seen,[1]);
 assert.equal(context.snapshot()?.preferences.revision,2);
});

test('refresh is the launcher refresh it was built with',async()=>{
 let calls=0;
 const {context}=makePanelContext(async()=>{calls++;});
 await context.refresh();
 assert.equal(calls,1);
});
