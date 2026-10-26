import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect,Schema} from 'effect';
import {LaunchStatus,launchBlocked,presyncFailed} from './launch-workflow';

const initial=()=>({schema_version:1,setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:1,ops_skipped:3,installation_id:'owner',observation:null as unknown,failure:null as unknown,native:{schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed' as const,width:1024,height:768}},operation:{schema_version:1,revision:0,operation:null as unknown}}});
const decode=(value:unknown)=>Effect.runPromise(Schema.decodeUnknownEffect(LaunchStatus,{onExcessProperty:'error'})(value));
test('missing game components and an active vendor installer each block Play',async()=>{
 for(const prerequisites_missing of [['directx'],['physx'],['directx','physx']]){
  assert.equal(launchBlocked(await decode({...initial(),prerequisites_missing})),true);
 }
 assert.equal(launchBlocked(await decode({...initial(),prerequisites_missing:[],prerequisites_installing:true})),true);
 assert.equal(launchBlocked(await decode({...initial(),prerequisites_missing:[],prerequisites_installing:false})),false);
 await assert.rejects(decode({...initial(),prerequisites_missing:['directx_12']}));
});

test('the incomplete patch counts decode with the launch status', async () => {
  const decoded=await decode(initial());
  assert.equal(decoded.patches_incomplete,1);
  assert.equal(decoded.ops_skipped,3);
});
test('native setup completeness is required independently of Play admission',async()=>{
 const {setup_complete:_setup,...old}=initial();await assert.rejects(decode(old));
 const blocked={...initial(),installation_id:null,launcher_update_required:true};
 assert.equal((await decode(blocked)).setup_complete,true);
});

test('a launch status without the incomplete patch counts is refused', async () => {
  const {patches_incomplete:_incomplete,ops_skipped:_skipped,...withoutCounts}=initial();
  await assert.rejects(decode(withoutCounts));
});

test('the game data check decodes when absent, null, in progress or failed, and an unknown reason still decodes', async () => {
  const failed={phase:'done',categories_done:0,categories_total:3,bytes_done:0,bytes_total:0,outcome:'failed',reason:'http_error',server:'play.example:8081',
    failure:{step:'download',reason:'http_error',category_id:4,pak:'CookedDataItems.pak',http_status:500,io_kind:null,os_code:null,retry_after_s:null,source_version:null,served_version:null,deadline_s:null}};
  assert.equal((await decode(initial())).content_sync,undefined);
  assert.equal((await decode({...initial(),content_sync:null})).content_sync,null);
  assert.equal((await decode({...initial(),content_sync:{...failed,phase:'downloading',outcome:null,reason:null,failure:null}})).content_sync?.phase,'downloading');
  assert.equal((await decode({...initial(),content_sync:failed})).content_sync?.failure?.http_status,500);
  assert.equal((await decode({...initial(),content_sync:{...failed,failure:{...failed.failure,reason:'something_new'}}})).content_sync?.failure?.reason,'something_new');
  await assert.rejects(decode({...initial(),content_sync:{...failed,phase:'paused'}}));
});
test('a presync that failed before Play is what the workflow reads as presyncFailed', async () => {
  const failed={phase:'done',categories_done:0,categories_total:3,bytes_done:0,bytes_total:0,outcome:'failed',reason:'timeout',server:null,failure:null};
  assert.equal(presyncFailed(await decode({...initial(),observation:{phase:'not_started'},content_sync:failed})),true);
  assert.equal(presyncFailed(await decode({...initial(),observation:{phase:'preparing'},content_sync:failed})),false);
  assert.equal(presyncFailed(await decode({...initial(),observation:{phase:'not_started'},content_sync:{...failed,outcome:'skipped'}})),false);
  assert.equal(presyncFailed(null),false);
});
test('a launch status carries the failure code of a launch that did not start', async () => {
  const decoded=await decode({...initial(),observation:{phase:'not_started'},failure:'rosetta_missing'});
  assert.equal(decoded.failure,'rosetta_missing');
  assert.equal((await decode(initial())).failure,null);
  await assert.rejects(decode({...initial(),failure:'not_a_failure_code'}));
  const {failure:_failure,...withoutFailure}=initial();
  await assert.rejects(decode(withoutFailure));
});
