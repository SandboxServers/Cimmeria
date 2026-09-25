//! Argument parsing for `occluder_extract`: `--flag value` pairs only.
//! An unknown flag, a repeat, a missing value or a bare positional is an
//! error, because every mode's output is a measurement or a shipped file.

use std::collections::HashMap;
use std::path::PathBuf;

use cimmeria_occluder::BuildParams;

/// Parsed `--flag value` pairs for one mode.
pub(crate) struct Flags {
    mode: &'static str,
    map: HashMap<String, String>,
}

impl Flags {
    /// Parse `args` against the allowed flag names (without `--`).
    pub fn parse(mode: &'static str, args: &[String], allowed: &[&str]) -> Result<Self, String> {
        let mut map = HashMap::new();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            let Some(key) = a.strip_prefix("--") else {
                return Err(format!("{mode}: unexpected argument {a:?}"));
            };
            if !allowed.contains(&key) {
                return Err(format!("{mode}: unknown flag --{key}"));
            }
            let Some(v) = it.next() else {
                return Err(format!("{mode}: --{key} needs a value"));
            };
            if map.insert(key.to_string(), v.clone()).is_some() {
                return Err(format!("{mode}: --{key} given twice"));
            }
        }
        Ok(Self { mode, map })
    }

    pub fn opt(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    pub fn req(&self, key: &str) -> Result<&str, String> {
        self.opt(key)
            .ok_or_else(|| format!("{}: --{key} is required", self.mode))
    }

    pub fn path(&self, key: &str) -> Result<PathBuf, String> {
        self.req(key).map(PathBuf::from)
    }

    pub fn f32_or(&self, key: &str, default: f32) -> Result<f32, String> {
        match self.opt(key) {
            None => Ok(default),
            Some(v) => v
                .parse()
                .map_err(|_| format!("{}: --{key} {v:?} is not a number", self.mode)),
        }
    }

    pub fn usize_or(&self, key: &str, default: usize) -> Result<usize, String> {
        match self.opt(key) {
            None => Ok(default),
            Some(v) => v
                .parse()
                .map_err(|_| format!("{}: --{key} {v:?} is not a count", self.mode)),
        }
    }

    /// A number, or `none` for `None`.
    pub fn opt_f32_or(&self, key: &str, default: Option<f32>) -> Result<Option<f32>, String> {
        match self.opt(key) {
            None => Ok(default),
            Some("none") => Ok(None),
            Some(v) => v
                .parse()
                .map(Some)
                .map_err(|_| format!("{}: --{key} {v:?} is not a number or `none`", self.mode)),
        }
    }

    /// A comma-separated list of numbers.
    pub fn f32_list_or(&self, key: &str, default: &[f32]) -> Result<Vec<f32>, String> {
        match self.opt(key) {
            None => Ok(default.to_vec()),
            Some(v) => v
                .split(',')
                .map(|s| {
                    s.trim()
                        .parse()
                        .map_err(|_| format!("{}: --{key} item {s:?} is not a number", self.mode))
                })
                .collect(),
        }
    }

    /// `x,y,z`.
    pub fn point(&self, key: &str) -> Result<[f32; 3], String> {
        let v = self.f32_list_or(key, &[])?;
        match v.as_slice() {
            [x, y, z] => Ok([*x, *y, *z]),
            _ => Err(format!("{}: --{key} needs x,y,z", self.mode)),
        }
    }

    /// The build knobs shared by `build` and `measure`; `cell` is the
    /// geometry cell unless the caller overrides it.
    pub fn build_params(&self) -> Result<BuildParams, String> {
        let d = BuildParams::default();
        Ok(BuildParams {
            cell: self.f32_or("cell", d.cell)?,
            terrain_pitch: self.opt_f32_or("terrain-pitch", d.terrain_pitch)?,
            y_step: self.f32_or("y-step", d.y_step)?,
            merge_gap: self.f32_or("merge-gap", d.merge_gap)?,
            margin: self.opt_f32_or("margin", d.margin)?,
            max_floor_slope_deg: d.max_floor_slope_deg,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn flags_reject_typos_repeats_and_positionals() {
        let ok = Flags::parse("m", &s(&["--cell", "0.25"]), &["cell"]).unwrap();
        assert_eq!(ok.f32_or("cell", 1.0).unwrap(), 0.25);
        assert!(Flags::parse("m", &s(&["--cel", "1"]), &["cell"]).is_err());
        assert!(Flags::parse("m", &s(&["--cell", "1", "--cell", "2"]), &["cell"]).is_err());
        assert!(Flags::parse("m", &s(&["--cell"]), &["cell"]).is_err());
        assert!(Flags::parse("m", &s(&["0.5"]), &["cell"]).is_err());
    }

    #[test]
    fn none_disables_an_optional_knob() {
        let f = Flags::parse("m", &s(&["--terrain-pitch", "none"]), &["terrain-pitch"]).unwrap();
        assert_eq!(f.build_params().unwrap().terrain_pitch, None);
        let f = Flags::parse("m", &s(&[]), &["terrain-pitch"]).unwrap();
        assert_eq!(f.build_params().unwrap().terrain_pitch, Some(1.0));
    }
}
