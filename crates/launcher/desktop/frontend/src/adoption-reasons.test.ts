import test from 'node:test';
import assert from 'node:assert/strict';
import {fallbackText,rejectionText} from './adoption-reasons';
// The engine's REJECTION_CHECKS (C14). A check added there must get its own sentence here.
const checks=['not_a_game_folder','several_game_folders','folder_not_writable','folder_in_use','folder_overlaps_launcher',
 'folder_unreadable','import_changed','folder_changed_during_check','folder_too_large',
 'folder_too_deep','path_not_canonical','unsupported_catalog','unsupported_configuration',
 'stale_revision','busy','destination_exists','destination_overlaps','launcher_too_old',
 'identity_conflict','comparison_failed','exe_at_root',
 'no_reusable_files','seed_mismatch','setup_rule_invalid','file_unclassified','login_servers_invalid',
 'artifact_missing','artifact_hash_mismatch','artifact_size_mismatch',
 'download_http_status','download_failed','local_seed_invalid','local_artifact_mismatch',
 'manifest_digest_malformed','patch_replay_mismatch','patch_archive_unsupported',
 'patch_input_unexpected','patch_layout_unsupported',
 'leftover_launcher_files','unsupported_layout','invalid_directory','reference_zip_invalid',
 'worker_stopped','blocking_files',
 'import_required','state_corrupt','release_unavailable','platform_unavailable',
 'recovery_required'];
const bare=(check:string)=>({check,entry:null,expected:null,found:null,http_status:null});
test('every engine check has its own sentence, not a bare code',()=>{
 for(const check of checks){
  const text=rejectionText(bare(check));
  assert.notEqual(text,`Check: ${check}.`,check);
  assert.doesNotMatch(text,/^Check: /,check);
  assert.match(text,/\.$/,check);
 }
});
test('an unknown check shows its code and never the platform text',()=>{
 assert.equal(rejectionText(bare('future_check')),'Check: future_check.');
});
test('the file, expected and found parts follow the sentence when present',()=>{
 assert.equal(rejectionText({check:'destination_exists',entry:'Stargate Worlds',expected:'a new folder',found:'an existing folder',http_status:null}),
  'The destination folder already exists. File: Stargate Worlds. Expected a new folder, found an existing folder.');
 assert.equal(rejectionText({check:'artifact_size_mismatch',entry:null,expected:'100',found:null,http_status:null}),
  'A signed file’s size did not match its download. Expected 100.');
 assert.equal(rejectionText({check:'download_http_status',entry:'seed',expected:null,found:null,http_status:500}),
  'The server refused the signed download. File: seed.');
});
test('the picked folder itself, entry ".", names no file; a file entry still does',()=>{
 assert.equal(rejectionText({check:'several_game_folders',entry:'.',expected:'one game folder',found:'2',http_status:null}),
  'This folder holds more than one Cimmeria install. Pick the one you want. Expected one game folder, found 2.');
 assert.equal(rejectionText({check:'destination_exists',entry:'Working',expected:null,found:null,http_status:null}),
  'The destination folder already exists. File: Working.');
});
test('a hash mismatch reads as the signed file did not match its download',()=>{
 assert.equal(rejectionText(bare('artifact_hash_mismatch')),'A signed file did not match its download.');
});
test('the index fallback names the archive size and the way to skip it',()=>{
 assert.equal(fallbackText({reason:'index_not_published',check:null,http_status:null,seed_bytes:4_100_000_000}),
  'This release has no published file index, so the launcher downloads the signed game archive (4.1 GB) to check your files. To skip that, use your game archive.');
});
test('a file index failure names the index, not the player folder',()=>{
 assert.equal(rejectionText({check:'identity_conflict',entry:'file_index',expected:null,found:null,http_status:null}),
  'The file index does not belong to this release. File: file_index.');
 assert.equal(rejectionText({check:'identity_conflict',entry:null,expected:null,found:null,http_status:null}),
  'A desktop-owned installation already exists, or this request was already used.');
 assert.equal(rejectionText({check:'unsupported_configuration',entry:null,expected:null,found:'setup policy 9',http_status:null}),
  'The file index uses a setup policy this launcher does not support. Found setup policy 9.');
});
test('an unknown fallback reason still says the archive is downloaded',()=>{
 assert.match(fallbackText({reason:'index_new_reason',check:null,http_status:null,seed_bytes:1}),/could not be used.*downloads the signed game archive \(0\.0 GB\)/);
});
