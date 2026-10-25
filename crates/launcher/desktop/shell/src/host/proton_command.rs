//! The Linux Proton picker: the Protons found and the saved choice (PP-10).
//! Only ids, names and kinds cross IPC; no path, root or version (R9).
use super::*;
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use super::launch::resources;
#[cfg(target_os = "linux")]
use cimmeria_launcher_engine::linux_proton::discovery::{self, ProtonKind};

/// Off Linux no Proton is ever listed, so there is no kind to name.
#[cfg(not(target_os = "linux"))]
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ProtonKind {}

/// One `proton_command` request. Unknown fields are refused.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProtonCommand {
    /// A struct variant, not a unit one: serde ignores unknown keys on a unit variant.
    Inspect {},
    /// `id: None` clears the saved choice.
    Select { id: Option<String> },
}

/// The picker's view: the Protons found, best first, and which one Play uses.
#[derive(Debug, Serialize)]
pub struct ProtonView {
    pub available: Vec<ProtonChoice>,
    pub selected: Option<String>,
    pub preferred: Option<String>,
}

/// One Proton the picker can offer.
#[derive(Debug, Serialize)]
pub struct ProtonChoice {
    pub id: String,
    pub name: String,
    pub kind: ProtonKind,
}

impl NativeHost {
    /// The Proton id the store saved, if any.
    pub(super) fn saved_proton(&self) -> Option<String> {
        self.with_state(|state| Ok(state.preferences().proton_preference.clone()))
            .ok()
            .flatten()
    }

    /// Lists the Protons found now, or saves `Select`'s id after it is checked
    /// against that list. Off Linux the view is empty and nothing is saved.
    pub fn proton_command(&self, request: ProtonCommand) -> Result<ProtonView, String> {
        #[cfg(target_os = "linux")]
        {
            self.linux_proton_command(request)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = request;
            Ok(ProtonView {
                available: Vec::new(),
                selected: None,
                preferred: None,
            })
        }
    }

    /// Every Proton discovery finds now, ranked. Tests read the fixture, never HOME (R3).
    #[cfg(all(target_os = "linux", test))]
    fn installs(&self) -> Vec<discovery::ProtonInstall> {
        match self.proton_fixture.as_ref() {
            Some((home, tools)) => resources::found(Some((home.as_path(), tools.as_path()))),
            None => Vec::new(),
        }
    }

    /// Every Proton discovery finds now, ranked.
    #[cfg(all(target_os = "linux", not(test)))]
    fn installs(&self) -> Vec<discovery::ProtonInstall> {
        resources::found(None)
    }

    #[cfg(target_os = "linux")]
    fn linux_proton_command(&self, request: ProtonCommand) -> Result<ProtonView, String> {
        let installs = self.installs();
        if let ProtonCommand::Select { id } = request {
            if let Some(wanted) = &id {
                if !installs.iter().any(|install| &install.id == wanted) {
                    return Err("unknown_proton".to_owned());
                }
            }
            self.with_state(|state| state.save_proton_preference(id))
                .map_err(|_| "preference_unsaved".to_owned())?;
        }
        let preferred = self.saved_proton();
        let selected = discovery::choose(&installs, preferred.as_deref()).map(|i| i.id.clone());
        Ok(ProtonView {
            available: installs
                .iter()
                .map(|install| ProtonChoice {
                    id: install.id.clone(),
                    name: install.name.clone(),
                    kind: install.kind,
                })
                .collect(),
            selected,
            preferred,
        })
    }
}

#[cfg(all(target_os = "linux", test))]
#[path = "proton_command_tests.rs"]
mod tests;
