//! Every way Cimmeria's launcher makes the player's client differ from
//! the stock 2009 install, for the "Changes to your client" list.
//!
//! The player either lets the launcher download the stock client or
//! points it at a copy they already have; either way the launcher then
//! changes that copy. This module is the one place that says how, so the
//! list stays complete: the launcher's own setup steps, one row per
//! manifest patch, what goes into `SGW.exe` at launch, and what the
//! server sends while the game runs.
//!
//! A manifest patch is described by its manifest `title` and
//! `description` when it has them (they are signed with the manifest),
//! else by [`builtin_description`] for the patches published before
//! those fields existed, else by its id. Nothing is ever left out.
//!
//! Pure: [`list`] takes plain inputs and is unit-tested without egui.

use crate::manifest::PatchEntry;
use crate::overlay_meta::{DEFAULT_ID_PREFIX, OVERLAY_DESCRIPTION, OVERLAY_TITLE};
use crate::state::InstalledState;

/// Where a change comes from; the list is grouped by this, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeGroup {
    /// Done by the launcher itself on install and before each launch.
    Setup,
    /// A patch set or file overlay from the content manifest.
    Patch,
    /// Code added to `SGW.exe` when the launcher starts it.
    Launch,
    /// What the server does to the client while you play.
    Server,
}

impl ChangeGroup {
    pub fn heading(self) -> &'static str {
        match self {
            Self::Setup => "Launcher setup",
            Self::Patch => "Patched files",
            Self::Launch => "Added when the game starts",
            Self::Server => "Sent by the server while you play",
        }
    }
}

/// Whether the change is in effect on this install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    /// Rewritten or re-checked before every launch.
    EveryLaunch,
    /// Done once, when the stock client is installed or adopted.
    AtInstall,
    /// The launcher's install record lists this patch.
    Applied,
    /// In the manifest, not applied yet.
    Pending,
    /// Goes into `SGW.exe` on the next "Launch SGW.exe".
    OnLaunch,
    /// The player turned it off, or has not opted in.
    Off,
    /// Happens during play, driven by the server.
    DuringPlay,
}

impl ChangeStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::EveryLaunch => "before every launch",
            Self::AtInstall => "at install",
            Self::Applied => "applied",
            Self::Pending => "applied on the next Install / Update",
            Self::OnLaunch => "on",
            Self::Off => "off",
            Self::DuringPlay => "during play",
        }
    }
}

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientChange {
    pub group: ChangeGroup,
    pub title: String,
    pub description: String,
    pub status: ChangeStatus,
    /// The manifest id, for patch rows.
    pub patch_id: Option<String>,
}

/// What [`list`] needs to know about this install.
pub struct ChangeInputs<'a> {
    /// The manifest's patches, or empty before the manifest is fetched.
    pub patches: &'a [PatchEntry],
    pub installed: &'a InstalledState,
    pub client_patches_enabled: bool,
    pub telemetry_opted_in: bool,
}

/// Every change, grouped in [`ChangeGroup`] order.
pub fn list(inputs: &ChangeInputs<'_>) -> Vec<ClientChange> {
    let mut out = setup_changes();
    out.extend(
        inputs
            .patches
            .iter()
            .map(|p| patch_change(p, inputs.installed)),
    );
    out.extend(launch_changes(inputs));
    out.push(ClientChange {
        group: ChangeGroup::Server,
        title: "Cooked game data".into(),
        description: "The server sends its own cooked data (missions, dialogs, items and \
            the rest) into Documents\\My Games\\Firesky\\SGWGame\\Cache.en-US, where the \
            client reads it ahead of its installed files. The stock game did the same with \
            the original servers' data. \"Reset client cache\" clears it."
            .into(),
        status: ChangeStatus::DuringPlay,
        patch_id: None,
    });
    out
}

fn setup_changes() -> Vec<ClientChange> {
    vec![
        ClientChange {
            group: ChangeGroup::Setup,
            title: "Login servers".into(),
            description: "Rewrites Working\\SGWGame\\Content\\UI\\Startup\\Login\\\
                LoginInternal.lua with the login servers set above, in place of the \
                original servers, which are gone. The game cannot log in without it."
                .into(),
            status: ChangeStatus::EveryLaunch,
            patch_id: None,
        },
        ClientChange {
            group: ChangeGroup::Setup,
            title: "ASLR off in SGW.exe".into(),
            description: "Clears one flag in SGW.exe's header (the byte at offset 0x186) so \
                it always loads at the same address. The client-patches DLL and telemetry \
                depend on that address. The same one-byte change as the modding kit's \
                \"Fix ASLR\"."
                .into(),
            status: ChangeStatus::EveryLaunch,
            patch_id: None,
        },
        ClientChange {
            group: ChangeGroup::Setup,
            title: "Bundled data folder renamed".into(),
            description: "Renames Working\\SGWGame\\Cache.en-US to SourceCache.en-us, the \
                read-only folder the client expects its shipped data in. The stock install \
                puts it under the wrong name, and the client logs an error for it."
                .into(),
            status: ChangeStatus::AtInstall,
            patch_id: None,
        },
    ]
}

fn patch_change(patch: &PatchEntry, installed: &InstalledState) -> ClientChange {
    let builtin = builtin_description(&patch.id);
    let title = patch
        .title
        .clone()
        .or_else(|| builtin.map(|(t, _)| t.to_string()))
        .unwrap_or_else(|| patch.id.clone());
    let description = patch
        .description
        .clone()
        .or_else(|| builtin.map(|(_, d)| d.to_string()))
        .unwrap_or_else(|| {
            format!(
                "Patch {} from the content manifest. It has no description yet.",
                patch.id
            )
        });
    let status = if installed.has_applied(&patch.state_key()) {
        ChangeStatus::Applied
    } else {
        ChangeStatus::Pending
    };
    ClientChange {
        group: ChangeGroup::Patch,
        title,
        description,
        status,
        patch_id: Some(patch.id.clone()),
    }
}

fn launch_changes(inputs: &ChangeInputs<'_>) -> Vec<ClientChange> {
    let patches_description = if inputs.client_patches_enabled {
        "Adds cimmeria-client-patches.dll to SGW.exe when it starts. It finishes client \
         features the 2009 build shipped half-built: today, the Black Market window. It \
         checks it is hooking the exact SGW.exe build it was made for and does nothing \
         otherwise, sends nothing to the server beyond the game's own messages, and writes \
         its log next to SGW.exe. Turn it off with \"Load client patches\"."
    } else {
        "Off: you turned off \"Load client patches\", so SGW.exe starts without \
         cimmeria-client-patches.dll and the Black Market window stays unavailable."
    };
    let telemetry_description = if inputs.telemetry_opted_in {
        "On, because you opted in. While the game runs, the launcher reads the client's \
         log files and uploads them, with this install's random id, to the Cimmeria \
         server so crashes and bugs can be traced. It writes \
         Working\\Binaries\\sessions\\current-session.json for that. It adds no code to \
         SGW.exe."
    } else {
        "Off. Telemetry is opt-in: the launcher reads and sends nothing unless you turn it \
         on."
    };
    vec![
        ClientChange {
            group: ChangeGroup::Launch,
            title: "Client patches DLL".into(),
            description: patches_description.into(),
            status: if inputs.client_patches_enabled {
                ChangeStatus::OnLaunch
            } else {
                ChangeStatus::Off
            },
            patch_id: None,
        },
        ClientChange {
            group: ChangeGroup::Launch,
            title: "Telemetry".into(),
            description: telemetry_description.into(),
            status: if inputs.telemetry_opted_in {
                ChangeStatus::OnLaunch
            } else {
                ChangeStatus::Off
            },
            patch_id: None,
        },
    ]
}

/// Title and description for the patches published before the manifest
/// carried them. Keep in step with `data/client-patches/README.md`.
pub fn builtin_description(id: &str) -> Option<(&'static str, &'static str)> {
    let known = match id {
        "001-dialog-portraits" => (
            "Dialog portraits",
            "Shows the speaker's portrait in NPC dialog windows, which the stock UI \
             never displays. Changes TaharezLook.scheme and the Dialog and Blurb window \
             layouts and scripts.",
        ),
        "002-castle-ring-transport" => (
            "Castle ring transport",
            "Adds the ring station on the CellBlock stasis-hall pad and the ring rig on the \
             Armory pad, which mission 688 needs. Changes two Castle CellBlock map files.",
        ),
        "003-cooked-data" => (
            "Merged cooked data",
            "Replaces three bundled data files (Kismet sequence events, Kismet set events \
             and interaction sets) with versions that match the server's, and adds \
             CookedBehaviorEvents.pak.",
        ),
        "004-log-config" => (
            "Client debug log",
            "Adds SGWLogConfig.xml so SGW.exe writes SGWDebugLog.log, which \"Upload Debug \
             Logs\" and telemetry read. Nothing is sent unless you use one of them.",
        ),
        "005-login-delay" => (
            "Login screen delay",
            "Makes the login screen wait 19 seconds so the gate-dialing intro finishes \
             first. Changes eula.lua.",
        ),
        "006-gate-sound-bank" => (
            "Gate sound bank",
            "Copies the client's own prp_gen gate sounds into Audio\\UI, where the game \
             looks for them.",
        ),
        _ if id.starts_with(DEFAULT_ID_PREFIX) => (OVERLAY_TITLE, OVERLAY_DESCRIPTION),
        _ => return None,
    };
    Some(known)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PatchRoot;

    fn patch(id: &str) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("{id}.zip"),
            size: 1,
            sha256: "h".into(),
            after: None,
            root: PatchRoot::InstallDir,
            title: None,
            description: None,
        }
    }

    fn inputs<'a>(
        patches: &'a [PatchEntry],
        installed: &'a InstalledState,
        dll: bool,
        telemetry: bool,
    ) -> ChangeInputs<'a> {
        ChangeInputs {
            patches,
            installed,
            client_patches_enabled: dll,
            telemetry_opted_in: telemetry,
        }
    }

    /// Every manifest patch gets a row, in manifest order, even one the
    /// launcher has never heard of: the list must never hide a change.
    #[test]
    fn every_manifest_patch_is_listed() {
        let patches = [patch("001-dialog-portraits"), patch("999-future")];
        let state = InstalledState::default();
        let rows = list(&inputs(&patches, &state, true, false));
        let ids: Vec<_> = rows.iter().filter_map(|r| r.patch_id.as_deref()).collect();
        assert_eq!(ids, ["001-dialog-portraits", "999-future"]);
        let unknown = rows
            .iter()
            .find(|r| r.patch_id.as_deref() == Some("999-future"))
            .unwrap();
        assert_eq!(unknown.title, "999-future");
        assert!(unknown.description.contains("no description"));
    }

    /// The manifest's own text wins over the built-in catalog.
    #[test]
    fn manifest_text_overrides_the_builtin_catalog() {
        let mut p = patch("001-dialog-portraits");
        p.title = Some("Portraits v2".into());
        p.description = Some("New text".into());
        let state = InstalledState::default();
        let rows = list(&inputs(std::slice::from_ref(&p), &state, true, false));
        let row = rows.iter().find(|r| r.patch_id.is_some()).unwrap();
        assert_eq!(row.title, "Portraits v2");
        assert_eq!(row.description, "New text");
    }

    /// Every patch set in `data/client-patches/` has built-in text, and
    /// it matches the title and description in the patch's spec (which
    /// `cimmeria-patchset build` prints into new manifest entries), so
    /// the manifest and the fallback never tell the player two stories.
    #[test]
    fn builtin_catalog_matches_every_patch_spec() {
        let dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches");
        let mut seen = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let spec_path = entry.unwrap().path().join("patch.json");
            if !spec_path.is_file() {
                continue;
            }
            let spec: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&spec_path).unwrap()).unwrap();
            let id = spec["id"].as_str().unwrap();
            let (title, description) = builtin_description(id)
                .unwrap_or_else(|| panic!("{id} has a spec but no built-in description"));
            assert_eq!(spec["title"], title, "{id} title");
            assert_eq!(spec["description"], description, "{id} description");
            seen += 1;
        }
        assert!(seen > 0, "no patch specs found under {}", dir.display());
    }

    #[test]
    fn overlay_ids_use_the_overlay_text() {
        let (title, _) = builtin_description("bm-ui-overlay-0123456789ab").unwrap();
        assert_eq!(title, OVERLAY_TITLE);
    }

    /// A `sgw_game` patch is applied under its `@sgw_game` state key.
    #[test]
    fn status_follows_the_install_record() {
        let mut overlay = patch("bm-ui-overlay-0123456789ab");
        overlay.root = PatchRoot::SgwGame;
        let patches = [patch("001-dialog-portraits"), overlay];
        let state = InstalledState {
            applied_patches: vec![
                "001-dialog-portraits".into(),
                "bm-ui-overlay-0123456789ab".into(),
            ],
            ..InstalledState::default()
        };
        let rows = list(&inputs(&patches, &state, true, false));
        let status = |id: &str| {
            rows.iter()
                .find(|r| r.patch_id.as_deref() == Some(id))
                .unwrap()
                .status
        };
        assert_eq!(status("001-dialog-portraits"), ChangeStatus::Applied);
        // Recorded under the plain id by a launcher older than `root`,
        // so it will be applied again, in the right place.
        assert_eq!(status("bm-ui-overlay-0123456789ab"), ChangeStatus::Pending);
    }

    #[test]
    fn launch_rows_follow_the_settings() {
        let state = InstalledState::default();
        let find = |rows: &[ClientChange], title: &str| {
            rows.iter().find(|r| r.title == title).unwrap().status
        };
        let off = list(&inputs(&[], &state, false, false));
        assert_eq!(find(&off, "Client patches DLL"), ChangeStatus::Off);
        assert_eq!(find(&off, "Telemetry"), ChangeStatus::Off);
        let on = list(&inputs(&[], &state, true, true));
        assert_eq!(find(&on, "Client patches DLL"), ChangeStatus::OnLaunch);
        assert_eq!(find(&on, "Telemetry"), ChangeStatus::OnLaunch);
    }

    /// The launcher's own changes are listed before the manifest is
    /// fetched, and the groups come out in display order.
    #[test]
    fn setup_rows_are_listed_without_a_manifest_and_groups_are_ordered() {
        let state = InstalledState::default();
        let rows = list(&inputs(
            &[patch("001-dialog-portraits")],
            &state,
            true,
            false,
        ));
        let titles: Vec<_> = rows.iter().map(|r| r.title.as_str()).collect();
        for t in [
            "Login servers",
            "ASLR off in SGW.exe",
            "Bundled data folder renamed",
        ] {
            assert!(titles.contains(&t), "missing {t}");
        }
        let order = |g| match g {
            ChangeGroup::Setup => 0,
            ChangeGroup::Patch => 1,
            ChangeGroup::Launch => 2,
            ChangeGroup::Server => 3,
        };
        assert!(rows
            .windows(2)
            .all(|w| order(w[0].group) <= order(w[1].group)));
    }
}
