//! Private, client-derived Livewire hit atlas loaded by the lab solver.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const ATLAS_VERSION: u8 = 1;
const HIT_STEP: i32 = 5;
const WIRE_BASES: &[&str] = &[
    "gBlackGreen",
    "gGreenBlack",
    "gRedSilver",
    "oBlack",
    "oGreen",
    "oRed",
    "mBlackStripe",
    "mOrangeStripe",
    "mPurpleStripe",
    "mYellowStripe",
    "pBronze",
    "pGray",
    "pGreen",
    "pRust",
];
const ELECTRODES: &[&str] = &[
    "mGear",
    "mMicroChip1",
    "mMicroChip2",
    "pLEDBlue",
    "pLEDGreen",
    "pLEDRed",
];

#[derive(Debug, Deserialize)]
pub struct LivewireAtlas {
    pub version: u8,
    pub step: i32,
    pub movie_sha256: String,
    pub libraries: BTreeMap<String, HitLibrary>,
}

#[derive(Debug, Deserialize)]
pub struct HitLibrary {
    /// Shape bounds in SWF twips, ordered x min, x max, y min, y max.
    pub bounds_twips: [f64; 4],
    /// Local stage points on the five-unit hit grid.
    pub points: Vec<[i32; 2]>,
}

impl LivewireAtlas {
    /// Read the atlas generated from the user's installed Livewire.upk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("cannot read Livewire atlas {}: {error}", path.display()))?;
        let atlas: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid Livewire atlas {}: {error}", path.display()))?;
        atlas.validate()?;
        Ok(atlas)
    }

    /// Cache written by `lab install`; honor its custom lab home.
    pub fn installed_path() -> PathBuf {
        Self::path_from(
            std::env::var_os("CIMMERIA_LIVEWIRE_ATLAS"),
            std::env::var_os("CIMMERIA_LAB_HOME"),
            std::env::var_os("LOCALAPPDATA"),
        )
    }

    fn path_from(
        atlas: Option<OsString>,
        home: Option<OsString>,
        local: Option<OsString>,
    ) -> PathBuf {
        if let Some(path) = atlas {
            return PathBuf::from(path);
        }
        if let Some(home) = home {
            return PathBuf::from(home).join("livewire_atlas.json");
        }
        local
            .map_or_else(std::env::temp_dir, PathBuf::from)
            .join("cimmeria-lab")
            .join("livewire_atlas.json")
    }

    pub fn library(&self, name: &str) -> Option<&HitLibrary> {
        self.libraries.get(name)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version != ATLAS_VERSION || self.step != HIT_STEP {
            return Err(format!(
                "unsupported Livewire atlas version/step: {}/{}",
                self.version, self.step
            ));
        }
        if self.movie_sha256.len() != 64
            || !self
                .movie_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("Livewire atlas lacks a valid movie SHA-256".into());
        }
        if self.libraries.len() != 62
            || WIRE_BASES.iter().any(|base| {
                (1..=4).any(|variant| !self.libraries.contains_key(&format!("{base}{variant}")))
            })
            || ELECTRODES
                .iter()
                .any(|name| !self.libraries.contains_key(*name))
        {
            return Err("Livewire atlas does not contain the expected 62 libraries".into());
        }
        for (name, library) in &self.libraries {
            let [x0, x1, y0, y1] = library.bounds_twips;
            if ![x0, x1, y0, y1].iter().all(|value| value.is_finite()) || x0 > x1 || y0 > y1 {
                return Err(format!("Livewire atlas {name} has invalid bounds"));
            }
            if library.points.is_empty() {
                return Err(format!("Livewire atlas {name} has no hit points"));
            }
            for [x, y] in &library.points {
                if x % HIT_STEP != 0 || y % HIT_STEP != 0 {
                    return Err(format!(
                        "Livewire atlas {name} has a point off the hit grid"
                    ));
                }
                // Shape bounds may exclude the stroke width. Allow the half-stroke
                // margin while catching a corrupt stage coordinate.
                let margin = 40.0;
                if f64::from(*x) * 20.0 < x0 - margin
                    || f64::from(*x) * 20.0 > x1 + margin
                    || f64::from(*y) * 20.0 < y0 - margin
                    || f64::from(*y) * 20.0 > y1 + margin
                {
                    return Err(format!(
                        "Livewire atlas {name} has a point outside its bounds"
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_atlas() -> LivewireAtlas {
        let mut libraries = BTreeMap::new();
        for base in WIRE_BASES {
            for variant in 1..=4 {
                libraries.insert(
                    format!("{base}{variant}"),
                    HitLibrary {
                        bounds_twips: [0.0, 100.0, 0.0, 100.0],
                        points: vec![[0, 0]],
                    },
                );
            }
        }
        for name in ELECTRODES {
            libraries.insert(
                (*name).into(),
                HitLibrary {
                    bounds_twips: [0.0, 100.0, 0.0, 100.0],
                    points: vec![[0, 0]],
                },
            );
        }
        LivewireAtlas {
            version: 1,
            step: 5,
            movie_sha256: "a".repeat(64),
            libraries,
        }
    }

    #[test]
    fn loader_rejects_off_grid_points_and_bad_bounds() {
        let mut atlas = complete_atlas();
        assert!(atlas.validate().is_ok());
        let removed = atlas.libraries.remove("gGreenBlack2").unwrap();
        assert!(atlas
            .validate()
            .unwrap_err()
            .contains("expected 62 libraries"));
        atlas.libraries.insert(
            "unrelated".into(),
            HitLibrary {
                bounds_twips: [0.0, 100.0, 0.0, 100.0],
                points: vec![[0, 0]],
            },
        );
        assert!(atlas
            .validate()
            .unwrap_err()
            .contains("expected 62 libraries"));
        atlas.libraries.remove("unrelated");
        atlas.libraries.insert("gGreenBlack2".into(), removed);
        atlas.libraries.get_mut("gGreenBlack1").unwrap().points[0] = [3, 0];
        assert!(atlas.validate().unwrap_err().contains("off the hit grid"));
        atlas.libraries.get_mut("gGreenBlack1").unwrap().points[0] = [0, 0];
        atlas
            .libraries
            .get_mut("gGreenBlack1")
            .unwrap()
            .bounds_twips = [100.0, 0.0, 0.0, 100.0];
        assert!(atlas.validate().unwrap_err().contains("invalid bounds"));
    }

    #[test]
    fn loader_accepts_locally_generated_atlas() {
        let Ok(path) = std::env::var("CIMMERIA_LIVEWIRE_ATLAS") else {
            return;
        };
        let atlas = LivewireAtlas::load(Path::new(&path)).unwrap();
        assert!(atlas.library("gGreenBlack1").is_some());
        assert!(atlas.library("pGray4").is_some());
    }

    #[test]
    fn atlas_path_follows_custom_lab_home() {
        let custom = PathBuf::from(r"C:\custom-lab");
        assert_eq!(
            LivewireAtlas::path_from(
                None,
                Some(custom.clone().into_os_string()),
                Some(OsString::from(r"C:\default"))
            ),
            custom.join("livewire_atlas.json")
        );
        assert_eq!(
            LivewireAtlas::path_from(
                Some(OsString::from(r"C:\override\atlas.json")),
                Some(custom.into_os_string()),
                None
            ),
            PathBuf::from(r"C:\override\atlas.json")
        );
    }
}
