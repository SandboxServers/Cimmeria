//! The effect-script registry: `script_name` to [`EffectScript`].
//!
//! The scripts themselves are in `cimmeria-cell-effect-scripts`, a leaf only
//! the composition root depends on (#962 step 4,
//! `docs/architecture/plugin-architecture.md` §4.4). The root builds an
//! [`EffectScripts`] from that crate's table at startup, the cell installs
//! it on its `SpaceManager` (`SpaceManager::install_effect_scripts`) before
//! anything spawns, and [`super::dispatch_by_name`] /
//! [`super::dispatch_on_remove`] look scripts up in it. So adding or editing
//! a script rebuilds that crate and the root, not the cell track.
//!
//! - **Lookup is by name.** A `HashMap` keyed by the exact, case-sensitive
//!   `script_name`; registration order never changes which script a name
//!   finds. [`EffectScripts::names`] reports the order for logs and tests.
//! - **A bad table fails at startup.** [`EffectScripts::build`] refuses a
//!   duplicate name ([`EffectScriptError::DuplicateScript`]) and an empty
//!   one ([`EffectScriptError::EmptyName`]), and the orchestrator refuses to
//!   start the cell with an empty registry.
//! - **A missing registration warns.** [`EffectScripts::unregistered`] lists
//!   the `script_name`s the loaded effect rows carry that no script answers;
//!   the cell logs them in one WARN once the effect definitions load, and
//!   dispatch still logs `effect_script_unknown` per call and falls back.
//!
//! The registry is an `Arc`, so a clone is a reference-count bump, and every
//! script is a `&'static` value, so a lookup hands back a reference that
//! does not borrow the `SpaceManager` it came from.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use cimmeria_entity::abilities::EffectDef;

use super::EffectScript;
use crate::cell::space_manager::SpaceManager;

/// Why an effect-script table did not build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectScriptError {
    /// Two rows register the same `script_name`: one would shadow the other.
    DuplicateScript { name: &'static str },
    /// A row registers the empty name, which no effect row can carry.
    EmptyName,
}

impl fmt::Display for EffectScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateScript { name } => {
                write!(f, "effect script {name:?} is registered twice")
            }
            Self::EmptyName => write!(f, "an effect script is registered under the empty name"),
        }
    }
}

impl std::error::Error for EffectScriptError {}

struct Inner {
    by_name: HashMap<&'static str, &'static dyn EffectScript>,
    /// Registration order, for logs and tests.
    names: Vec<&'static str>,
}

/// The built, frozen script registry the cell installs on its
/// `SpaceManager`. Cheap to clone.
#[derive(Clone)]
pub struct EffectScripts {
    inner: Arc<Inner>,
}

impl EffectScripts {
    /// A registry with no scripts: what a bare `SpaceManager` holds until the
    /// cell installs the real one. Every dispatch through it misses.
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(Inner {
                by_name: HashMap::new(),
                names: Vec::new(),
            }),
        }
    }

    /// Build a registry from `(script_name, script)` rows, in order. Fails on
    /// a duplicate or empty name.
    pub fn build(
        entries: impl IntoIterator<Item = (&'static str, &'static dyn EffectScript)>,
    ) -> Result<Self, EffectScriptError> {
        let mut by_name = HashMap::new();
        let mut names = Vec::new();
        for (name, script) in entries {
            if name.is_empty() {
                return Err(EffectScriptError::EmptyName);
            }
            if by_name.insert(name, script).is_some() {
                return Err(EffectScriptError::DuplicateScript { name });
            }
            names.push(name);
        }
        Ok(Self {
            inner: Arc::new(Inner { by_name, names }),
        })
    }

    /// The script registered under `name` (exact, case-sensitive), or
    /// `None`.
    ///
    /// Naming convention matches the original game's authoring scheme:
    /// PascalCase, no underscores, e.g., `HealFocus` / `MeleeDamage`.
    pub fn lookup(&self, name: &str) -> Option<&'static dyn EffectScript> {
        self.inner.by_name.get(name).copied()
    }

    /// Whether a script is registered under `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.inner.by_name.contains_key(name)
    }

    /// The registered names, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.inner.names.iter().copied()
    }

    /// How many scripts are registered.
    pub fn len(&self) -> usize {
        self.inner.names.len()
    }

    /// Whether no script is registered.
    pub fn is_empty(&self) -> bool {
        self.inner.names.is_empty()
    }

    /// The `script_name`s `effect_defs` carry that no registered script
    /// answers, each with the effect ids that name it (both sorted, so the
    /// startup log is stable). Such an effect falls back to the legacy NVP
    /// path at dispatch, with an `effect_script_unknown` WARN per call.
    pub fn unregistered<'a>(
        &self,
        effect_defs: impl IntoIterator<Item = &'a EffectDef>,
    ) -> Vec<(String, Vec<i32>)> {
        let mut missing: BTreeMap<&str, Vec<i32>> = BTreeMap::new();
        for def in effect_defs {
            if let Some(name) = def.script_name.as_deref() {
                if !self.contains(name) {
                    missing.entry(name).or_default().push(def.effect_id);
                }
            }
        }
        missing
            .into_iter()
            .map(|(name, mut ids)| {
                ids.sort_unstable();
                (name.to_string(), ids)
            })
            .collect()
    }
}

impl SpaceManager {
    /// The installed effect-script registry.
    pub fn effect_scripts(&self) -> &EffectScripts {
        &self.effect_scripts
    }

    /// Install the effect-script registry. The cell service calls this once,
    /// beside `install_plugins` and before anything spawns (the spawn-time
    /// cover hold runs Cover Stance); tests call it on the managers they
    /// build.
    pub fn install_effect_scripts(&mut self, scripts: EffectScripts) {
        self.effect_scripts = scripts;
    }
}

impl Default for EffectScripts {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for EffectScripts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EffectScripts")
            .field("names", &self.inner.names)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::super::EffectContext;
    use super::*;

    struct First;
    impl EffectScript for First {
        fn on_apply(&self, _ctx: &mut EffectContext) {}
    }
    struct Second;
    impl EffectScript for Second {
        fn on_apply(&self, _ctx: &mut EffectContext) {}
    }

    fn same(a: &'static dyn EffectScript, b: &'static dyn EffectScript) -> bool {
        std::ptr::addr_eq(a as *const dyn EffectScript, b as *const dyn EffectScript)
    }

    #[test]
    fn a_built_registry_finds_each_script_by_its_exact_name() {
        let scripts = EffectScripts::build([
            ("First", &First as &dyn EffectScript),
            ("Second", &Second as &dyn EffectScript),
        ])
        .unwrap();
        assert!(same(scripts.lookup("First").unwrap(), &First));
        assert!(same(scripts.lookup("Second").unwrap(), &Second));
        assert!(scripts.lookup("first").is_none(), "case-sensitive");
        assert!(scripts.lookup("").is_none());
        assert!(scripts.lookup("Third").is_none());
        assert_eq!(scripts.names().collect::<Vec<_>>(), ["First", "Second"]);
        assert_eq!(scripts.len(), 2);
    }

    /// Registration order reaches `names` only; the same rows in either
    /// order resolve every name to the same script.
    #[test]
    fn lookup_does_not_depend_on_registration_order() {
        let forward = EffectScripts::build([
            ("First", &First as &dyn EffectScript),
            ("Second", &Second as &dyn EffectScript),
        ])
        .unwrap();
        let reverse = EffectScripts::build([
            ("Second", &Second as &dyn EffectScript),
            ("First", &First as &dyn EffectScript),
        ])
        .unwrap();
        for name in ["First", "Second"] {
            assert!(same(
                forward.lookup(name).unwrap(),
                reverse.lookup(name).unwrap()
            ));
        }
    }

    /// #962 test rule: a duplicate id fails the build, naming the script,
    /// so the orchestrator refuses to start instead of one row silently
    /// shadowing the other.
    #[test]
    fn a_duplicate_script_name_fails_the_build() {
        let err = EffectScripts::build([
            ("First", &First as &dyn EffectScript),
            ("Second", &Second as &dyn EffectScript),
            ("First", &Second as &dyn EffectScript),
        ])
        .unwrap_err();
        assert_eq!(err, EffectScriptError::DuplicateScript { name: "First" });
        assert!(err.to_string().contains("\"First\""), "{err}");
    }

    #[test]
    fn an_empty_script_name_fails_the_build() {
        let err = EffectScripts::build([("", &First as &dyn EffectScript)]).unwrap_err();
        assert_eq!(err, EffectScriptError::EmptyName);
    }

    #[test]
    fn the_empty_registry_finds_nothing() {
        let scripts = EffectScripts::empty();
        assert!(scripts.is_empty());
        assert!(scripts.lookup("HealHealth").is_none());
        assert!(EffectScripts::default().is_empty());
    }

    /// #962 test rule: a missing registration is visible. Every script name
    /// an effect row carries that no script answers is listed, with its
    /// effect ids; registered names and script-less rows are not.
    #[test]
    fn unregistered_lists_the_effect_rows_no_script_answers() {
        let scripts = EffectScripts::build([("First", &First as &dyn EffectScript)]).unwrap();
        let def = |effect_id: i32, script: Option<&str>| EffectDef {
            effect_id,
            script_name: script.map(str::to_string),
            ..Default::default()
        };
        let defs = [
            def(30, Some("Missing")),
            def(10, Some("First")),
            def(20, None),
            def(5, Some("Missing")),
            def(7, Some("AlsoMissing")),
        ];
        assert_eq!(
            scripts.unregistered(&defs),
            vec![
                ("AlsoMissing".to_string(), vec![7]),
                ("Missing".to_string(), vec![5, 30]),
            ]
        );
        assert!(EffectScripts::build([
            ("First", &First as &dyn EffectScript),
            ("Missing", &Second as &dyn EffectScript),
            ("AlsoMissing", &Second as &dyn EffectScript),
        ])
        .unwrap()
        .unregistered(&defs)
        .is_empty());
    }
}
