import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect} from 'effect';
import {decodeReport,preflightWorkflow} from './preflight-workflow';

const check=(over:Record<string,unknown>={})=>({id:'rosetta',status:'missing',action:'auto_install',failure:'rosetta_missing',blocking:true,bytes_needed:null,bytes_free:null,download_bytes:null,...over});
const report=(over:Record<string,unknown>={})=>({schema_version:1,operation:'play',checks:[check()],ready:false,...over});
const decode=(value:unknown)=>Effect.runPromise(decodeReport(value));

test('a native preflight report decodes',async()=>{
 const decoded=await decode(report({checks:[check(),check({id:'disk_space',status:'warning',action:'manual',failure:'disk_space_low',blocking:false,bytes_needed:1e9,bytes_free:5e8})]}));
 assert.equal(decoded.operation,'play');assert.equal(decoded.ready,false);
 assert.deepEqual(decoded.checks.map(c=>[c.id,c.failure,c.bytes_free]),[['rosetta','rosetta_missing',null],['disk_space','disk_space_low',5e8]]);
});

test('a report with an extra key, an unknown check or a missing byte field is rejected',async()=>{
 await assert.rejects(decode({...report(),extra:true}));
 await assert.rejects(decode(report({checks:[{...check(),extra:1}]})));
 await assert.rejects(decode(report({checks:[check({id:'webview2'})]})));
 const {download_bytes:_bytes,...withoutBytes}=check();
 await assert.rejects(decode(report({checks:[withoutBytes]})));
 await assert.rejects(decode(report({schema_version:2})));
});

test('the workflow sends preflight_command requests and maps a JobError to its failure code',async()=>{
 const sent:unknown[]=[];
 const workflow=preflightWorkflow(async(command,args)=>{assert.equal(command,'preflight_command');sent.push(args!.request);
  if((args!.request as {command:string}).command==='install')throw 'busy';return report();});
 assert.equal((await Effect.runPromise(workflow.inspectPreflight('repair'))).ready,false);
 const failure=await Effect.runPromise(Effect.flip(workflow.installDependency('play','rosetta')));
 assert.equal(failure.code,'operation_busy');
 assert.deepEqual(sent,[{command:'inspect',schema_version:1,operation:'repair'},{command:'install',schema_version:1,operation:'play',check:'rosetta'}]);
 const garbled=preflightWorkflow(async()=>({...report(),extra:true}));
 assert.equal((await Effect.runPromise(Effect.flip(garbled.inspectPreflight('play')))).code,'schema');
 const thrown=preflightWorkflow(async()=>{throw new Error('ipc closed');});
 assert.equal((await Effect.runPromise(Effect.flip(thrown.inspectPreflight('play')))).code,'transport');
});
