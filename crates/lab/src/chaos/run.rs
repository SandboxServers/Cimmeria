//! A run's chaos: the `chaos` texts of one `lab_uat_run`, parsed, checked
//! and turned into what each profile needs. `cpu` becomes a [`CpuStress`]
//! for the whole run; `net` and `flush` become one `server_chaos_set` call
//! per row (the lab-mcp tool takes both profiles in one call, with one seed).
//! `burst` is refused until CH-13 (D-CH9) makes `.spawnrandom` report the
//! ids it spawned and `.despawn` take an id (CH-08 worknote, "Burst: deferred").
//!
//! Every knob the server would refuse is refused here, when the run is
//! asked for, so a plan call fails before anything is driven.

use serde_json::{json, Map, Value};

use super::cpu::CpuStress;
use super::levels::Levels;
use super::{header, parse, ChaosSpec, Profile};

/// Why a `burst` spec is refused.
pub const BURST_DEFERRED: &str =
    "burst is not wired yet; it waits on CH-13 (.spawnrandom ids, .despawn <id>)";

/// The longest label `server_chaos_set` takes (lab-mcp `LABEL_MAX`).
const LABEL_MAX: usize = 120;

// The server registry's knob bounds (base-session `lab_chaos/bounds.rs`),
// mirrored so a plan refuses what the set would refuse mid-run. This crate
// does not link the server, so a bound changed there must change here too.
const LATENCY_MAX_MS: u32 = 5000;
const JITTER_MAX_MS: u32 = 5000;
/// Loss and dup are percent with one decimal; the registry takes tenths up
/// to 1000 per thousand.
const PERCENT_MAX: f64 = 100.0;
const REORDER_MAX: u32 = 1000;
const DELAY_MAX_MS: u32 = 60_000;
/// `PACK_RECORDS_MAX`: one phase-1 bundle in a single packet.
const PACK_RECORDS_MAX: u32 = 34;
/// `SHIFT_LEN_MAX`; a shift of 1 byte has no filler shape, so it is 0 or 2..=38.
const SHIFT_LEN_MAX: u32 = 38;

/// What the plan header prints for a seed the run will pick (its run id's
/// hash): the plan's own run id is not the run's.
pub const PER_RUN_SEED: &str = "<per run>";

/// The run's chaos specs, in the order given. Empty means no chaos.
#[derive(Debug, Clone, Default)]
pub struct RunChaos {
    /// One resolved spec per chaos text, in the order given; at most one per
    /// profile, never `burst`.
    pub specs: Vec<ChaosSpec>,
    /// Per spec: whether its text named its seed (`seed=7`). A spec that did
    /// not takes the run's seed, which a plan does not know yet.
    named_seed: Vec<bool>,
}

/// The default seed for a run: a stable hash of its run id (FNV-1a), so a
/// run's seed is in its folder name and every row of the run shares it.
pub fn seed_of(run_id: &str) -> u64 {
    run_id.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl RunChaos {
    /// Parses every text with `default_seed`. Refuses an unknown profile or
    /// knob, `burst`, a profile named twice, a control character, a knob
    /// value the consumer cannot use or the server's bounds refuse, and `net`
    /// with a seeded `flush` mode under two different seeds (the server keeps
    /// one seed per session).
    pub fn parse(texts: &[String], default_seed: u64) -> Result<Self, String> {
        let levels = Levels::embedded().map_err(|e| format!("chaos levels file: {e}"))?;
        let mut chaos = Self::default();
        for text in texts.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            // The text becomes the header and the server's `label`, which
            // the set refuses with a control character in it.
            if text.chars().any(char::is_control) {
                return Err(format!(
                    "chaos {text:?}: a control character is not allowed"
                ));
            }
            let spec =
                parse(text, &levels, default_seed).map_err(|e| format!("chaos {text:?}: {e}"))?;
            if spec.profile == Profile::Burst {
                return Err(format!("chaos {text:?}: {BURST_DEFERRED}"));
            }
            if chaos.specs.iter().any(|s| s.profile == spec.profile) {
                return Err(format!(
                    "chaos {text:?}: profile {} is named twice",
                    spec.profile.name()
                ));
            }
            // A named seed survives a different default; the run's does not.
            let other = parse(text, &levels, default_seed ^ 1).map(|s| s.seed);
            chaos.named_seed.push(other == Ok(spec.seed));
            chaos.specs.push(spec);
        }
        if let (Some(net), Some(flush)) = (chaos.get(Profile::Net), chaos.get(Profile::Flush)) {
            // Only a flush mode that reads the seed shares the net's.
            let (_, flush_reads_seed) = flush_args(flush)?;
            if flush_reads_seed && net.seed != flush.seed {
                return Err(format!(
                    "chaos: net (seed={}) and flush (seed={}) share one server seed; give both the same seed",
                    net.seed, flush.seed
                ));
            }
        }
        chaos.cpu_stress()?;
        chaos.server_set_args(1, 60)?;
        Ok(chaos)
    }

    /// True when the run has no chaos at all (no text, or only blank ones).
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }

    fn get(&self, profile: Profile) -> Option<&ChaosSpec> {
        self.specs.iter().find(|s| s.profile == profile)
    }

    /// One CH-01 header per spec (`chaos=flush:pack seed=7`).
    pub fn headers(&self) -> Vec<String> {
        self.specs.iter().map(header).collect()
    }

    /// The headers joined with `"; "`, as the CLI prints them; `None`
    /// without chaos, so `run.json` gets no key.
    pub fn header_line(&self) -> Option<String> {
        (!self.is_empty()).then(|| self.headers().join("; "))
    }

    /// The headers a plan reports: a seed the text did not name reads
    /// `seed=<per run>`, since each real run takes its own run id's hash.
    pub fn plan_headers(&self) -> Vec<String> {
        self.specs
            .iter()
            .zip(&self.named_seed)
            .map(|(spec, &named)| {
                let line = header(spec);
                match line.rsplit_once(" seed=") {
                    Some((head, _)) if !named => format!("{head} seed={PER_RUN_SEED}"),
                    _ => line,
                }
            })
            .collect()
    }

    /// [`Self::plan_headers`] joined as [`Self::header_line`] joins them.
    pub fn plan_header_line(&self) -> Option<String> {
        (!self.is_empty()).then(|| self.plan_headers().join("; "))
    }

    /// Whether a row needs a `server_chaos_set` (`net` or `flush`).
    pub fn has_server_profile(&self) -> bool {
        self.get(Profile::Net).is_some() || self.get(Profile::Flush).is_some()
    }

    /// The run's cpu load, when it has a `cpu` spec.
    pub fn cpu_stress(&self) -> Result<Option<CpuStress>, String> {
        let Some(spec) = self.get(Profile::Cpu) else {
            return Ok(None);
        };
        let knob = |k: &str| spec.knobs.get(k).map(String::as_str).unwrap_or_default();
        CpuStress::from_knobs(knob("cores"), knob("duty"))
            .map(Some)
            .map_err(|e| format!("chaos cpu: {e}"))
    }

    /// `server_chaos_set`'s arguments for one row, or `None` when the run
    /// has neither `net` nor `flush`. The seed is sent only when a profile
    /// reads it: the tool refuses a seed nothing uses.
    pub fn server_set_args(&self, entity_id: u32, ttl_s: u32) -> Result<Option<Value>, String> {
        let net = self.get(Profile::Net);
        let flush = self.get(Profile::Flush);
        if net.is_none() && flush.is_none() {
            return Ok(None);
        }
        let mut args = Map::new();
        args.insert("entity_id".into(), json!(entity_id));
        args.insert("ttl_s".into(), json!(ttl_s));
        let mut seeded = None;
        if let Some(spec) = net {
            args.insert("net".into(), net_args(spec)?);
            seeded = Some(spec.seed);
        }
        if let Some(spec) = flush {
            let (value, reads_seed) = flush_args(spec)?;
            args.insert("flush".into(), value);
            if reads_seed {
                seeded = Some(spec.seed);
            }
        }
        if let Some(seed) = seeded {
            args.insert("seed".into(), json!(seed));
        }
        let label: Vec<String> = [net, flush]
            .into_iter()
            .flatten()
            .map(|s| header(s).trim_start_matches("chaos=").to_string())
            .collect();
        args.insert("label".into(), json!(clip(&label.join(" "), LABEL_MAX)));
        Ok(Some(Value::Object(args)))
    }
}

/// `net` knobs as the tool takes them: whole milliseconds, percents with
/// at most one decimal, a reorder buffer size. A profile whose knobs are
/// all zero is refused, as the tool would.
fn net_args(spec: &ChaosSpec) -> Result<Value, String> {
    let pct = |k: &str| percent(spec, k);
    let v = json!({
        "latency": at_most(spec, "latency", whole(spec, "latency", "ms")?, LATENCY_MAX_MS)?,
        "jitter": at_most(spec, "jitter", whole(spec, "jitter", "ms")?, JITTER_MAX_MS)?,
        "loss": pct("loss")?,
        "dup": pct("dup")?,
        "reorder": at_most(spec, "reorder", whole(spec, "reorder", "")?, REORDER_MAX)?,
    });
    let all_zero = ["latency", "jitter", "loss", "dup", "reorder"]
        .iter()
        .all(|k| v[k].as_f64() == Some(0.0));
    if all_zero {
        return Err("chaos net: every knob is 0, so it would change nothing".into());
    }
    Ok(v)
}

/// `flush` knobs for the resolved mode, and whether the mode reads the seed
/// (`reorder`, and `shift` = `sweep`).
fn flush_args(spec: &ChaosSpec) -> Result<(Value, bool), String> {
    let mode = spec
        .knobs
        .get("mode")
        .map(String::as_str)
        .unwrap_or_default();
    match mode {
        "delay" => {
            let ms = at_most(
                spec,
                "delay_ms",
                whole(spec, "delay_ms", "ms")?,
                DELAY_MAX_MS,
            )?;
            Ok((json!({ "mode": mode, "delay_ms": ms }), false))
        }
        "reorder" => Ok((json!({ "mode": mode }), true)),
        "pack" => {
            let records = whole(spec, "records", "")?;
            if !(1..=PACK_RECORDS_MAX).contains(&records) {
                return Err(format!(
                    "chaos flush: records {records} is not 1 to {PACK_RECORDS_MAX} (one packet)"
                ));
            }
            Ok((json!({ "mode": mode, "records": records }), false))
        }
        "shift" => match spec.knobs.get("shift").map(|s| s.trim()) {
            Some("sweep") => Ok((json!({ "mode": mode, "shift": "sweep" }), true)),
            _ => {
                let len = whole(spec, "shift", "")?;
                if len != 0 && !(2..=SHIFT_LEN_MAX).contains(&len) {
                    return Err(format!(
                        "chaos flush: shift {len} is not 0 or 2 to {SHIFT_LEN_MAX}"
                    ));
                }
                Ok((json!({ "mode": mode, "shift": len }), false))
            }
        },
        other => Err(format!("chaos flush: unknown mode {other:?}")),
    }
}

/// `value`, refused over the server's `max` for `knob`.
fn at_most(spec: &ChaosSpec, knob: &str, value: u32, max: u32) -> Result<u32, String> {
    if value > max {
        return Err(format!(
            "chaos {}: {knob} {value} is over the server's {max}",
            spec.profile.name()
        ));
    }
    Ok(value)
}

/// A whole number knob, with `unit` allowed as a suffix (`120ms`).
fn whole(spec: &ChaosSpec, knob: &str, unit: &str) -> Result<u32, String> {
    let raw = spec.knobs.get(knob).map(|s| s.trim()).unwrap_or("0");
    let digits = if unit.is_empty() {
        raw
    } else {
        raw.strip_suffix(unit).unwrap_or(raw)
    };
    digits.parse().map_err(|_| {
        format!(
            "chaos {}: {knob} {raw:?} is not a whole number",
            spec.profile.name()
        )
    })
}

/// A percent knob (`0.5`, `1%`): 0 to 100, at most one decimal.
fn percent(spec: &ChaosSpec, knob: &str) -> Result<f64, String> {
    let raw = spec.knobs.get(knob).map(|s| s.trim()).unwrap_or("0");
    let bad = || {
        format!("chaos net: {knob} {raw:?} is not a percent from 0 to 100 with at most one decimal")
    };
    let value: f64 = raw
        .strip_suffix('%')
        .unwrap_or(raw)
        .parse()
        .map_err(|_| bad())?;
    let tenths = value * 10.0;
    if !value.is_finite()
        || value < 0.0
        || value > PERCENT_MAX
        || (tenths - tenths.round()).abs() > 1e-9
    {
        return Err(bad());
    }
    Ok(value)
}

/// At most `max` characters, cut on a character boundary.
fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
