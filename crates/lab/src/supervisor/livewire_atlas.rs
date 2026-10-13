//! Private, client-derived Livewire hit atlas loaded by the lab solver.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const ATLAS_VERSION: u8 = 1;
const HIT_STEP: i32 = 5;

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

    /// Default cache written by `lab install`; override for a private client copy.
    pub fn installed_path() -> PathBuf {
        if let Some(path) = std::env::var_os("CIMMERIA_LIVEWIRE_ATLAS") {
            return PathBuf::from(path);
        }
        let base = std::env::var_os("LOCALAPPDATA").map_or_else(PathBuf::new, PathBuf::from);
        base.join("cimmeria-lab").join("livewire_atlas.json")
    }

    pub fn library(&self, name: &str) -> Option<&HitLibrary> {
        self.libraries.get(name)
    }

    fn validate(&self) -> Result<(), String> {
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
        if self.libraries.is_empty() {
            return Err("Livewire atlas has no libraries".into());
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

    #[test]
    fn loader_rejects_off_grid_points_and_bad_bounds() {
        let mut atlas = LivewireAtlas {
            version: 1,
            step: 5,
            movie_sha256: "a".repeat(64),
            libraries: BTreeMap::from([(
                "gGreenBlack1".into(),
                HitLibrary {
                    bounds_twips: [0.0, 100.0, 0.0, 100.0],
                    points: vec![[0, 0]],
                },
            )]),
        };
        assert!(atlas.validate().is_ok());
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
}
