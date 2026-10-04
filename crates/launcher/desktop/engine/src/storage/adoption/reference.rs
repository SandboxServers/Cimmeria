//! Derive a reference from authenticated local artifacts, in signed patch order.
//! No historical installed-state claims participate in reconstruction.
use super::*;
use crate::{install_progress::ProgressSink, unpack::UnpackSink};
use std::collections::BTreeMap;

pub(super) struct Reference {
    pub _work: tempfile::TempDir,
    pub prepared: PathBuf,
    pub raw: inventory::Index,
    pub index: inventory::Index,
}
pub(super) fn reconstruct(
    parent: &Path,
    release: &VerifiedRelease,
    artifacts: &Artifacts,
    servers: &[crate::client_setup::LoginServer],
    cancel: &CancellationToken,
    progress: ProgressSink,
) -> Result<Reference, Error> {
    let manifest = release.manifest();
    if artifacts.patches.len() != manifest.patches.len() {
        return Err(Error::InvalidArtifact);
    }
    let work = tempfile::Builder::new()
        .prefix(".adoption-reference-")
        .tempdir_in(parent)?;
    let raw = work.path().join("raw");
    std::fs::create_dir(&raw)?;
    let archive = work.path().join("artifact.zip");
    let sink = UnpackSink {
        progress,
        label: "Authenticated adoption reference".into(),
        cancel: cancel.clone(),
    };
    extract(
        &artifacts.seed,
        &archive,
        manifest.seed.size,
        &manifest.seed.sha256,
        &raw,
        cancel,
        &sink,
    )?;
    crate::install_layout::place_bundled_cooked_data(&raw)?;
    for (patch, artifact) in manifest.patches.iter().zip(&artifacts.patches) {
        let destination =
            crate::patch_dest::patch_dest(&raw, patch).map_err(|_| Error::InvalidArtifact)?;
        extract(
            artifact,
            &archive,
            patch.size,
            &patch.sha256,
            &destination,
            cancel,
            &sink,
        )?;
        inventory::scan(&raw, cancel)?;
    }
    let raw_index = inventory::scan(&raw, cancel)?;
    let prepared = work.path().join("prepared");
    std::fs::create_dir(&prepared)?;
    for (path, entry) in &raw_index {
        inventory::copy(&raw.join(path), &prepared.join(path), entry, cancel)?;
    }
    crate::client_setup::prepare(&prepared, servers)?;
    let ledger = crate::state::InstalledState {
        seed_sha256: Some(manifest.seed.sha256.clone()),
        seed_adopted: false,
        applied_patches: manifest.patches.iter().map(|p| p.state_key()).collect(),
    };
    ledger.save(&prepared).map_err(|_| StorageError::Io)?;
    if !super::super::install_worker::content_valid(&prepared, release) {
        return Err(Error::InvalidArtifact);
    }
    let index = inventory::scan(&prepared, cancel)?;
    Ok(Reference {
        _work: work,
        prepared,
        raw: raw_index,
        index,
    })
}
fn extract(
    source: &Path,
    archive: &Path,
    size: u64,
    sha: &str,
    destination: &Path,
    cancel: &CancellationToken,
    sink: &UnpackSink,
) -> Result<(), Error> {
    let entry = inventory::hash(source, cancel)?;
    let actual: String = entry.sha256.iter().map(|b| format!("{b:02x}")).collect();
    if entry.size != size || !actual.eq_ignore_ascii_case(sha) {
        return Err(Error::InvalidArtifact);
    }
    inventory::copy(source, archive, &entry, cancel)?;
    strict_zip(archive)?;
    crate::unpack::unpack(archive, destination, sink).map_err(|_| Error::InvalidArtifact)?;
    std::fs::remove_file(archive)?;
    Ok(())
}
fn strict_zip(path: &Path) -> Result<(), Error> {
    if crate::unpack::detect(path).map_err(|_| Error::UnsupportedArchive)?
        != crate::unpack::ArchiveKind::Zip
    {
        return Err(Error::UnsupportedArchive);
    }
    let mut zip =
        zip::ZipArchive::new(inventory::open(path)?).map_err(|_| Error::InvalidArtifact)?;
    if zip.len() > 200_000 {
        return Err(StorageError::TooLarge.into());
    }
    let mut names = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|_| Error::InvalidArtifact)?;
        let path = entry.enclosed_name().ok_or(Error::InvalidArtifact)?;
        let name = path.to_str().ok_or(Error::InvalidArtifact)?.to_string();
        if !name.is_ascii()
            || name.contains('\\')
            || name.is_empty()
            || name.len() > 4096
            || name.contains(':')
            || name
                .split('/')
                .any(|s| s == ".." || s == "." || s.is_empty())
        {
            return Err(Error::InvalidArtifact);
        }
        if let Some(mode) = entry.unix_mode() {
            if !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000) {
                return Err(Error::InvalidArtifact);
            }
        }
        if names
            .insert(name.to_ascii_lowercase(), entry.is_dir())
            .is_some()
        {
            return Err(Error::InvalidArtifact);
        }
        total = total
            .checked_add(entry.size())
            .ok_or(StorageError::TooLarge)?;
        if total > 200 * 1024 * 1024 * 1024 {
            return Err(StorageError::TooLarge.into());
        }
    }
    for name in names.keys() {
        let mut parent = Path::new(name).parent();
        while let Some(path) = parent {
            if names.get(path.to_str().ok_or(Error::InvalidArtifact)?) == Some(&false) {
                return Err(Error::InvalidArtifact);
            }
            parent = path.parent();
        }
    }
    Ok(())
}
