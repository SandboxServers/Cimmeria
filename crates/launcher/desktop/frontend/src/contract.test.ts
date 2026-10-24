import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect,Exit} from 'effect';
import {decodeSnapshot,safeFailure,scaffoldFailure,type Command} from './contract';

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

// C-PD8: the displays and the sizes that fit each, as `DesktopState::inspect` sends them.
const withDisplays=()=>({...wireSnapshot(),
 displays:[{index:0,main:true,x:0,y:0,width:1710,height:1107,scale_percent:200,refresh_hz:120}],
 display_sizes:[[[1024,768],[1280,720],[1280,800],[1440,900],[1600,900]]]});

test('a snapshot with the displays and their sizes decodes them',()=>{
 const decoded=Effect.runSync(decodeSnapshot(withDisplays()));
 assert.deepEqual(decoded.displays,[{index:0,main:true,x:0,y:0,width:1710,height:1107,scale_percent:200,refresh_hz:120}]);
 assert.deepEqual(decoded.display_sizes,[[[1024,768],[1280,720],[1280,800],[1440,900],[1600,900]]]);
});

test('a display with an unknown field or a size that is not a pair is rejected',()=>{
 assert.equal(decodes(withDisplays()),true);
 const extra=withDisplays();(extra.displays[0] as Record<string,unknown>).name='Built-in Retina Display';
 assert.equal(decodes(extra),false);
 const triple=withDisplays();(triple.display_sizes[0] as unknown[]).push([1,2,3]);
 assert.equal(decodes(triple),false);
});

// C-PD8 (PD-32): the monitor the current display setup remembers.
test('a snapshot with display_monitor decodes it, and null means the main display',()=>{
 const two={...withDisplays(),displays:[...withDisplays().displays,{index:1,main:false,x:1710,y:0,width:1920,height:1080,scale_percent:100,refresh_hz:60}],display_monitor:1};
 assert.equal(Effect.runSync(decodeSnapshot(two)).display_monitor,1);
 assert.equal(Effect.runSync(decodeSnapshot({...withDisplays(),display_monitor:null})).display_monitor,null);
 assert.equal(Effect.runSync(decodeSnapshot(withDisplays())).display_monitor,undefined);
 assert.equal(decodes({...withDisplays(),display_monitor:'1'}),false);
});

test('a save_display_options command may carry a monitor rectangle',()=>{
 const save:Command={command:'save_display_options',schema_version:1,expected_revision:3,
  display:{mode:'windowed',width:1600,height:900},monitor:{x:1710,y:0,width:1920,height:1080}};
 assert.deepEqual(JSON.parse(JSON.stringify(save)).monitor,{x:1710,y:0,width:1920,height:1080});
 const main:Command={command:'save_display_options',schema_version:1,expected_revision:3,display:{mode:'windowed',width:1024,height:768},monitor:null};
 assert.equal(main.command==='save_display_options'&&main.monitor,null);
});
