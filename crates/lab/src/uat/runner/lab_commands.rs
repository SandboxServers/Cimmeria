//! Running the ability lab commands (`@dummy`, `@cooldowns_reset`,
//! `@clear_effects`; the lines are built in `crate::uat::lab_commands`):
//! an optional real-input target, the typed line, then the command's own
//! feedback line. No reply, or a refusal, fails the action, so a teardown
//! that never cleared anything is flagged instead of trusted.

use std::time::{Duration, Instant};

use serde_json::json;

use super::actions::{captured_value, fail};
use super::players::Who;
use super::{RowCtx, Runner};
use crate::uat::clause::new_lines;
use crate::uat::evidence::ActionRecord;
use crate::uat::invoke::ToolInvoker;
use crate::uat::lab_commands::{LabCommand, ERROR_REPLY};

/// How long a command's feedback line may take (the `.bug` anchor allows
/// the same).
const REPLY_WAIT: Duration = Duration::from_secs(5);
const REPLY_POLL: Duration = Duration::from_millis(400);

impl<I: ToolInvoker> Runner<'_, I> {
    pub(crate) async fn exec_lab_command(
        &mut self,
        cmd: LabCommand,
        who: Who,
        ctx: &mut RowCtx,
        rec: &mut ActionRecord,
    ) {
        if let Some(name) = &cmd.target_name {
            let args = json!({ "name": name });
            let out = self.on(who).call("client_target", args.clone()).await;
            rec.calls.push(json!({
                "tool": "client_target", "args": args, "ok": out.ok, "error": out.error,
            }));
            if !out.ok {
                // Typing now would act on whatever was selected before.
                return fail(
                    rec,
                    format!(
                        "client_target {name}: {} (nothing typed)",
                        out.error.unwrap_or_default()
                    ),
                );
            }
        }
        let before = self.read_chat_of(who).await.unwrap_or_default();
        self.send_chat(who, &cmd.line, rec).await;
        if !rec.ok {
            return;
        }
        let ok_re = regex::Regex::new(cmd.reply).expect("static reply regex");
        let err_re = regex::Regex::new(ERROR_REPLY).expect("static error regex");
        let wait = if self.req.no_settle {
            Duration::ZERO
        } else {
            REPLY_WAIT
        };
        let t0 = Instant::now();
        loop {
            let after = self.read_chat_of(who).await.unwrap_or_default();
            let (lines, _) = new_lines(&before, &after);
            if let Some(l) = lines.iter().find(|l| err_re.is_match(l)) {
                rec.result = json!({ "line": cmd.line, "reply": l });
                return fail(rec, format!("{} refused: {l}", cmd.line));
            }
            if let Some(l) = lines.iter().find(|l| ok_re.is_match(l)) {
                let mut result = json!({ "line": cmd.line, "reply": l });
                let group = ok_re
                    .captures(l)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string());
                if let (Some(var), Some(g)) = (cmd.capture, group) {
                    // Ids are stored as numbers, so `entity_id = "${dummy_id}"`
                    // reaches a server tool as one.
                    let v = captured_value(&g);
                    ctx.vars.insert(var.into(), v.clone());
                    result["captured"] = json!({ var: v });
                }
                rec.result = result;
                return;
            }
            if t0.elapsed() >= wait {
                rec.result = json!({ "line": cmd.line, "new_lines": lines });
                return fail(
                    rec,
                    format!(
                        "no reply to {} within {} ms (is the account a GM?)",
                        cmd.line,
                        wait.as_millis()
                    ),
                );
            }
            tokio::time::sleep(REPLY_POLL).await;
        }
    }
}
