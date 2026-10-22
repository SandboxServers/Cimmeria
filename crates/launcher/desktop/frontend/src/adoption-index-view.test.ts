import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {indexText,renderIndex} from './adoption-index-view';
import type {AdoptionStatus} from './adoption-workflow';

type Light=NonNullable<NonNullable<AdoptionStatus['review']>['light']>;
const light=(over:Partial<Light>={}):Light=>({patches_total:1,patches_current:0,patches_pending:1,download_bytes:0,skipped:[],skipped_total:0,index_state:'compared',index_counts:{},index_files:[],index_files_total:0,...over});
const mountIndex=(review:Light|undefined)=>{
 const {document}=parseHTML('<html><body><p id="adoption-light-skipped"></p></body></html>');
 renderIndex(document,document.getElementById('adoption-light-skipped')!,review);
 return {document,get:(id:string)=>document.getElementById(id)!};
};

test('a compared folder names each count, and a count missing from the list is 0',()=>{
 assert.equal(indexText(light({index_counts:{stock:3,patched:1,changed:2,extra:4}})),'Compared with the published file list: 3 original, 1 already patched, 0 prepared by the launcher, 2 changed, 0 missing, 4 extra.');
});

test('each index state that did not compare says why, and lists no files',()=>{
 assert.equal(indexText(light({index_state:'index_not_published'})),'No published file list to compare against yet.');
 assert.equal(indexText(light({index_state:'index_unreachable'})),'The file list could not be downloaded; the folder was not compared.');
 assert.equal(indexText(light({index_state:'index_invalid'})),'The published file list failed its check; the folder was not compared.');
 const {get}=mountIndex(light({index_state:'index_unreachable',index_files:[['Working/a.upk','changed']],index_files_total:1}));
 assert.equal(get('adoption-index').textContent,'The file list could not be downloaded; the folder was not compared.');
 assert.equal(get('adoption-index-files').hidden,true);
});

test('the files list shows each path with its class and says how many are not listed',()=>{
 const {get}=mountIndex(light({index_counts:{changed:1,extra:1},index_files:[['Working/a.upk','changed'],['Working/b.lua','extra']],index_files_total:502}));
 assert.equal(get('adoption-index-files').hidden,false);
 assert.equal(get('adoption-index-files').querySelector('summary')!.textContent,'Show the files');
 const items=Array.from(get('adoption-index-files').querySelectorAll('li'),li=>li.textContent);
 assert.deepEqual(items,['Working/a.upk — changed','Working/b.lua — extra','…and 500 more not listed here.']);
});

test('a compared folder with no listed files shows the line and no files list',()=>{
 const {get}=mountIndex(light({index_counts:{stock:40}}));
 assert.equal(get('adoption-index').hidden,false);
 assert.equal(get('adoption-index-files').hidden,true);
});

test('no light review renders no index line and no files list',()=>{
 const {get}=mountIndex(undefined);
 assert.equal(get('adoption-index').hidden,true);
 assert.equal(get('adoption-index-files').hidden,true);
});
