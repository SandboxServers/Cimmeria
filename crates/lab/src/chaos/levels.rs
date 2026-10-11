//! The committed chaos levels: one named knob set per profile and level,
//! each with the source it was taken from. `levels.toml` is embedded so the
//! CLI, the runner and the docs all read one file.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Level, Profile};

/// The levels file, compiled into the binary.
pub const EMBEDDED: &str = include_str!("levels.toml");

/// Every profile's safe and break levels. A missing level fails to load,
/// so a lookup cannot miss.
#[derive(Debug, Deserialize)]
pub struct Levels {
    cpu: LevelPair,
    net: LevelPair,
    flush: LevelPair,
    burst: LevelPair,
}

#[derive(Debug, Deserialize)]
struct LevelPair {
    safe: LevelEntry,
    #[serde(rename = "break")]
    brk: LevelEntry,
}

/// One level: its knob values and where they came from.
#[derive(Debug, Deserialize)]
pub struct LevelEntry {
    /// Required so no level ships without a bound behind it.
    pub source: String,
    #[serde(flatten)]
    pub knobs: BTreeMap<String, String>,
}

impl Levels {
    /// Parses a levels file. Fails when a profile lacks a level, or a level
    /// lacks its `source`.
    pub fn load(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// The levels compiled into the binary.
    pub fn embedded() -> Result<Self, toml::de::Error> {
        Self::load(EMBEDDED)
    }

    /// The entry for a profile's level. `Explicit` has no entry of its own:
    /// an explicit spec starts from the safe level and overrides knobs.
    pub fn get(&self, profile: Profile, level: &Level) -> &LevelEntry {
        let pair = match profile {
            Profile::Cpu => &self.cpu,
            Profile::Net => &self.net,
            Profile::Flush => &self.flush,
            Profile::Burst => &self.burst,
        };
        match level {
            Level::Break => &pair.brk,
            Level::Safe | Level::Explicit => &pair.safe,
        }
    }
}
