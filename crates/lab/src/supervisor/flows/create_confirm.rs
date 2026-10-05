//! `lab_create_character`'s last step: prove the character was made.
//!
//! Right after Create, the client is busy (the server's answer, then the
//! new character's preview model), and a bridge call can sit in the
//! client's main-thread queue past the bridge's 5 s response limit. The
//! first live run (colo, 2026-10-04) failed here with "bridge error
//! -32603: dispatch timeout" after 9.9 s, although the character was
//! created and showed at character select. So a busy client is not a
//! failure: a timed-out poll is retried until the step's budget runs out,
//! and once any poll has timed out, the character list is read too, so a
//! screen that settled while the bridge was busy is still recognised.

use std::time::{Duration, Instant};

use serde_json::json;

use super::characters::{list_chunk, parse_characters, CharacterRow};
use super::ui_state::{parse_poll, poll_chunk};
use super::widgets::{self, CHAR_CREATE_WIN, CHAR_SELECT_WIN, PROMPT_TEXT};
use super::{settle, FlowError, FlowRun, POLL};
use crate::supervisor::Supervisor;

/// How long the list may lag the screen: once character select is back,
/// the new name must show within this.
const LIST_LAG: Duration = Duration::from_secs(5);

/// A bridge error that means the client's main thread was busy, not that
/// the bridge or the client is gone: the call timed out in the dispatch
/// queue, or the queue was full.
pub fn transient_bridge_error(e: &str) -> bool {
    e.contains("dispatch timeout") || e.contains("dispatch queue full")
}

/// The condition for "the create screen closed, character select is up".
fn back_at_select() -> String {
    format!(
        "not {} and {}",
        widgets::visible(CHAR_CREATE_WIN),
        widgets::visible(CHAR_SELECT_WIN)
    )
}

impl Supervisor {
    /// Wait for character select to come back with `name` in the list.
    /// A prompt (`CharCreateMod.onCreateFailed` raises one with the
    /// reason) ends the wait as the client's own error.
    pub(super) async fn confirm_created(
        &self,
        run: &mut FlowRun<'_>,
        name: &str,
        timeout: Duration,
    ) -> Result<(CharacterRow, Vec<CharacterRow>), FlowError> {
        let t0 = Instant::now();
        let poll = poll_chunk(&back_at_select(), Some(PROMPT_TEXT));
        let list = list_chunk();
        let mut busy = 0u32;
        let mut last_busy: Option<String> = None;
        let mut at_select: Option<Instant> = None;
        let mut last_rows: Vec<CharacterRow> = Vec::new();
        loop {
            if at_select.is_none() {
                match self.lua_results(&poll).await.and_then(|r| parse_poll(&r)) {
                    Ok(p) if p.met => {
                        run.record("created", t0, json!({ "busy_retries": busy }));
                        // A frame or two for the list to take the new row.
                        settle(1000).await;
                        at_select = Some(Instant::now());
                    }
                    Ok(p) => {
                        if let Some(f) = p.fail {
                            return Err(run
                                .fail_with_state("created", format!("the client reported: {f}"))
                                .await);
                        }
                    }
                    Err(e) if transient_bridge_error(&e) => {
                        busy += 1;
                        last_busy = Some(e);
                    }
                    Err(e) => return Err(run.fail("created", e)),
                }
            }
            // At select, or unsure because the bridge was busy: the list
            // decides. Off the select screen it reads `not_at_select`.
            if at_select.is_some() || busy > 0 {
                match self.lua_results(&list).await {
                    Ok(r) => {
                        if let Ok(rows) = parse_characters(&r) {
                            if let Some(c) = rows.iter().find(|r| r.name == name).cloned() {
                                run.record(
                                    "verify_created",
                                    t0,
                                    json!({ "busy_retries": busy, "slot": c.index }),
                                );
                                return Ok((c, rows));
                            }
                            last_rows = rows;
                        }
                    }
                    Err(e) if transient_bridge_error(&e) => {
                        busy += 1;
                        last_busy = Some(e);
                    }
                    Err(e) => return Err(run.fail("verify_created", e)),
                }
            }
            let list_late = at_select.is_some_and(|s| s.elapsed() >= LIST_LAG);
            if list_late || t0.elapsed() >= timeout {
                let busy_note = last_busy
                    .map(|e| {
                        format!("; {busy} bridge calls timed out on a busy client (last: {e})")
                    })
                    .unwrap_or_default();
                return Err(match at_select {
                    Some(_) => {
                        let names: Vec<&str> = last_rows.iter().map(|r| r.name.as_str()).collect();
                        run.fail_with_state(
                            "verify_created",
                            format!(
                                "back at character select but no {name:?} in the list \
                                 {names:?}{busy_note}"
                            ),
                        )
                        .await
                    }
                    None => {
                        run.fail_with_state(
                            "created",
                            format!(
                                "character select did not come back with {name:?} within {} ms\
                                 {busy_note}",
                                timeout.as_millis()
                            ),
                        )
                        .await
                    }
                });
            }
            settle(POLL.as_millis() as u64).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};

    const DISPATCH_TIMEOUT: &str = "dispatch timeout";

    #[test]
    fn only_a_busy_client_is_transient() {
        assert!(transient_bridge_error(
            "bridge: bridge error -32603: dispatch timeout"
        ));
        assert!(transient_bridge_error(
            "bridge: bridge error -32001: bridge dispatch queue full"
        ));
        assert!(!transient_bridge_error("bridge: connection refused"));
        assert!(!transient_bridge_error("bridge error -32602: bad params"));
    }

    /// A client that answers the create-screen poll and the list read from
    /// two scripts: each call takes the next answer, the last one repeats.
    fn fake(
        poll: Vec<Result<&'static str, &'static str>>,
        list: Vec<Result<Vec<&'static str>, &'static str>>,
    ) -> (Responder, Arc<Mutex<(usize, usize)>>) {
        let calls = Arc::new(Mutex::new((0usize, 0usize)));
        let c2 = calls.clone();
        let r: Responder = Arc::new(move |method, params| {
            let chunk = params["chunk"].as_str().unwrap_or_default();
            assert_eq!(method, "lua_eval");
            let mut n = c2.lock().unwrap();
            if chunk.contains("local okc, r = pcall") {
                let i = n.0.min(poll.len() - 1);
                n.0 += 1;
                match poll[i] {
                    Ok("met") => Ok(lua_ok(&["true", "", ""])),
                    Ok("waiting") => Ok(lua_ok(&["false", "", ""])),
                    Ok(prompt) => Ok(lua_ok(&["false", "", prompt])),
                    Err(e) => Err(e.to_string()),
                }
            } else if chunk.contains("getCharacterCount()") && chunk.contains("not_at_select") {
                let i = n.1.min(list.len() - 1);
                n.1 += 1;
                match &list[i] {
                    Ok(rows) => Ok(lua_ok(rows)),
                    Err(e) => Err(e.to_string()),
                }
            } else {
                // The failure's screen summary.
                Ok(lua_ok(&[""]))
            }
        });
        (r, calls)
    }

    const WITH_NEW: [&str; 3] = [
        "ok",
        "1\tLabone\t5\tPraxis\tSoldier\t1",
        "2\tFrostab\t1\tPraxis\tSoldier\t1",
    ];

    async fn confirm(
        r: Responder,
        timeout: Duration,
    ) -> Result<(CharacterRow, Vec<CharacterRow>), FlowError> {
        let sup = fake_bridge::supervisor(r).await;
        let mut run = FlowRun::new(&sup, "lab_create_character");
        sup.confirm_created(&mut run, "Frostab", timeout).await
    }

    /// The live failure: the polls right after Create time out on the busy
    /// client, then the screen settles. Not a failure.
    #[tokio::test]
    async fn dispatch_timeouts_after_create_are_retried() {
        let (r, calls) = fake(
            vec![Err(DISPATCH_TIMEOUT), Err(DISPATCH_TIMEOUT), Ok("met")],
            vec![
                Ok(vec!["not_at_select"]),
                Ok(vec!["not_at_select"]),
                Err(DISPATCH_TIMEOUT),
                Ok(WITH_NEW.to_vec()),
            ],
        );
        let (c, rows) = confirm(r, Duration::from_secs(20))
            .await
            .map_err(|e| e.summary())
            .unwrap();
        assert_eq!(c.name, "Frostab");
        assert_eq!(c.index, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            calls.lock().unwrap().0,
            3,
            "polled until the screen settled"
        );
    }

    /// The poll never gets an answer, but the list (read because the
    /// bridge was busy) already has the character: created.
    #[tokio::test]
    async fn a_busy_poll_falls_back_to_the_character_list() {
        let (r, _) = fake(vec![Err(DISPATCH_TIMEOUT)], vec![Ok(WITH_NEW.to_vec())]);
        let (c, _) = confirm(r, Duration::from_secs(5))
            .await
            .map_err(|e| e.summary())
            .unwrap();
        assert_eq!(c.name, "Frostab");
    }

    #[tokio::test]
    async fn a_prompt_is_the_clients_own_error() {
        let (r, _) = fake(
            vec![Ok("waiting"), Ok("Error: That name is taken")],
            vec![Ok(vec!["not_at_select"])],
        );
        let e = confirm(r, Duration::from_secs(5)).await.unwrap_err();
        assert_eq!(e.step, "created");
        assert!(e.message.contains("That name is taken"), "{}", e.message);
    }

    #[tokio::test]
    async fn a_broken_bridge_still_fails_at_once() {
        let (r, calls) = fake(vec![Err("connection reset")], vec![Ok(WITH_NEW.to_vec())]);
        let e = confirm(r, Duration::from_secs(5)).await.unwrap_err();
        assert_eq!(e.step, "created");
        assert_eq!(
            calls.lock().unwrap().1,
            0,
            "no list read after a hard error"
        );
    }

    #[tokio::test]
    async fn a_busy_client_that_never_settles_times_out_naming_the_retries() {
        let (r, _) = fake(vec![Err(DISPATCH_TIMEOUT)], vec![Err(DISPATCH_TIMEOUT)]);
        let e = confirm(r, Duration::from_millis(1200)).await.unwrap_err();
        assert_eq!(e.step, "created");
        assert!(
            e.message.contains("bridge calls timed out"),
            "{}",
            e.message
        );
    }

    #[tokio::test]
    async fn back_at_select_without_the_name_is_verify_created() {
        let (r, _) = fake(
            vec![Ok("met")],
            vec![Ok(vec!["ok", "1\tLabone\t5\tPraxis\tSoldier\t1"])],
        );
        let e = confirm(r, Duration::from_secs(30)).await.unwrap_err();
        assert_eq!(e.step, "verify_created");
        assert!(e.message.contains("Labone"), "{}", e.message);
    }
}
