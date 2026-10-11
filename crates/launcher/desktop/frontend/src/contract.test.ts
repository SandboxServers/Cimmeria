import test from 'node:test';
import assert from 'node:assert/strict';
import {safeFailure,scaffoldFailure} from './contract';

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
