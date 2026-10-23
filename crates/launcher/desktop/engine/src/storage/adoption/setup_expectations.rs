//! Plan-bound original and approved setup states, checked before recovery or rollback.
use super::direct_paths::Parents;
use super::in_place::{refused, Hashed, SETUP_FILES};
use super::*;

pub(super) const NAME: &str = "setup-expectations.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expectations {
    version: u32,
    digest: [u8; 32],
    files: Vec<Expected>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    path: String,
    original_path: String,
    original: Option<Hashed>,
    approved: Option<Hashed>,
    rewrite: bool,
}
fn bytes_hash(bytes: &[u8]) -> Hashed {
    Hashed {
        size: bytes.len() as u64,
        sha256: Sha256::digest(bytes).into(),
    }
}
fn path(plan: &Plan) -> PathBuf {
    plan.stage().join(NAME)
}
fn load(plan: &Plan) -> Result<Expectations, Error> {
    let path = path(plan);
    let guard = Parents::new(plan, &[&path], false)?;
    let record: Expectations =
        serde_json::from_slice(&guard.read(&path)?).map_err(|_| StorageError::Corrupt)?;
    if record.version != 1 || record.digest != plan.digest()? {
        return Err(StorageError::Corrupt.into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &record.files {
        if !seen.insert(&row.path)
            || !SETUP_FILES.contains(&row.path.as_str())
                && !crate::client_setup::stock_case::PATCH_TARGETS.contains(&row.path.as_str())
            || !row.path.eq_ignore_ascii_case(&row.original_path)
        {
            return Err(StorageError::Corrupt.into());
        }
    }
    if !SETUP_FILES
        .iter()
        .all(|key| record.files.iter().any(|row| row.path == *key))
    {
        return Err(StorageError::Corrupt.into());
    }
    Ok(record)
}
// Enumerate exact spelling: a case-insensitive open alone cannot distinguish the
// original name from setup's approved normalization on Windows or macOS.
fn live(plan: &Plan, key: &str) -> Result<Option<(String, Hashed)>, Error> {
    let root = &plan.installation.destination;
    let target = inventory::native(root, key);
    let guard = Parents::new(plan, &[&target], false)?;
    let parent = target.parent().ok_or(StorageError::Corrupt)?;
    if !parent.exists() {
        return Ok(None);
    }
    let name = target.file_name().ok_or(StorageError::Corrupt)?;
    let names = guard.names(parent)?;
    let mut found = names.into_iter().filter(|entry| {
        entry
            .to_string_lossy()
            .eq_ignore_ascii_case(&name.to_string_lossy())
    });
    let Some(found_name) = found.next() else {
        return Ok(None);
    };
    if found.next().is_some() {
        return Err(refused(Error::SourceChanged, "ambiguous setup spelling"));
    }
    let found = parent.join(found_name);
    let relative = found
        .strip_prefix(root)
        .map_err(|_| StorageError::Corrupt)?
        .to_str()
        .ok_or(StorageError::Corrupt)?
        .replace('\\', "/");
    Ok(Some((relative, guard.hash(&found)?)))
}
/// Persist before the first setup write. Originals are after reviewed replacements,
/// so rollback restores setup first and then restores those replacements.
pub(super) fn record(plan: &Plan, ledger: &[u8]) -> Result<(), Error> {
    let file = path(plan);
    let guard = Parents::new(plan, &[&file], true)?;
    if guard.present(&file)? {
        return check(plan);
    }
    // A crash after saving originals but before this record was published must
    // never re-pin a later player edit or overwrite the only saved original.
    if guard.present(&plan.stage().join(super::in_place_check::SETUP_MANIFEST))?
        || guard.is_dir(&plan.stage().join(super::in_place_check::SETUP_ORIGINALS))?
    {
        return Err(refused(
            Error::SourceChanged,
            "setup expectations were not committed; saved originals retained",
        ));
    }
    super::in_place_check::save_setup_originals(plan, &plan.installation.destination)?;
    let mut files = Vec::new();
    for key in SETUP_FILES.into_iter().chain(
        crate::client_setup::stock_case::PATCH_TARGETS
            .iter()
            .copied(),
    ) {
        if files.iter().any(|row: &Expected| row.path == key) {
            continue;
        }
        let current = live(plan, key)?;
        let original = current.as_ref().map(|(_, hash)| *hash);
        let original_path = current
            .as_ref()
            .map(|(name, _)| name.as_str())
            .unwrap_or(key)
            .to_owned();
        let approved = match key {
            "Working/Binaries/SGW.exe" => {
                let exe = inventory::native(&plan.installation.destination, &original_path);
                let source = Parents::new(plan, &[&exe], false)?;
                let mut scratch = tempfile::tempfile()?;
                std::io::copy(&mut source.open(&exe, false, false)?, &mut scratch)?;
                crate::client_setup::aslr::disable_open(&mut scratch)?;
                use std::io::{Seek, SeekFrom};
                scratch.seek(SeekFrom::Start(0))?;
                let mut digest = Sha256::new();
                let mut bytes = [0u8; 65536];
                let mut size = 0;
                loop {
                    let count = scratch.read(&mut bytes)?;
                    if count == 0 {
                        break;
                    }
                    size += count as u64;
                    digest.update(&bytes[..count]);
                }
                Some(Hashed {
                    size,
                    sha256: digest.finalize().into(),
                })
            }
            "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua" => {
                crate::client_setup::login_servers::validate(&plan.installation.login_servers)
                    .map_err(|_| StorageError::Corrupt)?;
                Some(bytes_hash(
                    crate::client_setup::login_servers::render(&plan.installation.login_servers)
                        .as_bytes(),
                ))
            }
            "launcher-installed.json" => Some(bytes_hash(ledger)),
            _ => original,
        };
        files.push(Expected {
            path: key.into(),
            original_path,
            original,
            approved,
            rewrite: SETUP_FILES.contains(&key),
        });
    }
    guard.write(
        &file,
        &serde_json::to_vec(&Expectations {
            version: 1,
            digest: plan.digest()?,
            files,
        })
        .map_err(|_| StorageError::Corrupt)?,
    )
}
fn matches(current: &Option<(String, Hashed)>, path: &str, expected: Option<Hashed>) -> bool {
    match (current, expected) {
        (None, None) => true,
        (Some((name, actual)), Some(expected)) => name == path && *actual == expected,
        _ => false,
    }
}
fn checked(plan: &Plan) -> Result<Expectations, Error> {
    let record = load(plan)?;
    for row in &record.files {
        let current = live(plan, &row.path)?;
        if !matches(&current, &row.original_path, row.original)
            && !matches(&current, &row.path, row.approved)
        {
            return Err(refused(
                Error::SourceChanged,
                "a setup target changed after interruption",
            ));
        }
    }
    Ok(record)
}
pub(super) fn check(plan: &Plan) -> Result<(), Error> {
    checked(plan).map(|_| ())
}
/// Check all targets before changing any, then restore through retained parents.
/// Keep saved originals until the existing durable setup checkpoint is committed.
pub(super) fn restore(plan: &Plan) -> Result<u32, Error> {
    let record = checked(plan)?;
    let root = &plan.installation.destination;
    let mut touched = 0;
    for row in record.files {
        let current = live(plan, &row.path)?;
        if matches(&current, &row.original_path, row.original) {
            continue;
        }
        let target = inventory::native(root, &row.path);
        let original = inventory::native(root, &row.original_path);
        let source = inventory::native(
            &plan.stage().join(super::in_place_check::SETUP_ORIGINALS),
            &row.original_path,
        );
        let guard = Parents::new(plan, &[&target, &original, &source], true)?;
        if row.rewrite {
            if let Some(expected) = row.original {
                if !guard.holds(&source, &expected)? {
                    return Err(StorageError::Corrupt.into());
                }
                guard.copy_atomic(&source, &target)?;
                if row.original_path != row.path {
                    guard.rename(&target, &original, false)?;
                }
            } else {
                guard.remove(&target, false)?;
            }
        } else if row.original.is_some() && row.original_path != row.path {
            guard.rename(&target, &original, false)?;
        }
        touched += 1;
    }
    Ok(touched)
}
