//! Reaching a row's state (client running, logged in, in world as the
//! right character), the `.bug` anchor, and the client build fingerprint.
//! Every call made here is recorded as a setup action on the row.

use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::players::Who;
use super::{now_ms, RowCtx, Runner};
use crate::uat::evidence::Anchor;
use crate::uat::invoke::ToolInvoker;
use crate::uat::spec::{ActionSpec, SectionSpec};
use crate::uat::tier::Role;

/// How long the anchor waits for the `Bookmark <id> recorded` reply.
const ANCHOR_REPLY: Duration = Duration::from_secs(4);

fn letters_name(run_id: &str) -> String {
    let mut s: String = run_id
        .chars()
        .filter(char::is_ascii_lowercase)
        .take(10)
        .collect();
    if let Some(f) = s.get_mut(..1) {
        f.make_ascii_uppercase();
    }
    s
}

impl<I: ToolInvoker> Runner<'_, I> {
    /// The character this section plays: lab-account's, or this run's
    /// fresh one (`<first> <Runid>`; its list name is the last name).
    pub(crate) fn character_name(&self, spec: &SectionSpec) -> Option<String> {
        if spec.section.character == "fresh" {
            Some(letters_name(&self.manifest.run_id))
        } else {
            self.req.lab_character.clone()
        }
    }

    /// `{name, archetype, level}` from the last character list seen.
    pub(crate) fn character_value(&self, spec: &SectionSpec) -> Value {
        let name = self.character_name(spec);
        let row = self
            .characters
            .iter()
            .find(|c| c.get("name").and_then(Value::as_str) == name.as_deref());
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

    async fn setup_call(
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
            let current = self
                .in_world_as
                .clone()
                .or_else(|| self.req.lab_character.clone());
            if current.is_some() && current == want {
                return Ok(());
            }
            self.setup_call("lab_logout", json!({}), ctx).await?;
        }
        let name =
            want.ok_or("no character: set lab-account.json `character` or pass `character`")?;
        let mut created = false;
        if spec.section.character == "fresh" && !self.fresh.contains_key(&spec.section.id) {
            let f = spec
                .section
                .fresh
                .clone()
                .ok_or("character = fresh without [section.fresh]")?;
            let protect: Vec<String> = self.req.lab_character.iter().cloned().collect();
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
            self.fresh.insert(spec.section.id.clone(), name.clone());
            created = true;
        }
        self.setup_call("lab_play_character", json!({ "name": name }), ctx)
            .await?;
        self.in_world_as = Some(name);
        if created {
            // A new character lands in the intro dialog; finish it like a
            // player (Next to Done). Best effort.
            let _ = self.setup_call("lab_finish_dialog", json!({}), ctx).await;
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
