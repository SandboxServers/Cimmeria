//! The login screen's server combo (C17).
//!
//! The client's `Login.lua` preselects the first hash-table entry of its
//! server list, which can be a dead server, and the combo is hidden when it
//! has one row or none. `lab_login` reads the rows, picks the one the login
//! asked for (or the only one), and reads the selection back before typing
//! credentials, so the client logs into the server the lab minted a token for.

use std::time::Instant;

use serde_json::json;

use super::login::ResolvedLogin;
use super::widgets;
use super::{FlowError, FlowRun};
use crate::supervisor::Supervisor;

/// The login screen's server-type combo box.
const LOGIN_SERVER_COMBO: &str = "Login_ServerTypeCombo";

/// What the login screen's server combo should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComboPick {
    /// Nothing to select; the login proceeds as before.
    Skip,
    /// Select this row (its own text, as the client shows it).
    Use(String),
    /// Several rows and no usable name: fail rather than take the preselect.
    Error(String),
}

/// Pure. `rows` are the combo's item texts in index order; `want` is the
/// resolved login's server name (request over `lab-account.json`).
pub fn decide_combo(rows: &[String], want: Option<&str>) -> ComboPick {
    match rows {
        [] => ComboPick::Skip,
        [only] if only == "no_combo" => ComboPick::Skip,
        [only] => ComboPick::Use(only.clone()),
        many => {
            let wanted = want.map(str::trim).filter(|n| !n.is_empty());
            if let Some(n) = wanted {
                if let Some(row) = many.iter().find(|r| r.eq_ignore_ascii_case(n)) {
                    return ComboPick::Use(row.clone());
                }
            }
            let names = many.join(", ");
            match wanted {
                Some(n) => ComboPick::Error(format!(
                    "login server '{n}' is not on the login screen; rows: {names}"
                )),
                None => ComboPick::Error(format!(
                    "several login servers and no server in the request or lab-account.json; \
                     rows: {names}"
                )),
            }
        }
    }
}

/// Lua returning the combo's item texts, one result string per row
/// (`run.lua` results), or the single string `"no_combo"`.
pub fn combo_rows_chunk() -> String {
    let c = LOGIN_SERVER_COMBO;
    format!(
        "local ok, rows = pcall(function() \
           local out = {{}} \
           for i = 0, {c}:getItemCount() - 1 do \
             out[#out + 1] = tostring({c}:getListboxItemFromIndex(i):getText()) \
           end \
           return out end) \
         if not ok then return 'no_combo' end \
         return table.unpack(rows)"
    )
}

/// Lua that selects the row whose text is `name` and returns the read-back
/// `getSelectedItem():getText()` (or `"none"`).
pub fn select_combo_chunk(name: &str) -> String {
    let c = LOGIN_SERVER_COMBO;
    format!(
        "local name = {n} \
         local item = nil \
         pcall(function() item = {c}:findItemWithText(name, nil) end) \
         if item == nil then pcall(function() \
           for i = 0, {c}:getItemCount() - 1 do \
             local it = {c}:getListboxItemFromIndex(i) \
             if tostring(it:getText()) == name then item = it return end \
           end end) end \
         if item == nil then return 'none' end \
         local ok, got = pcall(function() \
           {c}:clearAllSelections() \
           {c}:setItemSelectState(item, true) \
           {c}:setText(name) \
           local s = {c}:getSelectedItem() \
           return tostring(s and s:getText() or 'none') end) \
         if ok then return got end \
         return 'none'",
        n = widgets::lua_quote(name)
    )
}

impl Supervisor {
    /// Pick the login screen's server row for this login. `Ok(None)` when
    /// the combo has nothing to choose; otherwise the selected row's name,
    /// read back from the widget.
    pub(super) async fn pick_login_combo(
        &self,
        run: &mut FlowRun<'_>,
        creds: &ResolvedLogin,
    ) -> Result<Option<String>, FlowError> {
        let t0 = Instant::now();
        let rows = run.lua("login_combo_rows", &combo_rows_chunk()).await?;
        let name = match decide_combo(&rows, creds.server.as_deref()) {
            ComboPick::Skip => return Ok(None),
            ComboPick::Error(msg) => return Err(run.fail("login_combo", msg)),
            ComboPick::Use(name) => name,
        };
        let got = run
            .lua("login_combo", &select_combo_chunk(&name))
            .await?
            .first()
            .cloned()
            .unwrap_or_default();
        if got != name {
            return Err(run.fail(
                "login_combo",
                format!("login server combo: selected '{got}', wanted '{name}'"),
            ));
        }
        run.record(
            "login_combo",
            t0,
            json!({ "login_server": name, "rows": rows.len() }),
        );
        Ok(Some(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_rows_skips() {
        assert_eq!(decide_combo(&[], None), ComboPick::Skip);
        assert_eq!(decide_combo(&[], Some("Beta")), ComboPick::Skip);
        assert_eq!(
            decide_combo(&rows(&["no_combo"]), Some("Beta")),
            ComboPick::Skip
        );
    }

    #[test]
    fn one_row_is_used_whatever_the_name() {
        let one = rows(&["Alpha"]);
        assert_eq!(
            decide_combo(&one, Some("Beta")),
            ComboPick::Use("Alpha".into())
        );
        assert_eq!(decide_combo(&one, None), ComboPick::Use("Alpha".into()));
    }

    #[test]
    fn named_row_wins_case_insensitively() {
        assert_eq!(
            decide_combo(&rows(&["Alpha", "Beta"]), Some("beta")),
            ComboPick::Use("Beta".into())
        );
    }

    #[test]
    fn several_rows_and_a_miss_errors_with_rows() {
        match decide_combo(&rows(&["Alpha", "Beta"]), Some("Gamma")) {
            ComboPick::Error(msg) => {
                assert!(msg.contains("Gamma"), "{msg}");
                assert!(msg.contains("Alpha"), "{msg}");
                assert!(msg.contains("Beta"), "{msg}");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn several_rows_and_no_name_errors() {
        match decide_combo(&rows(&["Alpha", "Beta"]), None) {
            ComboPick::Error(msg) => {
                assert!(msg.contains("Alpha"), "{msg}");
                assert!(msg.contains("Beta"), "{msg}");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn select_chunk_quotes_the_name() {
        let name = r#"Odd "name" \ x"#;
        let chunk = select_combo_chunk(name);
        assert!(chunk.contains(&widgets::lua_quote(name)), "{chunk}");
        assert!(!chunk.contains(name), "raw name leaked into chunk: {chunk}");
    }

    #[test]
    fn select_chunk_reads_back() {
        let chunk = select_combo_chunk("Alpha");
        assert!(chunk.contains("setItemSelectState"), "{chunk}");
        assert!(chunk.contains("getSelectedItem"), "{chunk}");
        assert!(chunk.contains("pcall"), "{chunk}");
    }
}
