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
