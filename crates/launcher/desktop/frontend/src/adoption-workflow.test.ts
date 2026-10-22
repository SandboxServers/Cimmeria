import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect,Schema} from 'effect';
import {AdoptionStatus} from './adoption-workflow';

const native=(revision:number)=>({schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1,revision,operation:null}});
const status=(modes:unknown[]):unknown=>({schema_version:1,native:native(1),backend:'in_place',imported:null,modes,activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],owned:false,completed:null,last_error:null,last_rejection:null,unsafe_entries:null});
const decode=(value:unknown)=>Effect.runPromise(Schema.decodeUnknownEffect(AdoptionStatus,{onExcessProperty:'error'})(value));

test('the light mode is an offered mode and decodes first', async () => {
  const decoded=await decode(status(['light','copy']));
  assert.deepEqual(decoded.modes,['light','copy']);
});

test('an unknown mode is refused by the schema', async () => {
  await assert.rejects(decode(status(['bulk'])));
});

const light={patches_total:2,patches_current:1,patches_pending:1,download_bytes:1024,skipped:[{path:'Working/a.upk',patch:'p2',reason:'source_mismatch'}],skipped_total:1,index_state:'compared',index_counts:{stock:5,patched:1,prepared:0,changed:1,missing:0,extra:2},index_files:[['Working/a.upk','changed'],['notes.txt','extra']],index_files_total:2};
const review=(extra:object={})=>({preview_handle:'p',operation_revision:1,preferences_revision:1,source:'/f',destination:'/f',source_kind:'folder',release:{manifest_sha256:'m',seed_sha256:'s',patches:['p1','p2']},counts:{matched:1,known_transform:0,modified:0,needs_patching:0,missing:0,extra:0},differences:[],differences_omitted:0,requires_replacement:false,game_telemetry_available:true,requires_normalization:false,requires_telemetry_acceptance:false,user_data_remains_in_source:false,in_place:true,blocked_files:[],...extra});

test('a light review decodes its patch counts and index comparison', async () => {
  const decoded=await decode({...(status(['light','copy']) as object),activity:'review',review:review({light})});
  assert.equal(decoded.review?.light?.patches_pending,1);
  assert.equal(decoded.review?.light?.index_files[0][1],'changed');
});

test('a light review with an unknown field is refused by the schema', async () => {
  await assert.rejects(decode({...(status(['light','copy']) as object),activity:'review',review:review({light:{...light,surprise:1}})}),/\["light"\]\["surprise"\]/);
});

test('the scan progress item decodes, and an unknown item is refused', async () => {
  const progress={phase:'extraction',current:0,total:15,item:'scan'};
  const decoded=await decode({...(status(['light','copy']) as object),activity:'preparing',progress});
  assert.equal(decoded.progress?.item,'scan');
  await assert.rejects(decode({...(status(['light','copy']) as object),activity:'preparing',progress:{...progress,item:'scanning'}}));
});

test('a completed light adoption decodes its flag, and a non-boolean flag is refused', async () => {
  const done=await decode({...(status(['light','copy']) as object),owned:true,completed:{directory:'/f',light:true}});
  assert.equal(done.completed?.light,true);
  assert.equal((await decode({...(status(['light','copy']) as object),completed:{directory:'/f'}})).completed?.light,undefined);
  await assert.rejects(decode({...(status(['light','copy']) as object),completed:{directory:'/f',light:'yes'}}));
});
