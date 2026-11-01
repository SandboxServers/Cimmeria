//! Evidence for judged tool results (contract C1): each result carries one
//! `evidence` object with a verdict, the C2 reason and the checks behind it.

#![cfg_attr(not(test), allow(dead_code))] // Callers land in AX-11..AX-44; AX-44 removes this.

pub mod claims;
pub mod corroborate;
pub mod fresh;
pub mod gate;
pub mod stop;
pub mod why;

pub use why::Why;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The verdict of a judged result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvVerdict {
    Confirmed,
    Denied,
    Inconclusive,
    Unjudged,
}

/// Where a check's reading came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Client,
    Server,
    Pixel,
    Bridge,
}

/// One measured (or unmeasured) fact behind a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub source: Source,
    /// `None` means "not measured".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<u64>,
    /// The C2 code this check implies when it is the deciding check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<Why>,
}

impl Check {
    /// A check with only its name, source, reading and requirement set.
    pub fn new(name: &str, source: Source, ok: Option<bool>, required: bool) -> Check {
        Check {
            name: name.to_string(),
            source,
            ok,
            required,
            before: None,
            after: None,
            frame: None,
            why: None,
        }
    }

    pub fn with_before(mut self, v: Value) -> Self {
        self.before = Some(v);
        self
    }

    pub fn with_after(mut self, v: Value) -> Self {
        self.after = Some(v);
        self
    }

    pub fn with_frame(mut self, frame: u64) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn with_why(mut self, why: Why) -> Self {
        self.why = Some(why);
        self
    }
}

/// A judged result: the verdict, its reason, the hint for it and the checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    pub verdict: EvVerdict,
    pub why: Why,
    pub hint: String,
    pub checks: Vec<Check>,
}

impl Evidence {
    /// Derive the verdict from the checks (the C1 rules, in order):
    /// the first required check that is false denies; the first required
    /// check not measured is inconclusive; no required check at all is
    /// inconclusive; otherwise the result is confirmed. Optional checks
    /// never change the verdict.
    pub fn judge(checks: Vec<Check>) -> Evidence {
        let (verdict, why) =
            if let Some(c) = checks.iter().find(|c| c.required && c.ok == Some(false)) {
                (EvVerdict::Denied, c.why.unwrap_or(Why::NoEffect))
            } else if let Some(c) = checks.iter().find(|c| c.required && c.ok.is_none()) {
                (EvVerdict::Inconclusive, c.why.unwrap_or(Why::NoPostRead))
            } else if !checks.iter().any(|c| c.required) {
                (EvVerdict::Inconclusive, Why::NoEvidence)
            } else {
                (EvVerdict::Confirmed, Why::Confirmed)
            };
        Evidence::from_verdict(verdict, why, checks)
    }

    /// A result that has not been judged yet.
    pub fn unjudged() -> Evidence {
        Evidence::from_verdict(EvVerdict::Unjudged, Why::NotYetJudged, Vec::new())
    }

    /// A result whose tool should have judged it but produced no checks.
    pub fn no_evidence() -> Evidence {
        Evidence::from_verdict(EvVerdict::Inconclusive, Why::NoEvidence, Vec::new())
    }

    /// For callers whose rule is not [`Evidence::judge`].
    pub fn from_verdict(verdict: EvVerdict, why: Why, checks: Vec<Check>) -> Evidence {
        Evidence {
            verdict,
            why,
            hint: why.hint().to_string(),
            checks,
        }
    }

    /// Does this evidence count as a confirmed result?
    pub fn counts(&self) -> bool {
        self.verdict == EvVerdict::Confirmed
    }

    /// The JSON the tool result carries under `evidence`.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("evidence serializes to JSON")
    }

    /// Read evidence back from JSON; `None` on any parse failure.
    pub fn from_value(v: &Value) -> Option<Evidence> {
        serde_json::from_value(v.clone()).ok()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
