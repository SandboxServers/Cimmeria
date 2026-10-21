import type {AdoptionStatus} from './adoption-workflow';
type Rejection=NonNullable<AdoptionStatus['last_rejection']>;
type IndexFallbackView=NonNullable<NonNullable<AdoptionStatus['review']>['index_fallback']>;
/** One sentence per engine check (REJECTION_CHECKS, C14). Each names the failed check in plain words. */
const checkTexts:Record<string,string>={
 not_a_game_folder:'That folder is not a Stargate Worlds game folder.',
 folder_not_writable:'The launcher cannot write to this folder, so it cannot adopt or copy into it.',
 folder_in_use:'Another launcher has this folder open.',
 folder_overlaps_launcher:'The chosen folder overlaps the launcher’s own folders.',
 folder_unreadable:'A file or folder in the game folder could not be read.',
 import_changed:'The old launcher’s saved settings changed while they were being read.',
 folder_changed_during_check:'The game folder changed while the launcher was checking it.',
 folder_too_large:'The game folder holds more files or data than this launcher will check.',
 folder_too_deep:'The game folder’s folders nest deeper than this launcher will check.',
 path_not_canonical:'A path in the game folder is not in a form the launcher can check safely.',
 unsupported_catalog:'The imported settings use a custom content catalog, which adoption does not support.',
 unsupported_configuration:'The imported settings load a custom patch DLL, which adoption does not support.',
 stale_revision:'Settings or launcher operations changed after this review was prepared.',
 busy:'Another launcher operation is running.',
 destination_exists:'The destination folder already exists.',
 destination_overlaps:'The destination overlaps the original game folder or the launcher’s folders.',
 launcher_too_old:'This release needs a newer launcher than this one.',
 identity_conflict:'A desktop-owned installation already exists, or this request was already used.',
 comparison_failed:'The launcher could not compare your files with the signed release.',
 exe_at_root:'SGW.exe is at a location this launcher cannot adopt.',
 no_reusable_files:'None of the folder’s files match this release.',
 seed_mismatch:'The game archive does not match the signed release.',
 setup_rule_invalid:'A signed setup rule for this release is invalid.',
 file_unclassified:'A file in the game folder could not be classified against the signed release.',
 artifact_missing:'A signed file was missing from its download.',
 artifact_hash_mismatch:'A signed file did not match its download.',
 artifact_size_mismatch:'A signed file’s size did not match its download.',
 download_http_status:'The server refused the signed download.',
 download_failed:'The signed download failed before it finished.',
 local_seed_invalid:'The game archive you chose is not a valid Stargate Worlds archive.',
 local_artifact_mismatch:'A file in your game archive did not match the signed release.',
 manifest_digest_malformed:'The signed release’s digest is malformed.',
 patch_replay_mismatch:'Replaying a signed patch did not produce the signed release file.',
 patch_archive_unsupported:'A signed patch uses an archive format this launcher cannot read.',
 patch_input_unexpected:'A signed patch received an input it does not expect.',
 patch_layout_unsupported:'A signed patch uses a folder layout this launcher cannot apply.',
 leftover_launcher_files:'The folder already holds launcher files from an earlier installation or adoption.',
 unsupported_layout:'The game folder uses a layout this launcher cannot adopt in place.',
 invalid_directory:'The chosen location cannot be used for this adoption.',
 reference_zip_invalid:'The signed reference archive is damaged or cannot be read.',
 worker_stopped:'The adoption worker stopped before it finished.',
 blocking_files:'Files in the game folder block this adoption.',
 import_required:'The old launcher’s settings must be imported first.',
 state_corrupt:'The saved adoption state is damaged.',
 release_unavailable:'The signed release could not be read from the catalog.',
 platform_unavailable:'Verified copy adoption is not available in this build.',
 recovery_required:'An interrupted adoption must be resolved first.',
};
// Checks the engine raises for the file index, not for the player's folder (index_source.rs build_error).
const indexTexts:Record<string,string>={
 folder_unreadable:'The launcher could not read or store the file index it built.',
 folder_too_large:'The file index is larger than this launcher will accept.',
 identity_conflict:'The file index does not belong to this release.',
 path_not_canonical:'The file index names a path the launcher cannot use safely.',
 file_unclassified:'The file index lists the same file twice.',
 artifact_hash_mismatch:'The file index did not match its signed digest.',
};
/** The sentence for a check, or `Check: <check>.` for one this build does not know. */
export function rejectionText(r:Rejection):string {
 const sentence=r.entry==='file_index'&&Object.prototype.hasOwnProperty.call(indexTexts,r.check)?indexTexts[r.check]
  :r.check==='unsupported_configuration'&&r.found!==null&&r.found.startsWith('setup policy')?'The file index uses a setup policy this launcher does not support.'
  :Object.prototype.hasOwnProperty.call(checkTexts,r.check)?checkTexts[r.check]:`Check: ${r.check}.`;
 const parts=[sentence];
 if(r.entry!==null)parts.push(`File: ${r.entry}.`);
 if(r.expected!==null&&r.found!==null)parts.push(`Expected ${r.expected}, found ${r.found}.`);
 else if(r.expected!==null)parts.push(`Expected ${r.expected}.`);
 else if(r.found!==null)parts.push(`Found ${r.found}.`);
 return parts.join(' ');
}
const indexReasons:Record<string,string>={
 index_not_published:'This release has no published file index,',
 index_reference_malformed:'The published file index is malformed,',
 index_download_failed:'The published file index could not be downloaded,',
 index_size_mismatch:'The published file index has the wrong size,',
 index_digest_mismatch:'The published file index does not match its signed digest,',
 index_parse_failed:'The published file index could not be read,',
 index_bind_failed:'The published file index does not belong to this release,',
 index_read_failed:'The published file index could not be read from disk,',
};
/** Why the launcher downloads the signed archive instead of using the file index, with the size it downloads. */
export function fallbackText(f:IndexFallbackView):string {
 const reason=Object.prototype.hasOwnProperty.call(indexReasons,f.reason)?indexReasons[f.reason]:'The published file index could not be used,';
 const gigabytes=(f.seed_bytes/1_000_000_000).toFixed(1);
 return `${reason} so the launcher downloads the signed game archive (${gigabytes} GB) to check your files. To skip that, use your game archive.`;
}
