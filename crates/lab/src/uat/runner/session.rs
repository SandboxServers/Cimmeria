//! Reaching a row's state (client running, logged in, in world as the
//! right character), the `.bug` anchor, the client build fingerprint, and
//! ending the run's session (TG-PIPE-08). Every call a row makes here is
//! recorded as a setup action on the row.

use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::players::Who;
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::{Anchor, RowEvidence};
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{ActionSpec, GoldenSpec, RowSpec, SectionSpec, ServerWait};
use crate::uat::tier::{Role, Tier};

/// How long the anchor waits for the `Bookmark <id> recorded` reply.
const ANCHOR_REPLY: Duration = Duration::from_secs(4);

fn letters_name(run_id: &str) -> String {
    let mut s: String = run_id.chars().filter(char::is_ascii_lowercase).collect();
    if let Some(f) = s.get_mut(..1) {
        f.make_ascii_uppercase();
    }
    s
}

/// Bundles written before AP-02 used at most ten run-id letters and no
/// section suffix. Keep that identity when adopting an older fresh run.
pub(crate) fn legacy_name(run_id: &str) -> String {
    letters_name(&run_id.chars().take(10).collect::<String>())
}

/// The section suffix keeps two fresh sections in one run distinct. A
/// collision is resolved while the manifest is built, then its chosen name
/// is persisted so changing the section list cannot rename a resumed player.
pub(crate) fn fresh_name(run_id: &str, section_id: &str, nonce: u32) -> String {
    let digest = Sha256::digest(format!("{section_id}:{nonce}"));
    let suffix: String = digest[..2]
        .iter()
        .map(|b| char::from(b'a' + *b % 26))
        .collect();
    format!("{}{}", letters_name(run_id), suffix)
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// The character this section plays: lab-account's, or this run's
    /// fresh one (`<first> <Runid>`; its list name is the last name).
    pub(crate) fn character_name(&self, spec: &SectionSpec) -> Option<String> {
        if spec.section.character == "fresh" {
            self.fresh.get(&spec.section.id).cloned()
        } else {
            self.req.lab_character.clone()
        }
    }

    /// `{name, archetype, level}` from the last character list seen.
    pub(crate) fn character_value(&self, spec: &SectionSpec) -> Value {
        let name = self.character_name(spec);
        let row = self.characters.iter().find(|c| {
            c.get("name").and_then(Value::as_str).is_some_and(|listed| {
                name.as_deref().is_some_and(|want| {
                    listed.eq_ignore_ascii_case(want)
                        || listed
                            .split_whitespace()
                            .any(|part| part.eq_ignore_ascii_case(want))
                })
            })
        });
        character_json(name, row)
    }

    /// [`Self::character_value`] for a fixture character. The list shows it
    /// by its full name (`Uat Fxpraxisdefa`) and the runner knows only the
    /// last name, so a row matches on the whole name or any one word of it.
    pub(crate) fn listed_character_value(&self, name: &str) -> Value {
        let row = self.characters.iter().find(|c| {
            c.get("name").and_then(Value::as_str).is_some_and(|n| {
                n.eq_ignore_ascii_case(name)
                    || n.split_whitespace().any(|w| w.eq_ignore_ascii_case(name))
            })
        });
        character_json(Some(name.to_string()), row)
    }

    pub(crate) async fn setup_call(
        &mut self,
        tool: &str,
        args: Value,
        ctx: &mut RowCtx,
    ) -> Result<Value, String> {
        self.setup_call_on(Who::P1, tool, args, ctx).await
    }

    /// Bring the client to `state`. Uses the flows by name, so a missing
    /// flow tool BLOCKs the row with its name.
    pub(crate) async fn ensure_state(
        &mut self,
        spec: &SectionSpec,
        state: &str,
        ctx: &mut RowCtx,
    ) -> Result<(), String> {
        if state == "any" {
            return Ok(());
        }
        let status = self.setup_call("lab_client_status", json!({}), ctx).await?;
        let running = status
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut login = status
            .get("login_state")
            .and_then(Value::as_str)
            .unwrap_or("not_started")
            .to_string();
        if state == "client_stopped" {
            if running {
                self.setup_call("lab_client_stop", json!({}), ctx).await?;
            }
            return Ok(());
        }
        if !running {
            self.setup_call("lab_client_start", json!({}), ctx).await?;
            login = "not_started".into();
            self.in_world_as = None;
        }
        if !matches!(login.as_str(), "character_select" | "in_world") {
            self.setup_call("lab_login", json!({}), ctx).await?;
            login = "character_select".into();
        }
        let want = self.character_name(spec);
        if state == "char_select" {
            if login == "in_world" {
                self.setup_call("lab_logout", json!({}), ctx).await?;
                self.in_world_as = None;
            }
            return Ok(());
        }
        // in_world as the right character.
        if login == "in_world" {
            // Caveat (FX-06 deviation 3): with no record of who is in world,
            // this assumes the lab character. A client someone left in world
            // as another character on the same account passes as the lab
            // character here, and `auto` may then chain from it.
            let current = self
                .in_world_as
                .clone()
                .or_else(|| self.req.lab_character.clone());
            if current.is_some() && current == want {
                // Remember who is in world even when the run found the client
                // there: fixture mode `auto` continues a chain only from a
                // client the runner knows is in world (D-FX10).
                self.in_world_as = current;
                return Ok(());
            }
            self.setup_call("lab_logout", json!({}), ctx).await?;
        }
        let name =
            want.ok_or("no character: set lab-account.json `character` or pass `character`")?;
        if spec.section.character == "fresh" {
            let f = spec
                .section
                .fresh
                .clone()
                .ok_or("character = fresh without [section.fresh]")?;
            let list = self.setup_call("lab_characters", json!({}), ctx).await?;
            let listed = list
                .get("characters")
                .and_then(Value::as_array)
                .is_some_and(|rows| {
                    rows.iter().any(|r| {
                        r.get("name").and_then(Value::as_str).is_some_and(|n| {
                            n.eq_ignore_ascii_case(&name)
                                || n.split_whitespace()
                                    .any(|part| part.eq_ignore_ascii_case(&name))
                        })
                    })
                });
            if !listed {
                // Another fresh section in this run may already have a
                // character; never free its slot to create this one.
                let mut protect: Vec<String> = self.req.lab_character.iter().cloned().collect();
                if let Some(rows) = list.get("characters").and_then(Value::as_array) {
                    protect.extend(rows.iter().filter_map(|r| {
                        let full = r.get("name")?.as_str()?;
                        self.fresh
                            .values()
                            .any(|last| {
                                full.split_whitespace()
                                    .any(|part| part.eq_ignore_ascii_case(last))
                            })
                            .then(|| full.to_string())
                    }));
                }
                self.setup_call(
                    "lab_ensure_character_slot",
                    json!({ "free_slots": 1, "protect": protect }),
                    ctx,
                )
                .await?;
                self.setup_call(
                    "lab_create_character",
                    json!({ "first": f.first, "last": name, "alignment": f.alignment,
                            "archetype": f.archetype, "gender": f.gender }),
                    ctx,
                )
                .await?;
            }
        }
        let played = self
            .setup_call("lab_play_character", json!({ "name": name }), ctx)
            .await?;
        self.in_world_as = Some(name);
        if spec.section.fresh.as_ref().is_some_and(|f| f.finish_intro) {
            // Play returns when the world HUD appears. The intro dialog may
            // arrive a few frames later, so its immediate flag is not enough.
            let dialog_open = if played.get("dialog_open").and_then(Value::as_bool) == Some(true) {
                true
            } else {
                self.setup_call(
                    "client_wait_for",
                    json!({
                        "lua_condition": "DialogWin ~= nil and DialogWin:isVisible()",
                        "timeout_ms": 15000,
                    }),
                    ctx,
                )
                .await?
                .get("met")
                .and_then(Value::as_bool)
                    == Some(true)
            };
            if dialog_open {
                // The intro cinematic can leave a letterbox over the dialog
                // buttons for several seconds after the dialog itself opens.
                let uncovered = self
                    .setup_call(
                        "client_wait_for",
                        json!({
                            "lua_condition": "not ((LetterboxedWin and LetterboxedWin:isVisible()) or (MoviePlayerWin and MoviePlayerWin:isVisible()))",
                            "timeout_ms": 20000,
                        }),
                        ctx,
                    )
                    .await?;
                if uncovered.get("met").and_then(Value::as_bool) != Some(true) {
                    return Err("intro dialog remained covered by the arrival cinematic".into());
                }
                self.setup_call("lab_finish_dialog", json!({}), ctx).await?;
            }
        }
        Ok(())
    }

    /// Type `.bug uat <row>` and read the bookmark reply: its id is the
    /// server's epoch ms, which gives the server clock offset.
    pub(crate) async fn anchor(&mut self, ctx: &mut RowCtx) {
        let note = format!("uat {}", ctx.row_id);
        let before = self.read_chat(ctx).await.unwrap_or_default();
        let sent = now_ms();
        let a = ActionSpec {
            chat: Some(format!(".bug {note}")),
            ..Default::default()
        };
        let rec = self.exec(&a, Role::Anchor, ctx).await;
        let ok = rec.ok;
        ctx.actions.push(rec);
        let mut anchor = Anchor {
            note: note.clone(),
            host_sent_ms: sent,
            ..Default::default()
        };
        if ok {
            let re = regex::Regex::new(r"Bookmark (\d+) recorded").expect("static regex");
            let t0 = std::time::Instant::now();
            while t0.elapsed() < ANCHOR_REPLY {
                let after = self.read_chat(ctx).await.unwrap_or_default();
                let (lines, _) = crate::uat::clause::new_lines(&before, &after);
                let id = lines
                    .iter()
                    .find_map(|l| re.captures(l).and_then(|c| c[1].parse::<u64>().ok()));
                if let Some(id) = id {
                    let seen = now_ms();
                    anchor.bookmark_id = Some(id);
                    anchor.host_seen_ms = Some(seen);
                    // The server stamped the id between send and seen.
                    anchor.server_offset_ms = Some(id as i64 - (sent + seen) / 2);
                    ctx.vars.insert("bookmark_id".into(), json!(id));
                    self.manifest.clocks["last_server_offset_ms"] = json!(anchor.server_offset_ms);
                    self.manifest.clocks["offset_source"] = json!("bookmark_id (.bug reply)");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
        ctx.anchor = Some(anchor);
    }

    /// TG-PIPE-08 (S1): end the run's session the way a player does, after
    /// the last row: `lab_logout` when in world, then `lab_disconnect` from
    /// character select, each checked against the server line it causes.
    /// Runs as the synthetic row [`SESSION_END_ROW`] of `spec` (the last
    /// row's section). `None`, with a log line, when there is nothing to
    /// end: the lease is gone, the client is not running, or it is already
    /// past character select.
    pub(crate) async fn end_run_session(&mut self, spec: &SectionSpec) -> Option<RowEvidence> {
        if let Some(r) = self.revocation_reason() {
            skipped("lease_revoked", &r);
            return None;
        }
        let status = self.inv.call("lab_client_status", json!({})).await;
        if !status.ok {
            skipped("status_failed", status.error.as_deref().unwrap_or_default());
            return None;
        }
        let running = status
            .json
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let login = status
            .json
            .get("login_state")
            .and_then(Value::as_str)
            .unwrap_or("not_started")
            .to_string();
        if !running {
            skipped("client_not_running", &login);
            return None;
        }
        let in_world = login == "in_world";
        if !in_world && login != "character_select" {
            skipped("not_logged_in", &login);
            return None;
        }
        let row = session_end_row(
            in_world,
            self.server.is_some(),
            self.req.account_name.as_deref(),
        );
        // The section's own account rule (`non-gm` BLOCKs) is about its
        // rows, not about ending the session the run left behind.
        let mut spec = spec.clone();
        spec.section.account = "gm".into();
        let ev = self.run_row(&spec, &row, vec![]).await;
        self.in_world_as = None;
        Some(ev)
    }
}

/// The row id the end of a run is recorded under (TG-PIPE-08).
pub(crate) const SESSION_END_ROW: &str = "_session_end";
/// How long each end-of-run server wait waits for its line.
const SESSION_END_WAIT_MS: u64 = 30_000;

fn skipped(reason: &'static str, detail: &str) {
    tracing::info!(target: "lab.uat", event = "run_session_end_skipped", reason, detail,
        "UAT run did not end the session: nothing to end");
}

/// The end-of-run row: logout (when in world) and disconnect, each
/// followed by a wait for the server line it causes (per the telemetry-gaps
/// shared contract, "S1 corrected assertion"). Without the server lab MCP
/// the waits are left out and the row drives the client only. `account`
/// pins both lines to this run's account; the runner knows no ids.
fn session_end_row(in_world: bool, with_server: bool, account: Option<&str>) -> RowSpec {
    let tool = |name: &str| ActionSpec {
        tool: Some(name.into()),
        args: Some(json!({})),
        ..Default::default()
    };
    let wait = |label: &str, log: &str, target: Option<&str>, reason: &str| {
        let mut fields = serde_json::Map::new();
        fields.insert("disconnect_reason".into(), json!(reason));
        if let Some(a) = account {
            fields.insert("account_name".into(), json!(a));
        }
        ActionSpec {
            wait_server: Some(ServerWait {
                log: log.into(),
                target: target.map(Into::into),
                fields: Some(fields),
                timeout_ms: SESSION_END_WAIT_MS,
            }),
            label: Some(label.into()),
            // A missing line still fails the row (grade counts every failed
            // step) but must not stop the disconnect after it: a client left
            // at character select keeps the account online.
            optional: true,
            ..Default::default()
        }
    };
    let mut steps = Vec::new();
    if in_world {
        steps.push(tool("lab_logout"));
        if with_server {
            steps.push(wait(
                "session_end",
                "player session ended",
                Some("session.end"),
                "logoff_character_select",
            ));
        }
    }
    steps.push(tool("lab_disconnect"));
    if with_server {
        steps.push(wait(
            "entities_cleaned",
            "Client entities cleaned up",
            None,
            "logoff",
        ));
    }
    RowSpec {
        id: SESSION_END_ROW.into(),
        depends_on: vec![],
        title: "end the run's session: logout and disconnect".into(),
        expected: "session.end disconnect_reason=logoff_character_select after the logout, \
                   then Client entities cleaned up disconnect_reason=logoff after the disconnect"
            .into(),
        required_native: Tier::N1,
        state: "any".into(),
        players: 1,
        known_issues: vec![],
        blocked: None,
        anchor: Some(false),
        relog: None,
        notes: Some(format!(
            "runner-level (TG-PIPE-08); the client was {} at the run's end",
            if in_world {
                "in world"
            } else {
                "at character select"
            }
        )),
        setup: vec![],
        steps,
        teardown: vec![],
        expect: vec![],
        evidence: vec![],
        // Not a spec row: never part of a golden fingerprint.
        golden: Some(GoldenSpec {
            skip: true,
            ..Default::default()
        }),
        pose: vec![],
        fixture: None,
    }
}

/// `{name, archetype, level, alignment}` from a character-list row, or
/// just the name without one.
fn character_json(name: Option<String>, row: Option<&Value>) -> Value {
    match row {
        Some(r) => json!({
            "name": name,
            "archetype": r.get("archetype"),
            "level": r.get("level"),
            "alignment": r.get("alignment"),
        }),
        None => json!({ "name": name }),
    }
}

fn sha256_file(p: &Path) -> Option<(String, u64)> {
    let bytes = std::fs::read(p).ok()?;
    let digest = Sha256::digest(&bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Some((hex, bytes.len() as u64))
}

/// The client build fingerprint: SHA-256 of `Binaries/SGW.exe` and of the
/// injected DLLs, from the install and DLL paths the supervisor uses.
pub fn client_fingerprint(install_dir: Option<&Path>, dlls: &[(&str, Option<&Path>)]) -> Value {
    let mut out = json!({});
    if let Some(dir) = install_dir {
        let exe = dir.join("Binaries").join("SGW.exe");
        out["sgw_exe"] = match sha256_file(&exe) {
            Some((h, n)) => json!({ "path": exe.display().to_string(), "sha256": h, "bytes": n }),
            None => json!({ "path": exe.display().to_string(), "error": "unreadable" }),
        };
    }
    for (name, path) in dlls {
        if let Some(p) = path {
            out[*name] = match sha256_file(p) {
                Some((h, n)) => json!({ "path": p.display().to_string(), "sha256": h, "bytes": n }),
                None => json!({ "path": p.display().to_string(), "error": "unreadable" }),
            };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_names_are_capitalized_letters() {
        assert_eq!(letters_name("dqzkfma"), "Dqzkfma");
        assert_eq!(letters_name(""), "");
        assert_ne!(
            fresh_name("dqzkfma", "praxis", 0),
            fresh_name("dqzkfma", "jaffa", 0)
        );
    }

    #[test]
    fn fingerprint_hashes_what_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("Binaries");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("SGW.exe"), b"abc").unwrap();
        let v = client_fingerprint(
            Some(tmp.path()),
            &[("telemetry_dll", Some(&tmp.path().join("missing.dll")))],
        );
        assert_eq!(
            v["sgw_exe"]["sha256"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(v["telemetry_dll"]["error"], "unreadable");
    }
}
