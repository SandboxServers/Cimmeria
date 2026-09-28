//! The `cimmeria-patch.json` recipe carried inside a patch zip.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{PatchsetError, Result};

/// Zip entry name of the recipe. A patch zip without it is a plain
/// overlay zip.
pub const RECIPE_NAME: &str = "cimmeria-patch.json";

/// Zip directory holding the deltas.
pub const DELTA_DIR: &str = "deltas/";

pub const RECIPE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub schema: u32,
    pub ops: Vec<Op>,
}

/// Rebuild `target` from `sources` and a delta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Op {
    /// Install-relative path, `/`-separated.
    pub target: String,
    /// Concatenated in order to form the delta's source image.
    pub sources: Vec<Source>,
    /// Zip entry holding the bsdiff delta.
    pub delta: String,
    /// SHA-256 of the rebuilt file.
    pub result_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// Install-relative path, `/`-separated.
    pub path: String,
    /// SHA-256 of the stock file as it sits on disk, before any transform.
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Transform::is_none")]
    pub transform: Transform,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Transform {
    /// The file's bytes as they are.
    #[default]
    None,
    /// A UE3 package decompressed and rewritten by `cimmeria-upk`'s
    /// patcher with no changes; see the crate docs.
    UpkNormalize,
}

impl Transform {
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }
}

impl Recipe {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let recipe: Recipe = serde_json::from_slice(bytes)?;
        if recipe.schema != RECIPE_SCHEMA {
            return Err(PatchsetError::UnsupportedSchema(recipe.schema));
        }
        for op in &recipe.ops {
            safe_relative(&op.target)?;
            for s in &op.sources {
                safe_relative(&s.path)?;
            }
        }
        Ok(recipe)
    }
}

/// Turn a recipe path into a relative path that stays inside the install
/// directory. Accepts `/` and `\`; rejects `..`, drive letters and empty
/// names.
pub fn safe_relative(path: &str) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => continue,
            ".." => return Err(PatchsetError::UnsafePath(path.to_string())),
            p if p.contains(':') => return Err(PatchsetError::UnsafePath(path.to_string())),
            p => out.push(p),
        }
    }
    if out.as_os_str().is_empty() {
        return Err(PatchsetError::UnsafePath(path.to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_round_trips_and_omits_the_default_transform() {
        let r = Recipe {
            schema: 1,
            ops: vec![Op {
                target: "Working/a.umap".into(),
                sources: vec![
                    Source {
                        path: "Working/a.umap".into(),
                        sha256: "aa".into(),
                        transform: Transform::UpkNormalize,
                    },
                    Source {
                        path: "Working/b.txt".into(),
                        sha256: "bb".into(),
                        transform: Transform::None,
                    },
                ],
                delta: "deltas/0.bsdiff".into(),
                result_sha256: "cc".into(),
            }],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"upk_normalize\""));
        assert_eq!(json.matches("transform").count(), 1);
        assert_eq!(Recipe::parse(json.as_bytes()).unwrap(), r);
    }

    #[test]
    fn rejects_other_schemas_and_escaping_paths() {
        assert!(matches!(
            Recipe::parse(br#"{"schema":2,"ops":[]}"#),
            Err(PatchsetError::UnsupportedSchema(2))
        ));
        let evil = br#"{"schema":1,"ops":[{"target":"../x","sources":[],"delta":"d","result_sha256":"r"}]}"#;
        assert!(matches!(
            Recipe::parse(evil),
            Err(PatchsetError::UnsafePath(_))
        ));
        for bad in ["..\\x", "C:\\Windows\\x", "", "/"] {
            assert!(safe_relative(bad).is_err(), "{bad:?}");
        }
    }
}
