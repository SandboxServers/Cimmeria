import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {makePanelContext} from './panel-context';
import {Effect} from 'effect';
import {decodeLogUploadStatus} from './contract';
import {mountLogUpload,logUploadErrorText,logUploadText,logUploadUploading} from './log-upload-view';
const html=readFileSync('ui/index.html','utf8');
type Request={command:string;schema_version:number};
type Answer={state?:string;error?:string;fields?:Record<string,unknown>};
/** A fake native host: `answers` maps each command to a status state (plus its extra fields) or a thrown code. */
function mount(answers:Record<string,Answer>) {
 const {document,window}=parseHTML(html);const calls:Request[]=[];let release:()=>void=()=>{};
 const gate=new Promise<void>(resolve=>{release=resolve;});
 const invoke=async(command:string,args?:Record<string,unknown>)=>{assert.equal(command,'log_upload_command');const request=args!.request as Request;calls.push(request);
  if(request.command==='upload')await gate;const answer=answers[request.command]!;if(answer.error)throw answer.error;return {schema_version:1,state:answer.state,...answer.fields};};
 const app=mountLogUpload(document as unknown as Document,invoke,makePanelContext().context);
 const button=()=>document.getElementById('log-upload-button') as HTMLButtonElement;
 return {document,app,calls,release,button,press:()=>button().dispatchEvent(new window.Event('click')),text:()=>document.getElementById('log-upload-status')!.textContent??''};
}

test('the first press shows Uploading at once, sends one upload, then says Uploaded',async()=>{
 const ui=mount({inspect:{state:'ready'},upload:{state:'uploaded'}});await ui.app.ready;
 assert.equal(ui.button().hidden,false);assert.equal(ui.button().disabled,false);
 ui.press();ui.press();
 assert.equal(ui.text(),logUploadUploading);assert.equal(ui.button().disabled,true);assert.equal(ui.button().getAttribute('aria-busy'),'true');
 ui.release();await ui.app.settled();
 assert.equal(ui.text(),'Uploaded');assert.equal(ui.button().disabled,false);
 assert.deepEqual(ui.calls,[{command:'inspect',schema_version:1},{command:'upload',schema_version:1}]);
 await ui.app.dispose();assert.equal(ui.document.getElementById('log-upload-title'),null);
});

test('a repeat of the same logs says there is nothing new',async()=>{
 const ui=mount({inspect:{state:'ready'},upload:{state:'nothing_new'}});await ui.app.ready;
 ui.press();ui.release();await ui.app.settled();
 assert.equal(ui.text(),'Nothing new to upload');await ui.app.dispose();
});

test('a build with no log store hides the button and sends no upload',async()=>{
 const ui=mount({inspect:{state:'disabled'},upload:{state:'uploaded'}});await ui.app.ready;
 assert.equal(ui.button().hidden,true);ui.press();ui.release();await ui.app.settled();
 assert.deepEqual(ui.calls.map(c=>c.command),['inspect']);await ui.app.dispose();
});

test('a busy refusal tells the player to finish or reconcile the running operation',async()=>{
 const ui=mount({inspect:{state:'ready'},upload:{error:'busy'}});await ui.app.ready;
 ui.press();ui.release();await ui.app.settled();
 assert.equal(ui.text(),logUploadErrorText.busy);assert.match(ui.text(),/Another log upload is still running/);
 assert.equal(ui.button().disabled,false);await ui.app.dispose();
});

test('a finished operation re-inspects, other snapshots do not',async()=>{
 const {document}=parseHTML(html);const feed=makePanelContext();let state='not_installed';const calls:string[]=[];
 const app=mountLogUpload(document as unknown as Document,async(_c,args)=>{calls.push((args!.request as Request).command);return {schema_version:1,state};},feed.context);
 await app.ready;const button=document.getElementById('log-upload-button') as HTMLButtonElement;assert.equal(button.disabled,true);
 const snapshot=(op:string|null)=>({operation:{operation:op?{id:'install',state:op}:null}}) as unknown as Parameters<typeof feed.publish>[0];
 feed.publish(snapshot('running'));feed.publish(snapshot('running'));assert.equal(calls.length,1);
 state='ready';feed.publish(snapshot('succeeded'));await app.settled();
 assert.equal(calls.length,2);assert.equal(button.disabled,false);await app.dispose();
});

test('an undecodable status is reported to telemetry and still shows the schema error',async()=>{
 const uiCalls:unknown[]=[];
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
 try{
  const ui=mount({inspect:{}});await ui.app.ready;
  assert.deepEqual(uiCalls,[['ui_event',{area:'log_upload',event:'status_decode_failed',code:'schema',level:'warn'}]]);
  assert.equal(ui.text(),logUploadErrorText.schema);await ui.app.dispose();
 }finally{Reflect.deleteProperty(globalThis,'window');}
});

test('a network failure is shown, and an unknown rejection is a transport failure',async()=>{
 const ui=mount({inspect:{state:'ready'},upload:{error:'network'}});await ui.app.ready;
 ui.press();ui.release();await ui.app.settled();assert.equal(ui.text(),logUploadErrorText.network);await ui.app.dispose();
 const odd=mount({inspect:{error:'https://store.example/c?sig=secret'},upload:{state:'uploaded'}});await odd.app.ready;
 assert.equal(odd.text(),logUploadErrorText.transport);await odd.app.dispose();
});

test('a later re-inspect keeps the result while ready, and replaces it when the state changes',async()=>{
 const answers:Record<string,Answer>={inspect:{state:'ready'},upload:{state:'uploaded_newest'}};
 const ui=mount(answers);await ui.app.ready;ui.press();ui.release();await ui.app.settled();
 assert.equal(ui.text(),logUploadText.uploaded_newest);assert.match(ui.text(),/Older logs were left out/);
 await ui.app.refresh();assert.equal(ui.text(),logUploadText.uploaded_newest);
 answers.inspect={state:'not_installed'};await ui.app.refresh();
 assert.equal(ui.text(),logUploadText.not_installed);assert.equal(ui.button().disabled,true);
 answers.inspect={state:'ready'};await ui.app.refresh();assert.equal(ui.text(),logUploadText.ready);
 answers.inspect={error:'busy'};ui.press();await ui.app.settled();await ui.app.refresh();
 assert.equal(ui.text(),logUploadErrorText.busy);await ui.app.dispose();
});
const decode=(value:unknown)=>Effect.runPromise(decodeLogUploadStatus(value));
test('the decoder accepts sent, partial and unreachable, and rejects a field the state does not carry',async()=>{
 assert.deepEqual(await decode({schema_version:1,state:'sent',lines_sent:3,lines_total:3,upload_id:'0123456789abcdef'}),{schema_version:1,state:'sent',lines_sent:3,lines_total:3,upload_id:'0123456789abcdef'});
 assert.equal((await decode({schema_version:1,state:'partial',lines_sent:1,lines_total:3,upload_id:'0123456789abcdef',retry_after_secs:60})).state,'partial');
 assert.equal((await decode({schema_version:1,state:'unreachable'})).state,'unreachable');
 await assert.rejects(decode({schema_version:1,state:'unreachable',lines_sent:1}));
 await assert.rejects(decode({schema_version:1,state:'sent',lines_sent:3,lines_total:3}));
 await assert.rejects(decode({schema_version:1,state:'sent',lines_sent:3,lines_total:3,upload_id:'x',extra:1}));
});

test('sent and partial show the counts and the upload id; unreachable keeps the button usable',async()=>{
 const sent={lines_sent:5,lines_total:5,upload_id:'0123456789abcdef'};
 const ui=mount({inspect:{state:'ready'},upload:{state:'sent',fields:sent}});await ui.app.ready;
 ui.press();ui.release();await ui.app.settled();
 assert.match(ui.text(),/Sent 5 of 5 log lines/);assert.match(ui.text(),/0123456789abcdef/);assert.equal(ui.button().disabled,false);
 const part=mount({inspect:{state:'ready'},upload:{state:'partial',fields:{...sent,lines_sent:2,retry_after_secs:60}}});await part.app.ready;
 part.press();part.release();await part.app.settled();
 assert.match(part.text(),/Sent 2 of 5 log lines/);assert.match(part.text(),/in 60 seconds/);
 const down=mount({inspect:{state:'unreachable'},upload:{state:'unreachable'}});await down.app.ready;
 assert.equal(down.text(),logUploadText.unreachable);assert.equal(down.button().hidden,false);assert.equal(down.button().disabled,false);
 await ui.app.dispose();await part.app.dispose();await down.app.dispose();
});
