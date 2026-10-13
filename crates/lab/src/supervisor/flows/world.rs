//! `lab_play_character` and `lab_finish_dialog`.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::widgets::{
    self, CHAR_SELECT_PLAY, CHAR_SELECT_WIN, DIALOG_ACCEPT, DIALOG_CLOSE, DIALOG_DONE, DIALOG_NEXT,
    DIALOG_WIN, MOVIE_WIN,
};
use super::{settle, FlowError, FlowRun};
use crate::supervisor::{LoginState, Supervisor};

/// Default budget from Play to the world HUD (map load + cutscene).
pub const DEFAULT_PLAY_TIMEOUT: Duration = Duration::from_secs(120);
/// Dialog pages `lab_finish_dialog` will turn before giving up.
pub const DEFAULT_MAX_PAGES: u32 = 12;
/// What one poll after Play saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayProbe {
    pub char_select: bool,
    /// The world HUD (`SelfStatusWin`) is visible.
    pub world_up: bool,
    pub dialog: bool,
    pub movie: bool,
}

pub fn play_probe_chunk() -> String {
    let p = |e: String| format!("tostring(select(2, pcall(function() return {e} end)) == true)");
    format!(
        "return {}, {}, {}, {}",
        p(widgets::visible(CHAR_SELECT_WIN)),
        p(widgets::world_up()),
        p(widgets::visible(DIALOG_WIN)),
        p(widgets::visible(MOVIE_WIN))
    )
}

pub fn parse_play_probe(r: &[String]) -> PlayProbe {
    let v = |i: usize| r.get(i).map(String::as_str) == Some("true");
    PlayProbe {
        char_select: v(0),
        world_up: v(1),
        dialog: v(2),
        movie: v(3),
    }
}

/// What to do after one play poll, when skipping cutscenes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayStep {
    /// Still on character select or loading: wait.
    Wait,
    /// Loading/cutscene with no HUD yet, or a movie playing: press Escape.
    Escape,
    /// The world is up.
    Done,
}

/// The skip policy. Escape while a movie plays, and every poll from
/// character select going away until the HUD (`SelfStatusWin`) shows: the
/// map load and a new character's arrival cutscene, which plays on under
/// the intro dialog (an open dialog or a visible minimap is not the HUD;
/// see [`widgets::world_up`]). Once the HUD is up an Escape would open the
/// game menu, so it stops.
pub fn plan_play_step(p: PlayProbe, skip_cutscene: bool) -> PlayStep {
    if p.movie && skip_cutscene {
        return PlayStep::Escape;
    }
    if p.char_select {
        return PlayStep::Wait;
    }
    if p.world_up {
        return PlayStep::Done;
    }
    if skip_cutscene {
        PlayStep::Escape
    } else {
        PlayStep::Wait
    }
}

/// Which dialog button to press next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogAction {
    Closed,
    Done,
    Accept,
    Next,
    Stuck,
}

#[async_trait::async_trait]
trait DialogDriver {
    async fn read(&mut self) -> Result<Vec<String>, FlowError>;
    async fn click(&mut self, step: &str, window: &str) -> Result<(), FlowError>;
    async fn visible(&mut self, step: &str) -> Result<bool, FlowError>;
    async fn fail(&mut self, step: &str, message: String) -> FlowError;
    async fn stuck(&mut self, accept: bool) -> FlowError;
}

#[async_trait::async_trait]
impl DialogDriver for FlowRun<'_> {
    async fn read(&mut self) -> Result<Vec<String>, FlowError> {
        self.lua("read_dialog", &dialog_probe_chunk()).await
    }

    async fn click(&mut self, step: &str, window: &str) -> Result<(), FlowError> {
        FlowRun::click(self, step, window).await
    }

    async fn visible(&mut self, step: &str) -> Result<bool, FlowError> {
        FlowRun::visible(self, step, DIALOG_WIN).await
    }

    async fn fail(&mut self, step: &str, message: String) -> FlowError {
        self.fail_with_state(step, message).await
    }

    async fn stuck(&mut self, accept: bool) -> FlowError {
        let ui = self.sup.ui_state(Some(0)).await.ok();
        let buttons = ui.map(|u| u["dialog"]["buttons"].clone());
        FlowRun::fail(
            self,
            "read_dialog",
            format!(
                "the dialog shows neither Done nor Next{}; buttons: {}",
                if accept { " nor Accept" } else { "" },
                buttons.unwrap_or(Value::Null)
            ),
        )
    }
}

/// Drive the complete dialog sequence through a replaceable client boundary.
/// The result is only successful after the window is observed closed.
async fn finish_dialog_steps<D: DialogDriver + Send>(
    driver: &mut D,
    accept: bool,
    max_pages: u32,
) -> Result<Option<(String, u32)>, FlowError> {
    let mut pages = 0u32;
    let mut title = String::new();
    loop {
        let r = driver.read().await?;
        if title.is_empty() {
            title = r.get(4).cloned().unwrap_or_default();
        }
        match plan_dialog(&r, accept) {
            DialogAction::Closed if pages == 0 => return Ok(None),
            DialogAction::Closed => break,
            DialogAction::Done => {
                driver.click("done", DIALOG_DONE).await?;
                pages += 1;
                break;
            }
            DialogAction::Accept => {
                driver.click("accept", DIALOG_ACCEPT).await?;
                pages += 1;
                break;
            }
            DialogAction::Next => {
                if pages >= max_pages {
                    return Err(driver
                        .fail(
                            "next",
                            format!("still paging after {max_pages} pages; no Done button"),
                        )
                        .await);
                }
                driver.click("next", DIALOG_NEXT).await?;
                pages += 1;
            }
            DialogAction::Stuck => {
                return Err(driver.stuck(accept).await);
            }
        }
        settle(1000).await;
    }
    settle(1000).await;
    if driver.visible("after").await? {
        driver.click("native_close", DIALOG_CLOSE).await?;
        settle(500).await;
        if driver.visible("after_native_close").await? {
            return Err(driver
                .fail(
                    "verify_closed",
                    "DialogWin remained visible after Done/Accept and native close".to_string(),
                )
                .await);
        }
    }
    Ok(Some((title, pages)))
}

pub fn dialog_probe_chunk() -> String {
    let v = |w: &str| format!("tostring({})", widgets::visible(w));
    format!(
        "return {}, {}, {}, {}, tostring(Dialog_NameText and Dialog_NameText:getText() or '')",
        v(DIALOG_WIN),
        v(DIALOG_DONE),
        v(DIALOG_ACCEPT),
        v(DIALOG_NEXT)
    )
}

/// Done (the green checkmark) wins; Accept only when asked; otherwise page
/// with Next. Never the close X: it sends choice -1, not a finish.
pub fn plan_dialog(r: &[String], accept: bool) -> DialogAction {
    let v = |i: usize| r.get(i).map(String::as_str) == Some("true");
    if !v(0) {
        DialogAction::Closed
    } else if v(1) {
        DialogAction::Done
    } else if accept && v(2) {
        DialogAction::Accept
    } else if v(3) {
        DialogAction::Next
    } else {
        DialogAction::Stuck
    }
}

impl Supervisor {
    /// `lab_play_character` — select, Play, skip the arrival cutscene, and
    /// stop when the world HUD is up.
    pub async fn play_flow(
        &self,
        name: &str,
        skip_cutscene: bool,
        timeout: Duration,
    ) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_play_character");
        self.input_focus(true)
            .await
            .map_err(|e| run.fail("focus", e))?;
        let row = self.select_character(&mut run, name).await?;
        if !row.playable {
            return Err(run.fail("play", format!("{name:?} is not playable")));
        }
        run.click("play", CHAR_SELECT_PLAY).await?;

        let t0 = Instant::now();
        let mut escapes = 0u32;
        let mut left_select_ms = None;
        let probe = loop {
            settle(1500).await;
            let r = run.lua("enter_world", &play_probe_chunk()).await?;
            let p = parse_play_probe(&r);
            if !p.char_select && left_select_ms.is_none() {
                left_select_ms = Some(t0.elapsed().as_millis() as u64);
            }
            match plan_play_step(p, skip_cutscene) {
                PlayStep::Done => break p,
                PlayStep::Escape => {
                    if self.input_key("Escape", "tap", None).await.is_ok() {
                        escapes += 1;
                    }
                }
                PlayStep::Wait => {}
            }
            if t0.elapsed() > timeout {
                self.set_login_state(LoginState::Failed).await;
                return Err(run
                    .fail_with_state(
                        "enter_world",
                        format!(
                            "the world HUD (SelfStatusWin) never came up ({escapes} Escapes \
                             sent; last poll: dialog {}, movie {})",
                            p.dialog, p.movie
                        ),
                    )
                    .await);
            }
        };
        run.record(
            "enter_world",
            t0,
            json!({ "escapes": escapes, "left_character_select_ms": left_select_ms }),
        );
        self.set_login_state(LoginState::InWorld).await;
        let ui = self.ui_state(Some(0)).await.ok();
        Ok(run.finish(json!({
            "in_world": true,
            "hud_visible": probe.world_up,
            "character": row,
            "dialog_open": probe.dialog,
            "dialog": ui.as_ref().map(|u| u["dialog"].clone()),
        })))
    }

    /// `lab_finish_dialog` — page with Next until the green checkmark
    /// (Done) shows, then press it.
    pub async fn finish_dialog_flow(
        &self,
        accept: bool,
        max_pages: u32,
    ) -> Result<Value, FlowError> {
        let mut run = FlowRun::new(self, "lab_finish_dialog");
        let result = finish_dialog_steps(&mut run, accept, max_pages).await?;
        match result {
            None => Ok(run.finish(json!({ "finished": false, "reason": "no dialog open" }))),
            Some((title, pages)) => Ok(run.finish(json!({
                "finished": true,
                "title": title,
                "pages_clicked": pages,
                "dialog_still_open": false,
            }))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(char_select: bool, world_up: bool, dialog: bool, movie: bool) -> PlayProbe {
        PlayProbe {
            char_select,
            world_up,
            dialog,
            movie,
        }
    }

    #[test]
    fn play_waits_on_select_escapes_while_loading_and_stops_at_the_hud() {
        assert_eq!(
            plan_play_step(probe(true, false, false, false), true),
            PlayStep::Wait
        );
        assert_eq!(
            plan_play_step(probe(false, false, false, false), true),
            PlayStep::Escape
        );
        assert_eq!(
            plan_play_step(probe(false, true, true, false), true),
            PlayStep::Done
        );
        // A movie is always skipped when asked.
        assert_eq!(
            plan_play_step(probe(false, true, false, true), true),
            PlayStep::Escape
        );
    }

    /// The first live run (colo, 2026-10-04): a new character's intro
    /// dialog was up over the Bink arrival cutscene with no HUD, and the
    /// flow called it in-world. The HUD test is `SelfStatusWin` alone, and
    /// every poll without it presses Escape, dialog or not.
    #[test]
    fn in_world_needs_self_status_and_escapes_continue_under_the_dialog() {
        let c = play_probe_chunk();
        assert!(c.contains("SelfStatusWin"), "{c}");
        assert!(!c.contains("MinimapWin"), "the minimap is not the HUD: {c}");
        assert_eq!(widgets::world_up(), widgets::visible("SelfStatusWin"));

        // Dialog open, cutscene still on, no HUD: Escape, poll after poll.
        let under_cutscene = probe(false, false, true, false);
        for _ in 0..5 {
            assert_eq!(plan_play_step(under_cutscene, true), PlayStep::Escape);
        }
        // The Escape skipped the cutscene and the HUD showed: stop there,
        // with the dialog still open.
        assert_eq!(
            plan_play_step(probe(false, true, true, false), true),
            PlayStep::Done
        );
        // Without skip, wait for the cutscene to end on its own.
        assert_eq!(plan_play_step(under_cutscene, false), PlayStep::Wait);
    }

    /// With skip_cutscene off, the flow never presses Escape.
    #[test]
    fn no_escapes_without_skip() {
        for p in [
            probe(false, false, false, false),
            probe(false, false, false, true),
        ] {
            assert_eq!(plan_play_step(p, false), PlayStep::Wait);
        }
    }

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn dialog_prefers_done_then_accept_then_next() {
        assert_eq!(
            plan_dialog(&s(&["false", "false", "false", "false"]), false),
            DialogAction::Closed
        );
        assert_eq!(
            plan_dialog(&s(&["true", "true", "true", "true"]), true),
            DialogAction::Done
        );
        assert_eq!(
            plan_dialog(&s(&["true", "false", "true", "true"]), true),
            DialogAction::Accept
        );
        assert_eq!(
            plan_dialog(&s(&["true", "false", "true", "true"]), false),
            DialogAction::Next
        );
        assert_eq!(
            plan_dialog(&s(&["true", "false", "false", "false"]), false),
            DialogAction::Stuck
        );
    }

    struct FakeDialog {
        visible: Vec<bool>,
        clicks: Vec<String>,
    }

    #[async_trait::async_trait]
    impl DialogDriver for FakeDialog {
        async fn read(&mut self) -> Result<Vec<String>, FlowError> {
            Ok(s(&["true", "true", "false", "false", "Pistol"]))
        }

        async fn click(&mut self, _step: &str, window: &str) -> Result<(), FlowError> {
            self.clicks.push(window.to_string());
            Ok(())
        }

        async fn visible(&mut self, _step: &str) -> Result<bool, FlowError> {
            Ok(self.visible.remove(0))
        }

        async fn fail(&mut self, step: &str, message: String) -> FlowError {
            FlowError(Box::new(super::super::FlowFailure {
                flow: "lab_finish_dialog",
                step: step.to_string(),
                message,
                state: None,
                elapsed_ms: 0,
                steps: vec![],
            }))
        }

        async fn stuck(&mut self, accept: bool) -> FlowError {
            self.fail("read_dialog", format!("stuck; accept={accept}"))
                .await
        }
    }

    #[tokio::test]
    async fn done_no_effect_uses_native_close_and_verifies_it() {
        let mut closed = FakeDialog {
            visible: vec![true, false],
            clicks: vec![],
        };
        assert_eq!(
            finish_dialog_steps(&mut closed, false, 12).await.unwrap(),
            Some(("Pistol".to_string(), 1))
        );
        assert_eq!(closed.clicks, [DIALOG_DONE, DIALOG_CLOSE]);

        let mut stuck = FakeDialog {
            visible: vec![true, true],
            clicks: vec![],
        };
        let error = finish_dialog_steps(&mut stuck, false, 12)
            .await
            .unwrap_err();
        assert_eq!(error.step, "verify_closed");
        assert_eq!(stuck.clicks, [DIALOG_DONE, DIALOG_CLOSE]);
    }

    #[test]
    fn play_probe_parses_in_order() {
        let p = parse_play_probe(&s(&["false", "true", "true", "false"]));
        assert_eq!(p, probe(false, true, true, false));
    }
}
