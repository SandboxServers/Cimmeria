//! Which `data/spaces/` file a world's navmesh (`.nav`) and occluder
//! (`.occ`) load from.
//!
//! Files are keyed by world name: lower case, spaces as underscores
//! (`Castle_CellBlock` → `castle_cellblock.nav`). A world with no files of its
//! own reads its **client map's** instead (decision D-DA5,
//! `docs/analysis/debug-area/README.md`): `DebugArea` (1300) plays the shipped
//! `Ihpet_Crater_Light` map, so it reads `ihpet_crater_light.nav` / `.occ`
//! rather than shipping a 5 MB copy of each.
//!
//! **The two kinds resolve as a pair, never mixed.** A world that ships
//! either file of its own owns its data: both kinds load from its own name,
//! and a missing one stays missing. `SandBox` ships `sandbox.nav` and no
//! `.occ`, so it runs its own mesh with no occluder, as before D-DA5, rather
//! than its own mesh with Harset_CmdCenter's occluder. An occluder built from
//! the map's geometry is only right beside a mesh built from the same
//! geometry, and a world gets a file of its own precisely when its data must
//! differ from its map's.
//!
//! The client map comes from
//! [`cimmeria_wire::mercury::world_data::client_map_for_world`], the same
//! table `onClientMapLoad` reads, so the mesh the cell loads is the map the
//! client was told to load.

use std::path::{Path, PathBuf};

use cimmeria_wire::mercury::world_data::{client_map_for_world, known_world_id};

/// The directory both file kinds load from, relative to the server's CWD.
pub(crate) const SPACE_DATA_DIR: &str = "data/spaces";

/// The file-name key of a world or map name: lower case, spaces as
/// underscores.
pub(crate) fn file_key(name: &str) -> String {
    name.to_lowercase().replace(' ', "_")
}

/// Where a resolved file came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpaceFileSource {
    /// The world's own file (`<world>.<ext>`).
    World,
    /// The world had none; this is its client map's (D-DA5).
    ClientMap { client_map: String },
}

impl SpaceFileSource {
    /// The `file_source` log field.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::World => "world",
            Self::ClientMap { .. } => "client_map",
        }
    }
}

/// A file that exists for a world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpaceFile {
    pub path: PathBuf,
    /// File key the file is named by (`ihpet_crater_light` for DebugArea):
    /// what the occluder cache and its residency gauges key on, so two worlds
    /// reading one file share it.
    pub key: String,
    pub source: SpaceFileSource,
}

/// The file kinds that resolve together (see the module docs).
const PAIRED_EXTS: [&str; 2] = ["nav", "occ"];

/// Whether `dir` holds any paired file named `key`.
fn ships_any(dir: &Path, key: &str) -> bool {
    PAIRED_EXTS
        .iter()
        .any(|ext| dir.join(format!("{key}.{ext}")).exists())
}

/// The `<ext>` file for `world_name` in `dir`: the world's own when it ships
/// any file of its own, else its client map's, else `Err(path of the world's
/// own file)` for the miss log.
pub(crate) fn resolve_space_file(
    dir: &Path,
    world_name: &str,
    ext: &str,
) -> Result<SpaceFile, PathBuf> {
    let own_key = file_key(world_name);
    let own = dir.join(format!("{own_key}.{ext}"));
    let client_map = client_map_for_world(world_name);
    let map_key = file_key(client_map);
    if map_key == own_key || ships_any(dir, &own_key) {
        return if own.exists() {
            Ok(SpaceFile {
                path: own,
                key: own_key,
                source: SpaceFileSource::World,
            })
        } else {
            Err(own)
        };
    }
    let path = dir.join(format!("{map_key}.{ext}"));
    if !path.exists() {
        return Err(own);
    }
    tracing::info!(
        target: "movement.navmesh",
        event = "space_file_fallback",
        world = %world_name,
        world_id = known_world_id(world_name),
        client_map,
        ext,
        path = %path.display(),
        missing = %own.display(),
        "space data: the world has no files of its own -- loading its client map's (D-DA5)"
    );
    Ok(SpaceFile {
        path,
        key: map_key,
        source: SpaceFileSource::ClientMap {
            client_map: client_map.to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The repo's own `data/spaces/`, so the guard runs against the files
    /// that ship.
    fn shipped_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces")
    }

    /// D-DA5: DebugArea ships no files of its own and reads the
    /// Ihpet_Crater_Light map's. Fails if the fallback is removed (the world
    /// key alone resolves to `debugarea.nav`, which does not exist).
    #[test]
    fn debug_area_reads_the_ihpet_crater_light_nav_and_occ() {
        let dir = shipped_dir();
        for ext in ["nav", "occ"] {
            assert!(
                !dir.join(format!("debugarea.{ext}")).exists(),
                "DebugArea must not ship its own .{ext}; it reads the client map's"
            );
            let file = resolve_space_file(&dir, "DebugArea", ext)
                .unwrap_or_else(|p| panic!("DebugArea .{ext} unresolved (tried {})", p.display()));
            assert_eq!(file.path, dir.join(format!("ihpet_crater_light.{ext}")));
            assert_eq!(file.key, "ihpet_crater_light");
            assert_eq!(
                file.source,
                SpaceFileSource::ClientMap {
                    client_map: "Ihpet_Crater_Light".into()
                }
            );
        }
    }

    /// A world's own file wins over its client map's, and owning one kind
    /// means owning both: SandBox plays the Harset_CmdCenter map but keeps
    /// `sandbox.nav`, and since it ships no `sandbox.occ` it runs with no
    /// occluder rather than `harset_cmdcenter.occ`. A stock world resolves to
    /// itself.
    #[test]
    fn own_file_wins_over_the_client_map() {
        let dir = shipped_dir();
        let sandbox = resolve_space_file(&dir, "SandBox", "nav").unwrap();
        assert_eq!(sandbox.path, dir.join("sandbox.nav"));
        assert_eq!(sandbox.source, SpaceFileSource::World);
        assert_eq!(sandbox.key, "sandbox");
        assert!(dir.join("harset_cmdcenter.occ").exists());
        assert_eq!(
            resolve_space_file(&dir, "SandBox", "occ"),
            Err(dir.join("sandbox.occ")),
            "SandBox must not pair its own mesh with its map's occluder"
        );

        let light = resolve_space_file(&dir, "Ihpet_Crater_Light", "occ").unwrap();
        assert_eq!(light.source, SpaceFileSource::World);
        assert_eq!(light.key, "ihpet_crater_light");
    }

    /// A scratch `data/spaces` holding exactly `files`, removed on drop.
    struct ScratchDir(PathBuf);

    impl ScratchDir {
        fn new(tag: &str, files: &[&str]) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("cimmeria-space-files-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for f in files {
                std::fs::write(dir.join(f), b"x").unwrap();
            }
            Self(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The pairing rule on its own, with SandBox (client map
    /// Harset_CmdCenter) in a scratch directory: the source is decided once
    /// for both kinds. Fails if the kinds resolve independently (an own
    /// `.nav` would then pick up the map's `.occ`).
    #[test]
    fn nav_and_occ_resolve_as_a_pair() {
        let map = ["harset_cmdcenter.nav", "harset_cmdcenter.occ"];
        let pick = |dir: &Path, ext: &str| match resolve_space_file(dir, "SandBox", ext) {
            Ok(f) => Some(f.key),
            Err(_) => None,
        };

        // No files of its own: both from the client map.
        let d = ScratchDir::new("map-only", &map);
        assert_eq!(pick(&d.0, "nav").as_deref(), Some("harset_cmdcenter"));
        assert_eq!(pick(&d.0, "occ").as_deref(), Some("harset_cmdcenter"));

        // Own .nav only: own mesh, no occluder.
        let d = ScratchDir::new("own-nav", &[map[0], map[1], "sandbox.nav"]);
        assert_eq!(pick(&d.0, "nav").as_deref(), Some("sandbox"));
        assert_eq!(pick(&d.0, "occ"), None);

        // Own .occ only: own occluder, no mesh.
        let d = ScratchDir::new("own-occ", &[map[0], map[1], "sandbox.occ"]);
        assert_eq!(pick(&d.0, "nav"), None);
        assert_eq!(pick(&d.0, "occ").as_deref(), Some("sandbox"));
    }

    /// A world with neither file stays a miss and reports its own path. The
    /// historical CellBlocks' client maps (`C43485_CellBlock`) have no
    /// files, so they keep loading nothing.
    #[test]
    fn a_world_with_neither_file_is_a_miss_naming_its_own_path() {
        let dir = shipped_dir();
        assert_eq!(
            resolve_space_file(&dir, "CellBlock43", "nav"),
            Err(dir.join("cellblock43.nav"))
        );
        assert_eq!(
            resolve_space_file(&dir, "No Such World", "occ"),
            Err(dir.join("no_such_world.occ"))
        );
    }
}
