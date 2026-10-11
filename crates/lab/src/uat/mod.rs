//! Automated in-game UAT: row specs as data, a runner that drives the
//! client through the lab tools by name, an evidence bundle per run, and
//! ledger output in the unified-uat.md "Recording results" format.
//!
//! - [`spec`] — the TOML row-spec schema (`docs/guides/uat-specs/*.toml`);
//!   [`spec_validate`] — the rules the types cannot express.
//! - [`tier`] — native levels (N1/N2/N3/G/X) and what each tool may claim.
//! - [`tools`] — the capability table (`@world_click` → tool name), the one
//!   place a tool rename is made; [`lab_commands`] — the ability dot
//!   commands (`@dummy`, `@cooldowns_reset`, `@clear_effects`) it names.
//! - [`runner`] — executes rows; [`invoke`] is its by-name action layer.
//! - [`seed_tags`] — the `spawnlist` tags the seed defines; every tag a spec
//!   names must be one of them.
//! - [`clause`] / [`grade`] — pure evaluation and grading rules.
//! - [`evidence`] — the bundle layout; [`ledger`] — the paste-ready text.
//! - [`golden`] — run fingerprints, golden agreement and diffs.
//! - [`attest`] — SigNoz rows, human answers and the build, after the run.
//!
//! How to run it: `docs/guides/automated-uat.md`.

pub mod attest;
pub mod clause;
pub mod evidence;
pub mod golden;
pub mod grade;
pub mod invoke;
pub mod lab_commands;
pub mod ledger;
pub mod runner;
pub mod seed_tags;
pub mod spec;
pub mod spec_fixture;
pub mod spec_validate;
pub mod tier;
pub mod tools;

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use runner::LoadedSpec;

/// Where the spec files live: `CIMMERIA_LAB_UAT_SPECS`, else the first
/// `docs/guides/uat-specs` found walking up from the working directory
/// and from this executable (a `target/<profile>/` build of the repo).
pub fn specs_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("CIMMERIA_LAB_UAT_SPECS") {
        if !d.is_empty() {
            return Some(PathBuf::from(d));
        }
    }
    let starts = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
    ];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            let cand = dir.join("docs").join("guides").join("uat-specs");
            if cand.is_dir() {
                return Some(cand);
            }
        }
    }
    None
}

/// Load every `*.toml` in `dir` (sorted), keeping the sections whose id is
/// in `only` (all when `None`). A file that fails to parse is an error:
/// running half a spec set silently would hide rows. When the seed folder
/// sits beside `dir`, every tag a spec names must be in it (SV-04).
pub fn load_sections(dir: &Path, only: Option<&[String]>) -> Result<Vec<LoadedSpec>, String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("read {}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();
    let seed = seed_tags::SeedTags::beside_specs(dir)
        .map(|d| seed_tags::SeedTags::load(&d))
        .transpose()?;
    let mut out = Vec::new();
    for f in files {
        let text = std::fs::read_to_string(&f).map_err(|e| format!("read {}: {e}", f.display()))?;
        let spec = spec::parse(&text).map_err(|e| format!("{}: {e}", f.display()))?;
        if let Some(seed) = &seed {
            seed_tags::check(&spec, seed).map_err(|e| format!("{}: {e}", f.display()))?;
        }
        if only.is_some_and(|o| !o.iter().any(|s| s == &spec.section.id)) {
            continue;
        }
        let sha: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        out.push(LoadedSpec {
            path: f.display().to_string(),
            sha256: sha,
            spec,
        });
    }
    if let Some(o) = only {
        for want in o {
            if !out.iter().any(|s| &s.spec.section.id == want) {
                return Err(format!("no spec section {want:?} in {}", dir.display()));
            }
        }
    }
    Ok(out)
}

/// The newest run directory under `root` (by name: they start with the
/// date and sort by it).
pub fn latest_run(root: &Path) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("run.json").is_file())
        .collect();
    dirs.sort_by_key(|p| {
        std::fs::metadata(p.join("run.json"))
            .and_then(|m| m.modified())
            .ok()
    });
    dirs.pop()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every committed spec parses and validates, and row ids are unique
    /// per section. This is the guard that keeps the spec files honest
    /// when someone edits them by hand.
    #[test]
    fn committed_specs_parse_and_validate() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guides/uat-specs");
        let specs = load_sections(&dir, None).unwrap();
        assert!(!specs.is_empty(), "no specs under {}", dir.display());
        let mut ids = std::collections::HashSet::new();
        for s in &specs {
            assert!(
                ids.insert(s.spec.section.id.clone()),
                "duplicate section {}",
                s.spec.section.id
            );
        }
    }

    /// A spec whose pose names a seeded tag loads; one naming a tag the seed
    /// lacks fails the load. Fails if `load_sections` skips the seed check.
    #[test]
    fn every_committed_spec_tag_is_seeded() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        load_sections(&root.join("docs/guides/uat-specs"), None).unwrap();

        // The same layout as the repo: specs under docs/guides beside the seed.
        let tmp = tempfile::tempdir().unwrap();
        let specs = tmp.path().join("docs/guides/uat-specs");
        std::fs::create_dir_all(&specs).unwrap();
        let seed = tmp.path().join("db/resources/Worlds/Seed");
        std::fs::create_dir_all(&seed).unwrap();
        for e in std::fs::read_dir(root.join("db/resources/Worlds/Seed"))
            .unwrap()
            .flatten()
        {
            std::fs::copy(e.path(), seed.join(e.file_name())).unwrap();
        }
        let check = |tag: &str| {
            let text = format!(
                "schema = 1\n[section]\nid = \"seed-check\"\nsystem = \"s\"\nguide = \"g\"\nledger = \"l\"\n\
                 [[row]]\nid = \"P\"\ntitle = \"t\"\nexpected = \"e\"\n\
                 pose = [{{ id = \"p\", tag = \"{tag}\", radius_m = 4.0, bearing_deg = 10.0, dy_m = 0.1 }}]\n\
                 step = [{{ chat = \".help\", label = \"help\" }}]\n\
                 [[row.expect]]\nid = \"help\"\ntext = \"t\"\nsource = \"chat\"\nsince = \"help\"\ncontains = \"help\"\n"
            );
            std::fs::write(specs.join("seed-check.toml"), text).unwrap();
            load_sections(&specs, None)
        };

        // The positive case: the real seeded tag loads.
        let loaded = check("SGCW1_GenHammond").unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].spec.rows[0].pose[0].tag, "SGCW1_GenHammond");

        let err = check("SGC_W1_GenHammond").unwrap_err();
        assert!(
            err.contains("tag SGC_W1_GenHammond is not in the seed"),
            "{err}"
        );
        assert!(!err.contains("did you mean"), "{err}");

        let err = check("armyourself_frostbody").unwrap_err();
        assert!(err.contains("did you mean ArmYourself_FrostBody?"), "{err}");
    }
}
