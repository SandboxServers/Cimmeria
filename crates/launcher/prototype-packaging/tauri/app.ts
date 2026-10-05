// Throwaway packaged UI. State comes from the shared Rust mock, never from downloads.
type Snapshot = {telemetry:boolean;settings:boolean;notes:boolean;status:string};
declare global { interface Window { __TAURI__: {core:{invoke<T>(command:string,args:object):Promise<T>}} } }
const element = (id:string) => document.getElementById(id)!;
function render(s:Snapshot) {
  (element('telemetry') as HTMLInputElement).checked=s.telemetry;
  element('gear').hidden=!s.settings;
  element('patches').hidden=!s.notes;
  element('play').hidden=s.notes;
  element('status').textContent=s.status;
  element('home').setAttribute('aria-pressed',String(!s.notes));
  element('notes').setAttribute('aria-pressed',String(s.notes));
  element('settings').setAttribute('aria-expanded',String(s.settings));
}
let pending=Promise.resolve();
function action(name:string) {
  pending=pending.then(async()=>render(await window.__TAURI__.core.invoke<Snapshot>('action',{name}))).catch(()=>{
    element('status').textContent='Prototype bridge unavailable. No action was taken.';
  });
}
for(const name of ['home','notes','settings','folder','repair','uninstall','install','telemetry']) {
  element(name).addEventListener(name==='telemetry'?'change':'click',()=>action(name));
}
action('snapshot');
fetch('./manifest.json').then(r=>r.json()).then((m:{patches:{id:string;title?:string;description?:string}[]})=>{
  for(const p of m.patches){
    const article=document.createElement('article'), title=document.createElement('h3'), description=document.createElement('p');
    title.textContent=p.title||p.id; description.textContent=p.description||'No description provided in the manifest.';
    article.append(title,description);element('entries').append(article);
  }
}).catch(()=>{element('entries').textContent='Bundled snapshot unavailable.';});
export {};
