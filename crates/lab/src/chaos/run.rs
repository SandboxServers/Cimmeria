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

/// The run's chaos specs, in the order given. Empty means no chaos.
#[derive(Debug, Clone, Default)]
pub struct RunChaos {
    pub specs: Vec<ChaosSpec>,
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
    /// knob, `burst`, a profile named twice, a knob value the consumer cannot
    /// use, and `net` with `flush` under two different seeds (the server
    /// keeps one seed per session).
    pub fn parse(texts: &[String], default_seed: u64) -> Result<Self, String> {
        let levels = Levels::embedded().map_err(|e| format!("chaos levels file: {e}"))?;
        let mut specs: Vec<ChaosSpec> = Vec::new();
        for text in texts.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            let spec =
                parse(text, &levels, default_seed).map_err(|e| format!("chaos {text:?}: {e}"))?;
            if spec.profile == Profile::Burst {
                return Err(format!("chaos {text:?}: {BURST_DEFERRED}"));
            }
            if specs.iter().any(|s| s.profile == spec.profile) {
                return Err(format!(
                    "chaos {text:?}: profile {} is named twice",
                    spec.profile.name()
                ));
            }
            specs.push(spec);
        }
        let chaos = Self { specs };
        if let (Some(net), Some(flush)) = (chaos.get(Profile::Net), chaos.get(Profile::Flush)) {
            if net.seed != flush.seed {
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
    let ms = |k: &str| whole(spec, k, "ms");
    let pct = |k: &str| percent(spec, k);
    let v = json!({
        "latency": ms("latency")?,
        "jitter": ms("jitter")?,
        "loss": pct("loss")?,
        "dup": pct("dup")?,
        "reorder": whole(spec, "reorder", "")?,
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
        "delay" => Ok((
            json!({ "mode": mode, "delay_ms": whole(spec, "delay_ms", "ms")? }),
            false,
        )),
        "reorder" => Ok((json!({ "mode": mode }), true)),
        "pack" => {
            let records = u16::try_from(whole(spec, "records", "")?)
                .map_err(|_| "chaos flush: records is over 65535".to_string())?;
            Ok((json!({ "mode": mode, "records": records }), false))
        }
        "shift" => match spec.knobs.get("shift").map(|s| s.trim()) {
            Some("sweep") => Ok((json!({ "mode": mode, "shift": "sweep" }), true)),
            _ => {
                let len = u16::try_from(whole(spec, "shift", "")?)
                    .map_err(|_| "chaos flush: shift is over 65535".to_string())?;
                Ok((json!({ "mode": mode, "shift": len }), false))
            }
        },
        other => Err(format!("chaos flush: unknown mode {other:?}")),
    }
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

/// A percent knob (`0.5`, `1%`): not negative, at most one decimal.
fn percent(spec: &ChaosSpec, knob: &str) -> Result<f64, String> {
    let raw = spec.knobs.get(knob).map(|s| s.trim()).unwrap_or("0");
    let bad = || format!("chaos net: {knob} {raw:?} is not a percent with at most one decimal");
    let value: f64 = raw
        .strip_suffix('%')
        .unwrap_or(raw)
        .parse()
        .map_err(|_| bad())?;
    let tenths = value * 10.0;
    if !value.is_finite() || value < 0.0 || (tenths - tenths.round()).abs() > 1e-9 {
        return Err(bad());
    }
    Ok(value)
}

/// At most `max` characters, cut on a character boundary.
fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(texts: &[&str]) -> Result<RunChaos, String> {
        let texts: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        RunChaos::parse(&texts, 7)
    }

    #[test]
    fn burst_is_refused_with_the_deferral() {
        let err = run(&["burst"]).unwrap_err();
        assert!(err.contains(BURST_DEFERRED), "{err}");
        assert!(run(&["cpu", "burst:count=5"]).is_err());
    }

    #[test]
    fn a_profile_named_twice_and_split_seeds_are_refused() {
        assert!(run(&["net", "net:break"])
            .unwrap_err()
            .contains("named twice"));
        assert!(run(&["net:seed=1", "flush:reorder:seed=2"])
            .unwrap_err()
            .contains("one server seed"));
        assert!(run(&["net:seed=3", "flush:reorder:seed=3"]).is_ok());
    }

    #[test]
    fn knobs_the_server_would_refuse_fail_at_parse() {
        for bad in [
            "net:latency=fast",
            "net:loss=0.05",
            "net:loss=-1",
            "net:latency=0,jitter=0,loss=0,dup=0,reorder=0",
            "flush:pack:records=70000",
            "flush:shift:shift=wide",
            "cpu:cores=0",
        ] {
            assert!(run(&[bad]).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn net_and_flush_go_in_one_set_call() {
        let chaos = run(&["net:latency=120ms,loss=1%", "flush:pack"]).unwrap();
        let args = chaos.server_set_args(42, 360).unwrap().unwrap();
        assert_eq!(args["entity_id"], 42);
        assert_eq!(args["ttl_s"], 360);
        assert_eq!(args["net"]["latency"], 120);
        assert_eq!(args["net"]["loss"], 1.0);
        assert_eq!(args["flush"], json!({ "mode": "pack", "records": 24 }));
        assert_eq!(args["seed"], 7, "net reads the seed");
        assert_eq!(
            args["label"],
            "net:latency=120ms,loss=1% seed=7 flush:pack seed=7"
        );
    }

    /// The tool refuses a seed nothing reads, so a pack-only set has none,
    /// and a reorder carries it.
    #[test]
    fn the_seed_goes_only_where_it_is_read() {
        let pack = run(&["flush:pack"]).unwrap();
        let args = pack.server_set_args(1, 60).unwrap().unwrap();
        assert!(args.get("seed").is_none());
        let reorder = run(&["flush:reorder"]).unwrap();
        assert_eq!(reorder.server_set_args(1, 60).unwrap().unwrap()["seed"], 7);
        let sweep = run(&["flush:shift"]).unwrap();
        let args = sweep.server_set_args(1, 60).unwrap().unwrap();
        assert_eq!(args["flush"]["shift"], "sweep");
        assert_eq!(args["seed"], 7);
    }

    #[test]
    fn cpu_alone_sends_nothing_to_the_server() {
        let chaos = run(&["cpu"]).unwrap();
        assert!(!chaos.has_server_profile());
        assert_eq!(chaos.server_set_args(1, 60).unwrap(), None);
        assert!(chaos.cpu_stress().unwrap().is_some());
        assert_eq!(
            chaos.header_line().as_deref(),
            Some(header(&chaos.specs[0]).as_str())
        );
    }

    #[test]
    fn no_chaos_has_no_header_and_empty_texts_are_skipped() {
        let chaos = run(&["", "  "]).unwrap();
        assert!(chaos.is_empty());
        assert_eq!(chaos.header_line(), None);
    }

    #[test]
    fn the_run_seed_is_stable_and_differs_by_run() {
        assert_eq!(seed_of("abc"), seed_of("abc"));
        assert_ne!(seed_of("abc"), seed_of("abd"));
    }
}
