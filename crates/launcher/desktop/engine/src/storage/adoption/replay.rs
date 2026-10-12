//! The minimal replay an adopted folder needs (any-install AI-09). Planning:
//! which signed patches and which seed bytes the replacement store must
//! produce, and which of the folder's own files already hold a needed
//! pre-patch state. [`replay`] then rebuilds the target bytes in a scratch tree
//! and stores them; it fetches nothing itself.
use super::artifacts::Fetched;
use super::*;
use crate::{manifest::Manifest, unpack::UnpackSink};
use cimmeria_launcher_core::file_index::{FileIndex, IndexedFile, Origin, PathHistory};
use std::collections::{BTreeMap, BTreeSet};

/// What the replay needs to rebuild the release bytes of every target.
#[allow(dead_code)] // AI-09 part b wires it
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplayPlan {
    /// Some needed pre-state is neither in the folder nor made by a patch.
    pub needs_seed: bool,
    /// Manifest patch indices, in signed order.
    pub patches: Vec<usize>,
    /// Stock path to the folder file that already holds its needed pre-state.
    pub local: BTreeMap<String, String>,
    /// Stock paths whose release bytes the replay produces.
    pub targets: Vec<String>,
}

/// Plans the replay for `files` (the classification of the folder). Targets are
/// `NeedsPatching`, `Modified` and `Missing`, plus a `KnownTransform` at another
/// spelling in copy mode.
#[allow(dead_code)] // AI-09 part b wires it
pub(super) fn plan(
    index: &FileIndex,
    files: &[Difference],
    scanned: &inventory::Index,
    mode: AdoptionMode,
) -> ReplayPlan {
    let histories = index.histories();
    let targets: Vec<String> = files
        .iter()
        .filter(|d| is_target(d, mode))
        .map(|d| d.path.clone())
        .collect();
    let mut planner = Planner {
        index,
        histories: &histories,
        scanned,
        folded: scanned
            .keys()
            .map(|name| (name.to_ascii_lowercase(), name))
            .collect(),
        patches: BTreeSet::new(),
        local: BTreeMap::new(),
        needs_seed: false,
    };
    for path in &targets {
        if let Some(release) = histories
            .get(&path.to_ascii_lowercase())
            .and_then(|history| history.states.last())
        {
            planner.need_state(release.origin);
        }
    }
    ReplayPlan {
        needs_seed: planner.needs_seed,
        patches: planner.patches.into_iter().collect(),
        local: planner.local,
        targets,
    }
}

/// The download a replay plan implies. Artifacts `stored` reports as on disk
/// are not downloaded. A `have_local_seed` replaces the seed download, and the
/// review then names the player's archive.
#[allow(dead_code)] // AI-09 part b wires it
pub(super) fn estimate(
    plan: &ReplayPlan,
    manifest: &Manifest,
    have_local_seed: bool,
    stored: impl Fn(&str) -> bool,
) -> DownloadEstimate {
    let mut bytes = 0;
    let mut patches = Vec::new();
    for &k in &plan.patches {
        // `FileIndex::bind` has checked the patch list against the manifest.
        let entry = &manifest.patches[k];
        if stored(entry.sha256.as_str()) {
            continue;
        }
        bytes += entry.size;
        patches.push(entry.state_key());
    }
    let seed = plan.needs_seed && !have_local_seed && !stored(manifest.seed.sha256.as_str());
    if seed {
        bytes += manifest.seed.size;
    }
    DownloadEstimate {
        seed,
        patches,
        bytes,
    }
}

fn is_target(difference: &Difference, mode: AdoptionMode) -> bool {
    match difference.classification {
        Classification::NeedsPatching | Classification::Modified | Classification::Missing => true,
        Classification::KnownTransform => {
            mode == AdoptionMode::Copy
                && difference
                    .source_path
                    .as_deref()
                    .is_some_and(|source| source != difference.path)
        }
        Classification::Matched | Classification::Extra => false,
    }
}

/// Walks the needed states backwards from the targets. Patch indices are
/// collected, never revisited.
struct Planner<'a> {
    index: &'a FileIndex,
    histories: &'a BTreeMap<String, PathHistory>,
    scanned: &'a inventory::Index,
    /// Lower-case path to the folder's spelling of the file.
    folded: BTreeMap<String, &'a String>,
    patches: BTreeSet<usize>,
    local: BTreeMap<String, String>,
    needs_seed: bool,
}

impl Planner<'_> {
    /// The state `origin` names: the seed, or the patch that wrote it.
    fn need_state(&mut self, origin: Origin) {
        match origin {
            Origin::Seed => self.needs_seed = true,
            Origin::Patch(k) => self.need_patch(k),
        }
    }

    /// Patch `k` must run in a scratch tree where each input holds its pre-`k`
    /// bytes: from the folder when a file there matches, else from the earlier
    /// state of that input, which is an earlier patch or the seed.
    fn need_patch(&mut self, k: usize) {
        if !self.patches.insert(k) {
            return;
        }
        let Some(layer) = self.index.patches.get(k) else {
            self.needs_seed = true;
            return;
        };
        let histories = self.histories;
        for input in &layer.inputs {
            let key = input.path.to_ascii_lowercase();
            if let Some(name) = self.folded.get(&key).copied() {
                let entry = &self.scanned[name];
                if entry.size == input.size && hex(&entry.sha256) == input.sha256 {
                    self.local.insert(input.path.clone(), name.clone());
                    continue;
                }
            }
            let Some(history) = histories.get(&key) else {
                self.needs_seed = true;
                continue;
            };
            // The last state written before patch `k` is the pre-`k` state.
            let before = history
                .states
                .iter()
                .rev()
                .find(|state| match state.origin {
                    Origin::Seed => true,
                    Origin::Patch(j) => j < k,
                });
            match before {
                Some(state) => self.need_state(state.origin),
                None => self.needs_seed = true,
            }
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Rebuilds the release bytes of every target in `plan` under `prep_dir`. The
/// seed and patches come from `fetched`; the folder's own needed files are
/// copied from `folder_root`. Each target is checked against the release and
/// stored at `replacements/<path>`, and the stored paths are returned by target.
/// The scratch tree is removed on every outcome.
#[allow(dead_code)] // AI-09 part b2 wires it
#[allow(clippy::too_many_arguments)]
pub(super) fn replay(
    plan: &ReplayPlan,
    index: &FileIndex,
    release: &VerifiedRelease,
    fetched: &Fetched,
    folder_root: &Path,
    scanned: &inventory::Index,
    prep_dir: &Path,
    cancel: &CancellationToken,
    sink: &UnpackSink,
) -> Result<BTreeMap<String, PathBuf>, Error> {
    // A Wine adapter would be needed to run the seed's RAR-era unpack on macOS.
    #[cfg(target_os = "macos")]
    {
        if plan.needs_seed {
            return Err(Error::UnsupportedMode);
        }
    }
    let scratch = prep_dir.join("scratch");
    std::fs::create_dir_all(&scratch)?;
    let outcome = (|| -> Result<BTreeMap<String, PathBuf>, Error> {
        let manifest = release.manifest();
        if plan.needs_seed {
            let seed = fetched.seed.as_deref().ok_or(Error::InvalidArtifact)?;
            if crate::unpack::detect(seed).map_err(|_| Error::UnsupportedArchive)?
                == crate::unpack::ArchiveKind::Zip
            {
                super::reference::strict_zip(seed)?;
            }
            crate::unpack::unpack(seed, &scratch, sink).map_err(|_| Error::InvalidArtifact)?;
        }
        for (stock, folder_file) in &plan.local {
            let known = scanned.get(folder_file).ok_or(Error::SourceChanged)?;
            let source = inventory::native(folder_root, folder_file);
            let entry = inventory::hash(&source, cancel)?;
            if entry.size != known.size || entry.sha256 != known.sha256 {
                return Err(Error::SourceChanged);
            }
            // The folder file must be the pre-patch state some planned patch reads.
            let hash = hex(&entry.sha256);
            let wanted = plan
                .patches
                .iter()
                .filter_map(|&k| index.patches.get(k))
                .flat_map(|layer| layer.inputs.iter())
                .any(|input| {
                    input.path.eq_ignore_ascii_case(stock)
                        && input.size == entry.size
                        && input.sha256.eq_ignore_ascii_case(&hash)
                });
            if !wanted {
                return Err(Error::InvalidArtifact);
            }
            inventory::copy(&source, &inventory::native(&scratch, stock), &entry, cancel)?;
        }
        for &k in &plan.patches {
            let patch = manifest.patches.get(k).ok_or(Error::InvalidArtifact)?;
            let archive = fetched
                .patches
                .get(k)
                .and_then(Option::as_deref)
                .ok_or(Error::InvalidArtifact)?;
            let dest = crate::patch_dest::patch_dest(&scratch, patch)
                .map_err(|_| Error::UnsupportedLayout)?;
            super::reference::strict_zip(archive)?;
            std::fs::create_dir_all(&dest)?;
            crate::unpack::unpack(archive, &dest, sink).map_err(|_| Error::InvalidArtifact)?;
        }
        let mut stored = BTreeMap::new();
        for target in &plan.targets {
            let expected = release_file(index, target).ok_or(Error::InvalidArtifact)?;
            let built = inventory::native(&scratch, target);
            let entry = inventory::hash(&built, cancel)?;
            if entry.size != expected.size
                || !hex(&entry.sha256).eq_ignore_ascii_case(&expected.sha256)
            {
                return Err(Error::InvalidArtifact);
            }
            let dest = inventory::native(&prep_dir.join("replacements"), target);
            inventory::copy(&built, &dest, &entry, cancel)?;
            stored.insert(target.clone(), dest);
        }
        Ok(stored)
    })();
    // Best effort: the scratch tree holds only copies of verified bytes.
    let _ = std::fs::remove_dir_all(&scratch);
    outcome
}

/// The release's file at `path`: the state the index gives it last, found in the
/// seed or in the patch that wrote it.
fn release_file<'a>(index: &'a FileIndex, path: &str) -> Option<&'a IndexedFile> {
    let origin = index
        .histories()
        .get(&path.to_ascii_lowercase())?
        .states
        .last()?
        .origin;
    let files = match origin {
        Origin::Seed => &index.seed.files,
        Origin::Patch(k) => &index.patches.get(k)?.outputs,
    };
    files
        .iter()
        .find(|file| file.path.eq_ignore_ascii_case(path))
}
