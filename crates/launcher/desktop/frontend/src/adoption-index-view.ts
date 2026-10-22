import type {AdoptionStatus} from './adoption-workflow';

type Light=NonNullable<NonNullable<AdoptionStatus['review']>['light']>;

// The counts the comparison reports, in the order the sentence names them.
const countNames=['stock','patched','prepared','changed','missing','extra'] as const;
const notPublished='No published file list to compare against yet.';
const unreachable='The file list could not be downloaded; the folder was not compared.';
const invalid='The published file list failed its check; the folder was not compared.';

/** The sentence for the index comparison state; the empty string when there is none to show. */
export function indexText(light:Light):string {
 switch(light.index_state){
  case 'compared': {
   const [stock,patched,prepared,changed,missing,extra]=countNames.map(name=>light.index_counts[name]??0);
   return `Compared with the published file list: ${stock} original, ${patched} already patched, ${prepared} prepared by the launcher, ${changed} changed, ${missing} missing, ${extra} extra.`;
  }
  case 'index_not_published':return notPublished;
  case 'index_unreachable':return unreachable;
  case 'index_invalid':return invalid;
 }
}

// A line placed right after its anchor, created once.
const elementAfter=(document:Document,id:string,tag:string,anchor:Element)=>{
 let el=document.getElementById(id);
 if(!el){el=document.createElement(tag);el.id=id;anchor.parentNode?.insertBefore(el,anchor.nextSibling);}
 return el;
};

/**
 * Shows the index comparison after the light review's skipped list (LA-18). Renders nothing
 * when there is no light review, and lists the files only when the folder was compared.
 */
export function renderIndex(document:Document,anchor:Element,light:Light|undefined):void {
 const line=elementAfter(document,'adoption-index','p',anchor),files=elementAfter(document,'adoption-index-files','details',line);
 line.hidden=files.hidden=!light;
 if(!light)return;
 line.textContent=indexText(light);
 const shown=light.index_state==='compared'&&light.index_files.length>0;
 files.hidden=!shown;
 if(!shown){files.replaceChildren();return;}
 const summary=document.createElement('summary'),list=document.createElement('ul');
 summary.textContent='Show the files';
 const rows=light.index_files.map(([path,fileClass])=>`${path} — ${fileClass}`);
 if(light.index_files_total>light.index_files.length)rows.push(`…and ${light.index_files_total-light.index_files.length} more not listed here.`);
 list.replaceChildren(...rows.map(text=>{const item=document.createElement('li');item.textContent=text;return item;}));
 files.replaceChildren(summary,list);
}
