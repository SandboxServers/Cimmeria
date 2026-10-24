import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect,Exit} from 'effect';
import {decodeSnapshot,safeFailure,scaffoldFailure} from './contract';

// Tauri rejects with the serialized Rust error. The scaffold errors serialize
// flat (engine `errors_serialize_as_the_flat_codes_the_bridge_accepts`).
test('a flat native code from a scaffold command survives the bridge',()=>{
 const wire=JSON.parse(JSON.stringify('busy'));
 assert.equal(safeFailure(wire).code,'busy');
 assert.equal(scaffoldFailure(wire),'busy');
 assert.equal(scaffoldFailure('unsupported_schema'),'unsupported_schema');
 assert.equal(scaffoldFailure('unavailable'),'unavailable');
});

test('anything else is a transport failure, never a guessed code',()=>{
 for(const wire of [{storage:'busy'},'unknown',null,undefined,7])assert.equal(scaffoldFailure(wire),'transport');
 // The general bridge still has no `unavailable` code.
 assert.equal(safeFailure('unavailable').code,'transport');
});

// A snapshot as `DesktopState::inspect` serializes it, with the display preference (C-PD6).
const wireSnapshot=()=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},
 preferences:{schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,
  display:{mode:'windowed',width:1600,height:900}},
 requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},display_modes:['windowed','fullscreen']});
const decodes=(value:unknown)=>Exit.isSuccess(Effect.runSyncExit(decodeSnapshot(value)));

test('a snapshot with the display preference and the offered modes decodes',()=>{
 const decoded=Effect.runSync(decodeSnapshot(wireSnapshot()));
 assert.deepEqual(decoded.preferences.display,{mode:'windowed',width:1600,height:900});
 assert.deepEqual(decoded.display_modes,['windowed','fullscreen']);
 assert.equal(safeFailure('invalid_display_options').code,'invalid_display_options');
});

test('a snapshot without display_modes or with an unknown mode is rejected',()=>{
 const {display_modes:_,...withoutModes}=wireSnapshot();
 assert.equal(decodes(withoutModes),false);
 const game=wireSnapshot();(game.preferences.display as {mode:string}).mode='game';
 assert.equal(decodes(game),false);
});
