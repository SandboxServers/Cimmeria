//! Chaos profiles for lab runs: the `profile[:level][:k=v,...]` spec that
//! `lab uat --chaos` and the lab-mcp tools accept, resolved against the
//! committed levels file in [`levels`].
//!
//! Grammar: the first segment is the profile. Each later segment is a level
//! word (`safe` or `break`), a flush mode word (`delay`, `reorder`, `pack`,
//! `shift`; flush only), or a comma list of `knob=value` pairs. `seed=<n>`
//! is consumed into [`ChaosSpec::seed`], never stored as a knob.
//!
//! Level rule: a level word names the level. A flush mode word in the level
//! position means the safe level with `mode` overridden. With neither, any
//! knob override makes the spec [`Level::Explicit`]; otherwise it is safe.

pub mod levels;

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use levels::Levels;

/// The four chaos profiles, one per layer a run can stress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    Cpu,
    Net,
    Flush,
    Burst,
}

impl Profile {
    /// The name used in specs, the levels file and the header.
    pub fn name(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Net => "net",
            Self::Flush => "flush",
            Self::Burst => "burst",
        }
    }

    /// The knobs this profile accepts. `seed` is not one: [`parse`] consumes it.
    pub fn knobs(self) -> &'static [&'static str] {
        match self {
            Self::Cpu => &["cores", "duty"],
            Self::Net => &["latency", "jitter", "loss", "dup", "reorder"],
            Self::Flush => &["mode", "delay_ms", "records", "shift"],
            Self::Burst => &["count", "template"],
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::Cpu, Self::Net, Self::Flush, Self::Burst]
            .into_iter()
            .find(|profile| profile.name() == name)
    }
}

/// Which knob set a spec started from. `Explicit` means knobs were set
/// without naming a level; it starts from the safe values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Safe,
    Break,
    Explicit,
}

impl Level {
    fn from_word(word: &str) -> Option<Self> {
        match word {
            "safe" => Some(Self::Safe),
            "break" => Some(Self::Break),
            _ => None,
        }
    }

    /// The level word, or `None` for `Explicit`, which has none.
    fn word(&self) -> Option<&'static str> {
        match self {
            Self::Safe => Some("safe"),
            Self::Break => Some("break"),
            Self::Explicit => None,
        }
    }
}

/// A resolved chaos spec: what a run applies to the lab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChaosSpec {
    pub profile: Profile,
    pub level: Level,
    /// Resolved knobs: the level's values with any explicit overrides applied.
    pub knobs: BTreeMap<String, String>,
    pub seed: u64,
}

/// Why a chaos spec was refused. Each variant names what the user typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChaosParseError {
    UnknownProfile(String),
    UnknownKnob {
        profile: Profile,
        knob: String,
    },
    BadValue {
        knob: String,
        value: String,
        reason: &'static str,
    },
}

impl fmt::Display for ChaosParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProfile(name) => {
                write!(f, "unknown chaos profile {name:?} (cpu, net, flush, burst)")
            }
            Self::UnknownKnob { profile, knob } => {
                write!(f, "unknown knob {knob:?} for {profile:?}")
            }
            Self::BadValue {
                knob,
                value,
                reason,
            } => {
                write!(f, "bad value {value:?} for {knob}: {reason}")
            }
        }
    }
}

impl std::error::Error for ChaosParseError {}

/// The flush modes: the values a flush profile's `mode` knob may take.
const FLUSH_MODES: [&str; 4] = ["delay", "reorder", "pack", "shift"];

/// "flush:pack", "net:break:seed=7", "net:latency=120ms,loss=1%", "cpu".
/// No level means safe. `default_seed` is used when the text has no seed.
pub fn parse(text: &str, levels: &Levels, default_seed: u64) -> Result<ChaosSpec, ChaosParseError> {
    let mut parts = text.split(':');
    let head = parts.next().unwrap_or_default();
    let profile = Profile::from_name(head)
        .ok_or_else(|| ChaosParseError::UnknownProfile(head.to_string()))?;

    let mut level_word = None;
    let mut mode_word = None;
    let mut overrides = BTreeMap::new();
    let mut seed = None;
    for part in parts {
        if let Some(level) = Level::from_word(part) {
            if level_word.replace(level).is_some() {
                return Err(bad_value("level", part, "only one level may be named"));
            }
        } else if profile == Profile::Flush && FLUSH_MODES.contains(&part) {
            mode_word = Some(part);
        } else {
            for pair in part.split(',') {
                let (knob, value) = knob_pair(profile, pair)?;
                if knob == "seed" {
                    let parsed = value
                        .parse()
                        .map_err(|_| bad_value("seed", &value, "not an unsigned integer"))?;
                    seed = Some(parsed);
                } else {
                    overrides.insert(knob, value);
                }
            }
        }
    }

    let level = match level_word {
        Some(level) => level,
        None if mode_word.is_some() || overrides.is_empty() => Level::Safe,
        None => Level::Explicit,
    };
    let mut knobs = levels.get(profile, &level).knobs.clone();
    if let Some(mode) = mode_word {
        knobs.insert("mode".to_string(), mode.to_string());
    }
    knobs.extend(overrides);

    Ok(ChaosSpec {
        profile,
        level,
        knobs,
        seed: seed.unwrap_or(default_seed),
    })
}

/// Splits one `knob=value` pair and checks the knob belongs to the profile.
/// A bare word is an unknown knob: `knob=value` is the only shape a knob takes.
fn knob_pair(profile: Profile, pair: &str) -> Result<(String, String), ChaosParseError> {
    let (knob, value) = pair
        .split_once('=')
        .ok_or_else(|| ChaosParseError::UnknownKnob {
            profile,
            knob: pair.to_string(),
        })?;
    if knob != "seed" && !profile.knobs().contains(&knob) {
        return Err(ChaosParseError::UnknownKnob {
            profile,
            knob: knob.to_string(),
        });
    }
    if knob == "mode" && !FLUSH_MODES.contains(&value) {
        return Err(bad_value(
            knob,
            value,
            "not a flush mode (delay, reorder, pack, shift)",
        ));
    }
    Ok((knob.to_string(), value.to_string()))
}

fn bad_value(knob: &str, value: &str, reason: &'static str) -> ChaosParseError {
    ChaosParseError::BadValue {
        knob: knob.to_string(),
        value: value.to_string(),
        reason,
    }
}

/// The one-line header every chaos report starts with: "chaos=flush:pack seed=7".
///
/// The word after the profile is the level word, except for flush at the safe
/// level, where it is the mode, so `flush:pack` and `flush:break` stay
/// distinct. `Explicit` specs print the bare profile; their knobs are in the
/// report, not the header.
pub fn header(spec: &ChaosSpec) -> String {
    let word = match (spec.profile, &spec.level) {
        (Profile::Flush, Level::Safe) => spec.knobs.get("mode").map(String::as_str),
        (_, level) => level.word(),
    };
    match word {
        Some(word) => format!("chaos={}:{} seed={}", spec.profile.name(), word, spec.seed),
        None => format!("chaos={} seed={}", spec.profile.name(), spec.seed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels() -> Levels {
        Levels::embedded().expect("embedded levels.toml parses")
    }

    #[test]
    fn bare_profile_means_safe_level() {
        let levels = levels();
        let spec = parse("cpu", &levels, 1).expect("bare cpu parses");
        assert_eq!(spec.level, Level::Safe);
        assert_eq!(spec.knobs, levels.get(Profile::Cpu, &Level::Safe).knobs);
    }

    #[test]
    fn explicit_knob_overrides_the_level() {
        let levels = levels();
        let spec = parse("net:break:latency=120", &levels, 1).expect("net break parses");
        assert_eq!(spec.level, Level::Break);
        assert_eq!(spec.knobs["latency"], "120");
        assert_eq!(
            spec.knobs["loss"],
            levels.get(Profile::Net, &Level::Break).knobs["loss"]
        );
    }

    #[test]
    fn unknown_profile_and_knob_are_refused() {
        let levels = levels();
        assert_eq!(
            parse("disk", &levels, 1),
            Err(ChaosParseError::UnknownProfile("disk".to_string()))
        );
        assert_eq!(
            parse("cpu:latency=5", &levels, 1),
            Err(ChaosParseError::UnknownKnob {
                profile: Profile::Cpu,
                knob: "latency".to_string(),
            })
        );
        assert_eq!(
            parse("flush:mode=warp", &levels, 1),
            Err(ChaosParseError::BadValue {
                knob: "mode".to_string(),
                value: "warp".to_string(),
                reason: "not a flush mode (delay, reorder, pack, shift)",
            })
        );
    }

    #[test]
    fn seed_defaults_and_overrides() {
        let levels = levels();
        assert_eq!(parse("net", &levels, 42).map(|s| s.seed), Ok(42));
        assert_eq!(parse("net:seed=7", &levels, 42).map(|s| s.seed), Ok(7));
        assert!(!parse("net:seed=7", &levels, 42)
            .expect("seeded net parses")
            .knobs
            .contains_key("seed"));
    }

    #[test]
    fn header_is_one_line() {
        let levels = levels();
        let spec = parse("flush:pack:seed=7", &levels, 1).expect("flush pack parses");
        assert_eq!(header(&spec), "chaos=flush:pack seed=7");
    }

    #[test]
    fn levels_file_parses_and_every_level_has_a_source() {
        let levels = levels();
        for profile in [Profile::Cpu, Profile::Net, Profile::Flush, Profile::Burst] {
            for level in [Level::Safe, Level::Break] {
                let entry = levels.get(profile, &level);
                assert!(
                    !entry.source.is_empty(),
                    "{profile:?} {level:?} has no source"
                );
                assert!(
                    !entry.knobs.is_empty(),
                    "{profile:?} {level:?} has no knobs"
                );
            }
        }
    }
}
