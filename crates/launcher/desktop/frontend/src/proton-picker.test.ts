import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountProton,protonSavedText} from './proton-picker';
const html=readFileSync('ui/index.html','utf8');
const experimental={id:'valve',name:'Proton Experimental',kind:'steam'};
const ge={id:'ge',name:'GE-Proton 9-2',kind:'compat'};
// A fake `proton_command`: `known` is what discovery finds now, `saved` is the stored choice.
function mounted(known:(typeof experimental)[],saved:string|null=null) {
 const {document,window}=parseHTML(html);const calls:Record<string,unknown>[]=[];
 let stored=saved;let found=known;
 const invoke=async(command:string,args?:Record<string,unknown>)=>{
  if(command!=='proton_command')throw new Error(`unexpected ${command}`);
  const request=args!.request as Record<string,unknown>;calls.push(request);
  if(request.op==='select'){
   if(request.id!==null&&!found.some(p=>p.id===request.id))throw 'unknown_proton';
   stored=request.id as string|null;
  }
  return {available:found,selected:stored&&found.some(p=>p.id===stored)?stored:found[0]?.id??null,preferred:stored};
 };
 const host=document.getElementById('display-panel') as unknown as HTMLElement;
 const app=mountProton(document as unknown as Document,invoke,host);
 const select=document.getElementById('proton-select') as unknown as HTMLSelectElement;
 const section=document.getElementById('proton-picker')!;
 const status=document.getElementById('proton-status')!;
 const choose=(value:string)=>{(select.querySelector(`option[value="${value}"]`) as HTMLOptionElement).selected=true;select.dispatchEvent(new window.Event('change'));};
 const options=()=>Array.from(select.querySelectorAll('option')).map(o=>[o.getAttribute('value'),o.textContent]);
 return {app,calls,select,section,status,choose,options,setFound:(next:(typeof experimental)[])=>{found=next;}};
}
test('the picker stays hidden when no Proton is found',async()=>{
 const m=mounted([]);await m.app.ready;
 assert.equal(m.section.hidden,true);
});
test('the picker lists Automatic and then each Proton by name, selecting the saved one',async()=>{
 const m=mounted([experimental,ge],'ge');await m.app.ready;
 assert.equal(m.section.hidden,false);
 assert.deepEqual(m.options(),[['','Automatic'],['valve','Proton Experimental'],['ge','GE-Proton 9-2']]);
 assert.equal(m.select.value,'ge');
});
test('the first change saves at once with visible feedback, and Automatic sends a null id',async()=>{
 const m=mounted([experimental,ge],'valve');await m.app.ready;
 m.choose('ge');await m.app.settled();
 assert.deepEqual(m.calls.at(-1),{op:'select',id:'ge'});
 assert.equal(m.status.textContent,protonSavedText);
 m.choose('');await m.app.settled();
 assert.deepEqual(m.calls.at(-1),{op:'select',id:null});
 assert.equal(m.status.textContent,protonSavedText);
});
test('a refused choice says why and puts the previous selection back',async()=>{
 const m=mounted([experimental,ge],'valve');await m.app.ready;
 m.setFound([experimental]);
 m.choose('ge');await m.app.settled();
 assert.match(m.status.textContent??'',/isn't installed any more/);
 assert.equal(m.select.value,'valve');
});
test('a reply that does not decode hides the picker rather than offering a bad list',async()=>{
 const {document}=parseHTML(html);
 const app=mountProton(document as unknown as Document,async()=>({available:[{id:'x'}],selected:null,preferred:null}),document.getElementById('display-panel') as unknown as HTMLElement);
 await app.ready;
 assert.equal(document.getElementById('proton-picker')!.hidden,true);
});
