import test from 'node:test';
import assert from 'node:assert/strict';
import { Effect, Fiber, Result } from 'effect';
import { TestClock } from 'effect/testing';
import { installBridgeLayer, InstallStatus, makeInstallWorkflow } from './install-workflow';
const id='d539d049-61b7-4c82-b3d7-cb9b7a991adc';
const initial=():InstallStatus=>({schema_version:1,install_supported:true,can_resume:true,can_reconcile:true,can_retry:false,uninstall:null,progress:null,outcome:null,native:{
  schema_version:1,requires_reopen:false,preferences:{schema_version:1,revision:1,install_directory:'/fixture',launcher_summary_consent:false},
  operation:{schema_version:1,revision:0,operation:null},
}});
const running=():InstallStatus=>({...initial(),native:{...initial().native,operation:{schema_version:1,revision:1,
  operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'running'}}}});

test('lost install reply is not replayed; subsequent inspection sees owned work',async()=>{
  let current=initial(); const commands:string[]=[];
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const workflow=yield* makeInstallWorkflow;
    const result=yield* Effect.result(workflow.install(id));
    assert.ok(Result.isFailure(result)); assert.equal(result.failure.code,'transport');
    assert.equal((yield* workflow.snapshot).needsInspection,true);
    const recovered=yield* workflow.inspect;
    assert.equal(recovered.native.operation.operation?.id,id);
    assert.deepEqual(commands,['inspect','install','inspect']);
  }).pipe(Effect.provide(installBridgeLayer(async request=>{
    commands.push(request.command);
    if(request.command==='install'){current=running();throw new Error('private path must not escape');}
    return current;
  })))));
});

test('cancel remains available during observation and waits for native terminal state',{timeout:5000},async()=>{
  let current=running(); let cancels=0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const workflow=yield* makeInstallWorkflow;
    const observing=yield* workflow.observe(id).pipe(Effect.forkScoped);
    yield* TestClock.adjust('250 millis');
    const cancelled=yield* workflow.cancel(id);
    assert.equal(cancelled.native.operation.operation?.state,'cancel_requested');
    current={...current,outcome:'cancelled',native:{...current.native,operation:{...current.native.operation,revision:3,
      operation:{...current.native.operation.operation!,state:'cancelled'}}}};
    yield* TestClock.adjust('1 second');
    const finished=yield* Fiber.join(observing);
    assert.equal(finished.native.operation.operation?.state,'cancelled'); assert.equal(cancels,1);
  }).pipe(Effect.provide(installBridgeLayer(async request=>{
    if(request.command==='cancel'){cancels++; current={...current,native:{...current.native,operation:{...current.native.operation,
      revision:2,operation:{...current.native.operation.operation!,state:'cancel_requested'}}}};}
    return current;
  })),Effect.provide(TestClock.layer()))));
});

test('disposing observation never sends cancel or claims rollback',{timeout:5000},async()=>{
  const commands:string[]=[];
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const workflow=yield* makeInstallWorkflow;
    const observer=yield* workflow.observe(id).pipe(Effect.forkScoped);
    yield* TestClock.adjust('250 millis');
    yield* Fiber.interrupt(observer);
    assert.equal((yield* workflow.snapshot).status?.native.operation.operation?.state,'running');
  }).pipe(Effect.provide(installBridgeLayer(async request=>{commands.push(request.command);return running();})),
    Effect.provide(TestClock.layer()))));
  assert.ok(commands.length>0); assert.ok(commands.every(command=>command==='inspect'));
});

test('recovery binds current revision; path-bearing progress is rejected',async()=>{
  let calls=0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const workflow=yield* makeInstallWorkflow;
    const result=yield* Effect.result(workflow.resume(id));
    assert.ok(Result.isFailure(result)); assert.equal(result.failure.code,'schema');
    assert.equal((yield* workflow.snapshot).needsInspection,true);
  }).pipe(Effect.provide(installBridgeLayer(async request=>{
    calls++;
    if(request.command==='resume'){
      assert.equal(request.operation_revision,1);
      return {...running(),progress:{phase:'download',current:1,total:2,path:'/private/example'}};
    }
    return running();
  })))));
  assert.equal(calls,2);
});

test('uncertain native persistence stops observation without polling or cancellation',async()=>{
  let calls=0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const workflow=yield* makeInstallWorkflow;
    const result=yield* Effect.result(workflow.observe(id));
    assert.ok(Result.isFailure(result));assert.equal(result.failure.code,'persistence_uncertain');
    assert.equal((yield* workflow.snapshot).needsInspection,true);
  }).pipe(Effect.provide(installBridgeLayer(async request=>{
    calls++;assert.equal(request.command,'inspect');return {...running(),native:{...running().native,requires_reopen:true}};
  })))));
  assert.equal(calls,1);
});
