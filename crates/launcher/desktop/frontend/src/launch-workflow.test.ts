import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect,Schema} from 'effect';
import {LaunchStatus} from './launch-workflow';

const initial=()=>({schema_version:1,setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:1,ops_skipped:3,installation_id:'owner',observation:null as unknown,native:{schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1,revision:0,operation:null as unknown}}});
const decode=(value:unknown)=>Effect.runPromise(Schema.decodeUnknownEffect(LaunchStatus,{onExcessProperty:'error'})(value));

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
