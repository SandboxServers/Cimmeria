//! Character select: list, create, delete, and keep a free slot.

use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};

use super::widgets::{
    self, Alignment, CHAR_CREATE_BUTTON, CHAR_CREATE_FIRST, CHAR_CREATE_LAST, CHAR_CREATE_WIN,
    CHAR_SELECT_CREATE, CHAR_SELECT_DELETE, CHAR_SELECT_WIN, MAX_CHARACTERS, PROMPT_INSTANCES,
    PROMPT_TEXT,
};
use super::{settle, FlowError, FlowRun};
use crate::supervisor::Supervisor;

const CREATE_TIMEOUT: Duration = Duration::from_secs(30);
const DELETE_TIMEOUT: Duration = Duration::from_secs(15);

/// One character-select slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterRow {
    /// 1-based slot, as `getCharacterInfo` and the slot widgets number it.
    pub index: usize,
    pub name: String,
    pub level: Option<i64>,
    pub alignment: String,
    pub archetype: String,
    pub playable: bool,
}

/// Lua that lists the characters as tab-separated rows, after an `ok`
/// marker (or `not_at_select` when the screen is not up).
pub fn list_chunk() -> String {
    format!(
        "if not {vis} then return 'not_at_select' end \
         local out = {{'ok'}} \
         for i = 1, getCharacterCount() do \
           local c = getCharacterInfo(i) \
           local al = CharSelectMod and CharSelectMod.alignmentNames[c.alignment] or c.alignment \
           local ar = CharSelectMod and CharSelectMod.archetypeNames[c.archetype] or c.archetype \
           out[#out + 1] = table.concat({{i, tostring(c.name), tostring(c.level), tostring(al), \
             tostring(ar), tostring(c.playable)}}, '\\t') \
         end return unpack(out)",
        vis = widgets::visible(CHAR_SELECT_WIN)
    )
}

/// Parse [`list_chunk`]'s results.
pub fn parse_characters(results: &[String]) -> Result<Vec<CharacterRow>, String> {
    match results.first().map(String::as_str) {
        Some("ok") => {}
        Some("not_at_select") => return Err("the client is not at character select".into()),
        other => return Err(format!("character list: unexpected results {other:?}")),
    }
    results[1..]
        .iter()
        .map(|row| {
            let f: Vec<&str> = row.split('\t').collect();
            if f.len() != 6 {
                return Err(format!("character row {row:?} has {} fields", f.len()));
            }
            Ok(CharacterRow {
                index: f[0]
                    .parse()
                    .map_err(|e| format!("character index {:?}: {e}", f[0]))?,
                name: f[1].to_string(),
                level: f[2].parse().ok(),
                alignment: f[3].to_string(),
                archetype: f[4].to_string(),
                playable: f[5].parse::<i64>().map(|p| p > 0).unwrap_or(false),
            })
        })
        .collect()
}

/// Which characters to delete so at least `free_slots` of the client's
/// eight slots are open: oldest (lowest slot) first, never a protected
/// name. Errors when the protected set leaves too few to delete.
pub fn plan_deletions(
    names: &[String],
    free_slots: usize,
    protect: &[String],
) -> Result<Vec<String>, String> {
    let free_slots = free_slots.clamp(1, MAX_CHARACTERS);
    let allowed = MAX_CHARACTERS - free_slots;
    let excess = names.len().saturating_sub(allowed);
    let deletable: Vec<&String> = names
        .iter()
        .filter(|n| !protect.iter().any(|p| p.eq_ignore_ascii_case(n)))
        .collect();
    if deletable.len() < excess {
        return Err(format!(
            "need to delete {excess} characters to free {free_slots} slots, but only {} are \
             unprotected ({deletable:?})",
            deletable.len()
        ));
    }
    Ok(deletable.into_iter().take(excess).cloned().collect())
}

/// Find a character's slot by exact name.
pub fn slot_of(rows: &[CharacterRow], name: &str) -> Result<usize, String> {
    rows.iter()
        .find(|r| r.name == name)
        .map(|r| r.index)
        .ok_or_else(|| {
            let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
            format!("no character named {name:?}; the list has {names:?}")
        })
}

/// Lua returning the client's selected slot and the name it shows.
const SELECTION_CHUNK: &str = "return tostring(CharSelectMod.selectedCharacterIndex), \
     tostring(CharSelect_NameText:getText())";

/// Lua returning `N` for the first visible `Prompt<N>_Button1`, and the
/// prompt message, or nothing.
fn visible_prompt_chunk() -> String {
    format!(
        "for i = 1, {PROMPT_INSTANCES} do local b = _G['Prompt' .. i .. '_Button1'] \
           if b ~= nil and b:isVisible() then \
             local m = _G['Prompt' .. i .. '_Message'] \
             return tostring(i), tostring(m and m:getText() or '') end end \
         return 'none'"
    )
}

/// `create` arguments.
#[derive(Debug, Clone)]
pub struct CreateRequest {
    pub first: String,
    pub last: String,
    pub alignment: String,
    pub archetype: String,
    pub gender: String,
}

impl Supervisor {
    /// The character list (errors when not at character select).
    pub async fn read_characters(
        &self,
        run: &mut FlowRun<'_>,
    ) -> Result<Vec<CharacterRow>, FlowError> {
        let r = run.lua("list_characters", &list_chunk()).await?;
        match parse_characters(&r) {
            Ok(rows) => Ok(rows),
            Err(e) => Err(run.fail_with_state("list_characters", e).await),
        }
    }

    /// `lab_characters`.
    pub async fn characters_flow(&self) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_characters");
        let rows = self.read_characters(&mut run).await?;
        Ok(run.finish(json!({ "count": rows.len(), "cap": MAX_CHARACTERS, "characters": rows })))
    }

    /// Click a character's slot and prove the client selected it: the
    /// delete and play buttons act on the *selected* slot, so a missed
    /// click must never fall through to another character.
    pub(super) async fn select_character(
        &self,
        run: &mut FlowRun<'_>,
        name: &str,
    ) -> Result<CharacterRow, FlowError> {
        let rows = self.read_characters(run).await?;
        let index = slot_of(&rows, name).map_err(|e| run.fail("find_character", e))?;
        let container = widgets::char_container(index).map_err(|e| run.fail("select", e))?;
        run.click("select", &container).await?;
        settle(500).await;
        let sel = run.lua("verify_selection", SELECTION_CHUNK).await?;
        let (got_index, got_name) = (
            sel.first().cloned().unwrap_or_default(),
            sel.get(1).cloned().unwrap_or_default(),
        );
        if got_index != index.to_string() || got_name != name {
            return Err(run
                .fail_with_state(
                    "verify_selection",
                    format!(
                        "clicked {container} for {name:?} but the client selected slot \
                         {got_index} ({got_name:?})"
                    ),
                )
                .await);
        }
        Ok(rows
            .into_iter()
            .find(|r| r.index == index)
            .expect("slot_of found it"))
    }

    /// `lab_create_character`.
    pub async fn create_character_flow(&self, req: CreateRequest) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_create_character");
        let alignment = Alignment::parse(&req.alignment).map_err(|e| run.fail("args", e))?;
        let archetype_btn = alignment
            .archetype_button(&req.archetype)
            .map_err(|e| run.fail("args", e))?;
        let gender_btn = widgets::gender_button(&req.gender).map_err(|e| run.fail("args", e))?;
        for (what, v) in [("first", &req.first), ("last", &req.last)] {
            if v.is_empty() || !v.chars().all(|c| c.is_ascii_alphabetic()) {
                return Err(run.fail("args", format!("{what} name {v:?} must be letters only")));
            }
        }
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;

        let rows = self.read_characters(&mut run).await?;
        if rows.len() >= MAX_CHARACTERS {
            return Err(run.fail(
                "slots",
                format!("all {MAX_CHARACTERS} slots are used; run lab_ensure_character_slot first"),
            ));
        }
        if rows.iter().any(|r| r.name == req.last) {
            return Err(run.fail("args", format!("a character named {:?} exists", req.last)));
        }

        run.click("open_create", CHAR_SELECT_CREATE).await?;
        run.wait(
            "create_screen",
            &widgets::visible(CHAR_CREATE_WIN),
            Some(PROMPT_TEXT),
            CREATE_TIMEOUT,
        )
        .await?;
        settle(500).await;
        run.click("alignment", &alignment.button()).await?;
        settle(500).await;
        run.click("archetype", &archetype_btn).await?;
        settle(500).await;
        if widgets::archetype_has_gender(&req.archetype) {
            run.click("gender", gender_btn).await?;
            settle(500).await;
        }
        run.type_into("first_name", CHAR_CREATE_FIRST, &req.first, false)
            .await?;
        run.type_into("last_name", CHAR_CREATE_LAST, &req.last, false)
            .await?;
        run.click("create", CHAR_CREATE_BUTTON).await?;
        // CharCreateMod.onCreateFailed raises a prompt with the reason.
        let back = format!(
            "not {} and {}",
            widgets::visible(CHAR_CREATE_WIN),
            widgets::visible(CHAR_SELECT_WIN)
        );
        run.wait("created", &back, Some(PROMPT_TEXT), CREATE_TIMEOUT)
            .await?;
        settle(1000).await;
        let rows = self.read_characters(&mut run).await?;
        let Some(created) = rows.iter().find(|r| r.name == req.last).cloned() else {
            return Err(run
                .fail_with_state(
                    "verify_created",
                    format!("back at character select but no {:?} in the list", req.last),
                )
                .await);
        };
        Ok(run.finish(json!({ "created": created, "characters": rows })))
    }

    /// `lab_delete_character`.
    pub async fn delete_character_flow(&self, name: &str) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_delete_character");
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;
        self.delete_one(&mut run, name).await?;
        let rows = self.read_characters(&mut run).await?;
        Ok(run.finish(json!({ "deleted": name, "characters": rows })))
    }

    async fn delete_one(&self, run: &mut FlowRun<'_>, name: &str) -> Result<(), FlowError> {
        self.select_character(run, name).await?;
        run.click("delete", CHAR_SELECT_DELETE).await?;
        // Wait for the confirmation prompt and prove it names this
        // character before pressing Yes.
        let t0 = Instant::now();
        let prompt = loop {
            let r = run.lua("confirm_prompt", &visible_prompt_chunk()).await?;
            if r.first().map(String::as_str) != Some("none") && !r.is_empty() {
                break r;
            }
            if t0.elapsed() > Duration::from_secs(10) {
                return Err(run
                    .fail_with_state("confirm_prompt", "no delete confirmation prompt appeared")
                    .await);
            }
            settle(300).await;
        };
        let n: u32 = prompt[0].parse().unwrap_or(0);
        let message = prompt.get(1).cloned().unwrap_or_default();
        if !message.contains(name) {
            return Err(run
                .fail_with_state(
                    "confirm_prompt",
                    format!("the prompt does not name {name:?}: {message:?}; not confirming"),
                )
                .await);
        }
        run.record("confirm_prompt", t0, json!({ "prompt": n }));
        run.click("confirm", &widgets::prompt_button1(n)).await?;
        let gone = format!(
            "(function() for i = 1, getCharacterCount() do \
               if getCharacterInfo(i).name == {q} then return false end end \
             return true end)()",
            q = widgets::lua_quote(name)
        );
        run.wait("deleted", &gone, None, DELETE_TIMEOUT).await?;
        settle(500).await;
        Ok(())
    }

    /// `lab_ensure_character_slot` — delete the oldest unprotected
    /// characters until `free_slots` slots are open.
    pub async fn ensure_slot_flow(
        &self,
        free_slots: usize,
        protect: Vec<String>,
    ) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_ensure_character_slot");
        let mut protect = protect;
        // The lab account's own character is always protected.
        if let Some(c) = self
            .config
            .install_dir
            .as_ref()
            .and_then(|d| crate::supervisor::session_file::read_lab_account(d).ok())
            .map(|a| a.character)
            .filter(|c| !c.is_empty())
        {
            protect.push(c);
        }
        let rows = self.read_characters(&mut run).await?;
        let names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();
        let plan = plan_deletions(&names, free_slots, &protect).map_err(|e| run.fail("plan", e))?;
        if !plan.is_empty() {
            self.input_focus(true)
                .await
                .map_err(|e| run.fail("focus", e))?;
        }
        for name in &plan {
            self.delete_one(&mut run, name).await?;
        }
        let rows = self.read_characters(&mut run).await?;
        Ok(run.finish(json!({
            "deleted": plan,
            "protected": protect,
            "free_slots": MAX_CHARACTERS.saturating_sub(rows.len()),
            "characters": rows,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn character_rows_parse() {
        let rows = parse_characters(&s(&[
            "ok",
            "1\tLabone\t5\tPraxis\tSoldier\t1",
            "2\tFrostab\t1\tPraxis\tSoldier\t0",
        ]))
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Labone");
        assert_eq!(rows[0].level, Some(5));
        assert!(rows[0].playable);
        assert!(!rows[1].playable);
        assert_eq!(slot_of(&rows, "Frostab").unwrap(), 2);
        assert!(slot_of(&rows, "Nobody").unwrap_err().contains("Labone"));
    }

    #[test]
    fn character_list_off_the_select_screen_is_an_error() {
        assert!(parse_characters(&s(&["not_at_select"]))
            .unwrap_err()
            .contains("not at character select"));
        assert!(parse_characters(&s(&["ok", "1\tbroken"])).is_err());
        assert!(parse_characters(&s(&["ok"])).unwrap().is_empty());
    }

    fn names(n: &[&str]) -> Vec<String> {
        s(n)
    }

    #[test]
    fn deletions_take_the_oldest_unprotected_first() {
        let list = names(&["Labone", "A", "B", "C", "D", "E", "F", "G"]);
        // 8 of 8 used, one slot wanted: delete one, skipping Labone.
        assert_eq!(
            plan_deletions(&list, 1, &s(&["labone"])).unwrap(),
            names(&["A"])
        );
        // Two slots wanted: delete two.
        assert_eq!(
            plan_deletions(&list, 2, &s(&["Labone"])).unwrap(),
            names(&["A", "B"])
        );
    }

    #[test]
    fn no_deletions_when_there_is_room() {
        let list = names(&["Labone", "A"]);
        assert!(plan_deletions(&list, 1, &[]).unwrap().is_empty());
    }

    #[test]
    fn protection_that_blocks_the_plan_is_an_error() {
        let list = names(&["A", "B", "C", "D", "E", "F", "G", "H"]);
        let all = list.clone();
        assert!(plan_deletions(&list, 1, &all)
            .unwrap_err()
            .contains("unprotected"));
    }

    /// free_slots 0 would let the list fill up and block creation: it is
    /// raised to 1.
    #[test]
    fn free_slots_is_at_least_one() {
        let list = names(&["A", "B", "C", "D", "E", "F", "G", "H"]);
        assert_eq!(plan_deletions(&list, 0, &[]).unwrap(), names(&["A"]));
    }

    #[test]
    fn list_chunk_guards_the_screen() {
        let c = list_chunk();
        assert!(c.starts_with("if not (_G[\"CharSelectWin\"] ~= nil"));
        assert!(c.contains("getCharacterInfo(i)"));
    }
}
