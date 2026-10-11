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

pub mod cpu;
pub mod levels;
pub mod run;

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
    /// The knobs the text set over its level (a flush mode word counts as
    /// `mode`). Kept so [`header`] can print a line that replays the run.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, String>,
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

/// The knobs each flush mode reads, with the default a mode gets when
/// neither its level nor the text sets it. A mode switched away from the
/// level's own (`flush:pack` on the safe level) would otherwise carry the
/// old mode's knobs and none of its own.
fn flush_mode_knobs(mode: &str) -> &'static [(&'static str, &'static str)] {
    match mode {
        "delay" => &[("delay_ms", "2000")],
        "pack" => &[("records", "24")],
        "shift" => &[("shift", "sweep")],
        _ => &[],
    }
}

/// "flush:pack", "net:break:seed=7", "net:latency=120,loss=1", "cpu".
/// No level word and no knobs means safe; knobs without a level word mean
/// [`Level::Explicit`]. Values are kept as typed, units and all; the
/// consumer parses them. Naming anything twice is refused, so a spec has one
/// reading. `default_seed` is used when the text has no seed.
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
        if part.is_empty() {
            return Err(bad_value("spec", text, "empty segment"));
        }
        if let Some(level) = Level::from_word(part) {
            if level_word.replace(level).is_some() {
                return Err(bad_value("level", part, "only one level may be named"));
            }
        } else if profile == Profile::Flush && FLUSH_MODES.contains(&part) {
            if mode_word.replace(part).is_some() {
                return Err(bad_value("mode", part, "only one mode may be named"));
            }
        } else {
            for pair in part.split(',') {
                let (knob, value) = knob_pair(profile, pair)?;
                if knob == "seed" {
                    let parsed = value
                        .parse()
                        .map_err(|_| bad_value("seed", &value, "not an unsigned integer"))?;
                    if seed.replace(parsed).is_some() {
                        return Err(bad_value("seed", &value, "seed is given twice"));
                    }
                } else {
                    match overrides.entry(knob) {
                        std::collections::btree_map::Entry::Occupied(entry) => {
                            return Err(bad_value(entry.key(), &value, "knob is given twice"));
                        }
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            entry.insert(value);
                        }
                    }
                }
            }
        }
    }
    if let Some(mode) = mode_word {
        if overrides.contains_key("mode") {
            return Err(bad_value("mode", mode, "mode is given twice"));
        }
    }

    let level = match level_word {
        Some(level) => level,
        None if mode_word.is_some() || overrides.is_empty() => Level::Safe,
        None => Level::Explicit,
    };
    if let Some(mode) = mode_word {
        overrides.insert("mode".to_string(), mode.to_string());
    }
    let mut knobs = levels.get(profile, &level).knobs.clone();
    knobs.extend(overrides.clone());
    if profile == Profile::Flush {
        fit_knobs_to_mode(&mut knobs, &overrides)?;
    }

    Ok(ChaosSpec {
        profile,
        level,
        knobs,
        overrides,
        seed: seed.unwrap_or(default_seed),
    })
}

/// Keeps only the knobs the resolved flush mode reads, filling its defaults.
/// A knob the text set for another mode is refused rather than dropped.
fn fit_knobs_to_mode(
    knobs: &mut BTreeMap<String, String>,
    overrides: &BTreeMap<String, String>,
) -> Result<(), ChaosParseError> {
    let mode = knobs.get("mode").cloned().unwrap_or_default();
    let wanted = flush_mode_knobs(&mode);
    for (knob, value) in overrides {
        if knob != "mode" && !wanted.iter().any(|(name, _)| name == knob) {
            return Err(bad_value(knob, value, "this flush mode does not read it"));
        }
    }
    knobs.retain(|knob, _| knob == "mode" || wanted.iter().any(|(name, _)| name == knob));
    for (knob, default) in wanted {
        knobs
            .entry((*knob).to_string())
            .or_insert_with(|| (*default).to_string());
    }
    Ok(())
}

/// Splits one `knob=value` pair and checks the knob belongs to the profile.
/// A bare word here is neither a level, a mode nor a knob, so it is refused
/// as a bad value naming all three (a typo like `Break` lands here).
fn knob_pair(profile: Profile, pair: &str) -> Result<(String, String), ChaosParseError> {
    let (knob, value) = pair.split_once('=').ok_or_else(|| {
        bad_value(
            "spec",
            pair,
            "not a level (safe, break), a flush mode or knob=value",
        )
    })?;
    if value.is_empty() {
        return Err(bad_value(knob, value, "empty value"));
    }
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
/// The text after `chaos=` is itself a spec that [`parse`] resolves to the
/// same knobs, so the header alone replays the run. The word after the
/// profile is the level word, except for flush at the safe level, where it
/// is the mode, so `flush:pack` and `flush:break` stay distinct. The text's
/// overrides follow as one `knob=value` list; `Explicit` specs have only that.
pub fn header(spec: &ChaosSpec) -> String {
    let mut out = format!("chaos={}", spec.profile.name());
    let mode_as_word = spec.profile == Profile::Flush && spec.level == Level::Safe;
    let word = if mode_as_word {
        spec.knobs.get("mode").map(String::as_str)
    } else {
        spec.level.word()
    };
    if let Some(word) = word {
        out.push(':');
        out.push_str(word);
    }
    let rest: Vec<String> = spec
        .overrides
        .iter()
        .filter(|(knob, _)| !(mode_as_word && knob.as_str() == "mode"))
        .map(|(knob, value)| format!("{knob}={value}"))
        .collect();
    if !rest.is_empty() {
        out.push(':');
        out.push_str(&rest.join(","));
    }
    out.push_str(&format!(" seed={}", spec.seed));
    out
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
    fn header_replays_the_spec() {
        let levels = levels();
        for text in [
            "net:break:latency=120,seed=3",
            "net:latency=120ms,seed=3",
            "flush:pack:records=8,seed=3",
            "flush:break:mode=delay,seed=3",
            "cpu:seed=3",
        ] {
            let spec = parse(text, &levels, 1).expect("spec parses");
            let line = header(&spec);
            let (body, seed) = line
                .strip_prefix("chaos=")
                .and_then(|rest| rest.split_once(' '))
                .expect("header has the chaos= shape");
            let replay = parse(&format!("{body}:{seed}"), &levels, 1).expect("header text parses");
            assert_eq!(replay.knobs, spec.knobs, "{text} -> {line}");
            assert_eq!(replay.seed, spec.seed, "{text} -> {line}");
        }
        let plain = parse("net:break", &levels, 1).expect("net break parses");
        let tuned = parse("net:break:latency=120", &levels, 1).expect("tuned parses");
        assert_ne!(header(&plain), header(&tuned));
    }

    #[test]
    fn a_mode_word_carries_its_own_knobs() {
        let levels = levels();
        let pack = parse("flush:pack", &levels, 1).expect("flush pack parses");
        assert_eq!(pack.knobs["records"], "24");
        assert!(!pack.knobs.contains_key("delay_ms"));
        let shift = parse("flush:shift", &levels, 1).expect("flush shift parses");
        assert_eq!(shift.knobs["shift"], "sweep");
        let reorder = parse("flush:reorder", &levels, 1).expect("flush reorder parses");
        assert_eq!(reorder.knobs.keys().collect::<Vec<_>>(), ["mode"]);
        assert!(parse("flush:pack:delay_ms=5", &levels, 1).is_err());
    }

    #[test]
    fn anything_named_twice_or_empty_is_refused() {
        let levels = levels();
        for text in [
            "net:break:safe",
            "flush:pack:delay",
            "flush:pack:mode=delay",
            "net:latency=1,latency=2",
            "net:seed=1,seed=2",
            "net:latency=",
            "cpu:",
            "net:Break",
        ] {
            assert!(
                matches!(
                    parse(text, &levels, 1),
                    Err(ChaosParseError::BadValue { .. })
                ),
                "{text} should be a bad value"
            );
        }
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
