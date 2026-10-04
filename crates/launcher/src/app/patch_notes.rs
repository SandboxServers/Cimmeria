//! The Patch Notes tab: the content manifest's patch titles and
//! descriptions, as published.
//!
//! The rows come only from the manifest the worker fetched and whose
//! Ed25519 signature verified ([`crate::manifest::fetch_manifest`]);
//! there is no second fetch path. The list is what the manifest offers,
//! in its order: it is not an install history and does not say which
//! patches this computer has (Settings › Advanced › Changes to your
//! client does). egui draws text as text, so a description that looks
//! like markup is shown as written.

use eframe::egui::{self, RichText};

use super::{theme, LauncherApp};
use crate::manifest::Manifest;

/// One patch as the tab shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NoteRow {
    pub id: String,
    pub title: String,
    /// True when the manifest gave no title and the id stands in.
    pub title_is_id: bool,
    /// `None` when the manifest gave no description, or a blank one.
    pub description: Option<String>,
}

/// Project the manifest's patches into rows, in manifest order. Blank
/// titles and descriptions count as absent; nothing is invented.
pub(super) fn rows(manifest: &Manifest) -> Vec<NoteRow> {
    manifest
        .patches
        .iter()
        .map(|p| {
            let title = p.title.as_deref().map(str::trim).filter(|t| !t.is_empty());
            NoteRow {
                id: p.id.clone(),
                title: title.unwrap_or(&p.id).to_owned(),
                title_is_id: title.is_none(),
                description: p
                    .description
                    .as_deref()
                    .map(str::trim)
                    .filter(|d| !d.is_empty())
                    .map(str::to_owned),
            }
        })
        .collect()
}

/// Shown in place of an absent description.
pub(super) const NO_DESCRIPTION: &str = "No description provided in the manifest.";

impl LauncherApp {
    pub(super) fn show_patch_notes(&mut self, ui: &mut egui::Ui) {
        let slot = &self.play.manifest;
        let count = slot.manifest.as_ref().map(|m| m.patches.len());
        ui.label(theme::eyebrow(&match count {
            Some(n) => format!("Current client patches · {n}"),
            None => "Current client patches".into(),
        }));
        ui.add_space(2.0);
        ui.label(RichText::new("Patch notes").size(22.0).strong());
        ui.label(theme::muted(
            "From the launcher's signed content manifest. This is the list of patches \
             the server currently publishes, not a record of what is installed on this \
             computer.",
        ));
        let fetching = slot.fetching;
        let error = slot.error.clone();
        let rows = slot.manifest.as_ref().map(rows);
        ui.horizontal(|ui| {
            let label = if fetching { "Refreshing…" } else { "Refresh" };
            if ui
                .add_enabled(!fetching, egui::Button::new(label).small())
                .clicked()
            {
                self.refresh_manifest();
            }
            if fetching {
                ui.spinner();
            }
        });
        if let Some(e) = &error {
            ui.colored_label(
                theme::DANGER,
                if rows.is_some() {
                    format!("Could not refresh: {e}. Showing the last list that verified.")
                } else {
                    format!("Could not load patch notes: {e}")
                },
            );
        }
        ui.add_space(6.0);
        let Some(rows) = rows else {
            if error.is_none() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(theme::muted("Loading patch information…"));
                });
            }
            return;
        };
        if rows.is_empty() {
            ui.label(theme::muted("The manifest lists no client patches."));
            return;
        }
        // The main column already scrolls; the list is part of it.
        for row in &rows {
            ui.separator();
            if !row.title_is_id {
                ui.label(RichText::new(&row.id).small().color(theme::MUTED));
            }
            ui.label(RichText::new(&row.title).size(16.0).strong());
            match &row.description {
                Some(d) => ui.label(RichText::new(d).color(theme::MUTED)),
                None => ui.label(RichText::new(NO_DESCRIPTION).italics().color(theme::MUTED)),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PatchEntry, PatchRoot, SeedEntry};

    fn patch(id: &str, title: Option<&str>, description: Option<&str>) -> PatchEntry {
        PatchEntry {
            id: id.into(),
            blob: format!("{id}.zip"),
            size: 1,
            sha256: "h".into(),
            after: None,
            root: PatchRoot::InstallDir,
            title: title.map(Into::into),
            description: description.map(Into::into),
        }
    }

    fn manifest(patches: Vec<PatchEntry>) -> Manifest {
        Manifest {
            schema: 1,
            min_launcher: None,
            seed: SeedEntry {
                blob: "s".into(),
                size: 1,
                sha256: "s".into(),
            },
            patches,
        }
    }

    #[test]
    fn rows_keep_manifest_order_and_publisher_text() {
        let m = manifest(vec![
            patch("002-b", Some("Second"), Some("Does B.")),
            patch("001-a", Some("First"), Some("Does A.")),
        ]);
        let r = rows(&m);
        assert_eq!(
            r.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["002-b", "001-a"]
        );
        assert_eq!(r[0].title, "Second");
        assert!(!r[0].title_is_id);
        assert_eq!(r[0].description.as_deref(), Some("Does B."));
    }

    // Missing and blank are the same to the player: the id stands in for
    // a title, and an absent description is labelled, never invented.
    #[test]
    fn missing_or_blank_fields_fall_back_honestly() {
        let m = manifest(vec![
            patch("003-c", None, None),
            patch("004-d", Some("   "), Some(" \n ")),
        ]);
        for r in rows(&m) {
            assert_eq!(r.title, r.id);
            assert!(r.title_is_id);
            assert_eq!(r.description, None);
        }
    }

    #[test]
    fn an_empty_manifest_has_no_rows() {
        assert!(rows(&manifest(Vec::new())).is_empty());
    }

    // Text is kept verbatim (trimmed only): Unicode and markup-looking
    // text reach the label as plain text, which egui never interprets.
    #[test]
    fn unicode_and_markup_like_text_is_kept_verbatim() {
        let m = manifest(vec![patch(
            "005-e",
            Some("Ринг <b>transport</b> 🚀"),
            Some("<script>alert(1)</script> & “quotes”"),
        )]);
        let r = &rows(&m)[0];
        assert_eq!(r.title, "Ринг <b>transport</b> 🚀");
        assert_eq!(
            r.description.as_deref(),
            Some("<script>alert(1)</script> & “quotes”")
        );
    }

    // No hard-coded count: every patch the manifest lists is a row.
    #[test]
    fn every_listed_patch_is_a_row() {
        let m = manifest(
            (0..23)
                .map(|i| patch(&format!("p{i}"), None, None))
                .collect(),
        );
        assert_eq!(rows(&m).len(), 23);
    }
}
