use super::*;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn chunk_wraps_both_handlers_and_reads_after_the_marks() {
    let c = pump_chunk(12, 34, 56, 50);
    assert!(c.contains("SCTMod.onUnitCombat = L.combat_wrapper"));
    assert!(c.contains("ChatMod.onMessageReceived = L.chat_wrapper"));
    // Re-subscribes by the stock name so a cached lookup is refreshed.
    assert!(c.contains(r#"SCTWin:subscribe(Events.UnitCombat, "SCTMod.onUnitCombat")"#));
    assert!(c.contains("e.seq > 12 and n < 50"));
    assert!(c.contains("e.seq > 34 and m < 50"));
    // The original is always called after recording.
    assert!(c.contains("pcall(orig, window, abilityId, hitType, statList)"));
    // Lua sees the separators as decimal escapes.
    assert!(c.contains(r#""\31""#) && c.contains(r#""\30""#));
}

#[test]
fn chunk_subscribes_the_click_ring_globally_and_reads_after_its_mark() {
    let c = pump_chunk(12, 34, 56, 50);
    // CEGUI global event Window/MouseClick, through the stock binding,
    // by the lab's handler name; bound constants preferred, literals
    // as the fallback.
    assert!(c.contains(r#"local ns, ev = "Window", "MouseClick""#));
    assert!(c.contains("ns = CEGUI.Window.EventNamespace or ns"));
    assert!(c.contains("ev = CEGUI.Window.EventMouseClick or ev"));
    assert!(c.contains(r#"host:subscribeGlobal(ns, ev, "CimmeriaLab.on_click")"#));
    assert!(c.contains("L.on_click = function(this, window, position, moveDelta, button)"));
    // The record goes into its own capped ring.
    assert!(c.contains(r#"push(ring("click"), e)"#));
    assert!(c.contains(r#"e.event_id = L.epoch .. ":" .. tostring(e.seq)"#));
    assert!(c.contains("local target_state, target_name = target_at_click()"));
    // The top-level window is the ancestor below Root.
    assert!(c.contains(r#"if pn == nil or pn == "Root" then break end"#));
    // Read after the click mark, bounded, with its head and status.
    assert!(c.contains("e.seq > 56 and q < 50"));
    assert!(c.contains(r#"add("head", "click", k.seq)"#));
    assert!(c.contains(r#"add("install", "click", st_click)"#));
    // The handler never returns a value: the click is never marked
    // handled, so the stock UI still sees it.
    let handler = &c[c.find("L.on_click = function").unwrap()..];
    let handler = &handler[..handler.find("\nend").unwrap()];
    assert!(!handler.contains("return true"), "{handler}");
}

/// The binding has no `unsubscribeGlobal`: a second subscribe would
/// stack a second handler. The chunk runs on every pump, so the
/// subscription must be gated on the per-Lua-state `L.click_host`, which
/// only a successful subscribe sets.
#[test]
fn the_click_install_is_idempotent() {
    let c = pump_chunk(0, 0, 0, READ_MAX);
    assert_eq!(c.matches("subscribeGlobal").count(), 1);
    let gate = c
            .find(r#"if L.click_host ~= nil then st_click = "ok host=" .. tostring(L.click_host_name) return end"#)
            .expect("install gate");
    let sub = c.find("host:subscribeGlobal").unwrap();
    let set = c
        .find("L.click_host = host")
        .expect("host recorded only on success");
    assert!(gate < sub && sub < set);
    // BackgroundWin is tried first and Root is only the fallback: Root is
    // not a child of a ScriptedWindow, so its subscribe raises.
    let hosts = c
        .find(r#"ipairs({ "BackgroundWin", "Root" })"#)
        .expect("hosts in order, BackgroundWin first");
    assert!(gate < hosts && hosts < sub);
    assert!(
        !c.contains(r#"pcall(getWindow, "Root")"#),
        "the Root-only host choice is back"
    );
    // The handler is defined before the subscribe (it is bound by name).
    assert!(c.find("L.on_click = function").unwrap() < sub);
    // The install has its own pcall, opened before the gate, so a fault
    // there cannot stop the combat and chat reads that follow.
    let guard = c
        .find("local okc, cerr = pcall(function()")
        .expect("install pcall");
    assert!(guard < gate && sub < c.find("add(\"head\", \"combat\"").unwrap());
}

/// A failed click install names every host it tried with the Lua error,
/// and the host that worked (#1660, probes C-P4 and C-P7). `failed` alone
/// gave the caller nothing to act on, and the Root-only choice failed
/// on every client.
#[test]
fn a_failed_click_install_reports_each_hosts_error_text() {
    let c = pump_chunk(0, 0, 0, READ_MAX);
    // Failure carries the error text of each host, clipped to 120 chars.
    assert!(c.contains(r#"local oks, serr = pcall(function() host:subscribeGlobal(ns, ev, "CimmeriaLab.on_click") end)"#));
    assert!(c.contains(r#"errs[#errs + 1] = hn .. "=" .. string.sub(clean(serr), 1, 120)"#));
    assert!(c.contains(r#"st_click = "failed: " .. table.concat(errs, "; ")"#));
    assert!(c.contains(
        r#"if not okc then st_click = "failed: " .. string.sub(clean(cerr), 1, 120) end"#
    ));
    // Success names the host.
    assert!(c.contains(r#"st_click = "installed host=" .. hn"#));
    // The pump parses the new statuses as one field each.
    let r = parse_ring_read(&s(&[
            "install\tclick\tfailed: BackgroundWin=no window; Root=Window 'Root' is not a child of a ScriptedWindow",
            "install\tchat\tok",
        ]));
    assert_eq!(
            r.install[0],
            (
                "click".to_string(),
                "failed: BackgroundWin=no window; Root=Window 'Root' is not a child of a ScriptedWindow".to_string()
            )
        );
    assert!(r.bad_lines.is_empty());
    let r = parse_ring_read(&s(&["install\tclick\tinstalled host=BackgroundWin"]));
    assert_eq!(
        r.install,
        vec![(
            "click".to_string(),
            "installed host=BackgroundWin".to_string()
        )]
    );
}

/// A bubbled fire is skipped only on the last click's ancestors, never
/// on the clicked window itself, so two real clicks on one button both
/// record; a fire on Root is never a click.
#[test]
fn the_click_dedup_skips_ancestors_only() {
    let c = pump_chunk(0, 0, 0, READ_MAX);
    assert!(
        c.contains("if name == nil then L.click_rejected = (L.click_rejected or 0) + 1 return end")
    );
    assert!(c.contains("if name == \"Root\" then return end"));
    assert!(c.contains("last.key == key and (t == 0 or last.t == t)"));
    assert!(c.contains("for i = 2, #last.chain do if last.chain[i] == name then return end end"));
    assert!(
        !c.contains("ipairs(last.chain)"),
        "the clicked window itself must not dedup"
    );
}

#[test]
fn click_records_parse_with_window_and_button() {
    let r = parse_ring_read(&s(&[
            "install\tclick\tinstalled",
            "epoch\tlua-a",
            "click\t2\t812.25\tTutorialWin\tTutorialWin__auto_closebutton__\tleft\tlua-a:2\tpresent\tTarget One",
            "click\t3\t813\tChatWin\tChatWin_Tab2\t\tlua-a:3\tabsent\t",
            "click\tnope\t1\ta\tb\tleft",
            "head\tclick\t3",
        ]));
    assert_eq!(
        r.install,
        vec![("click".to_string(), "installed".to_string())]
    );
    assert_eq!(r.click_head, 3);
    assert_eq!(r.click.len(), 2);
    let (seq, ev) = &r.click[0];
    assert_eq!(*seq, 2);
    assert_eq!(ev["window"], "TutorialWin");
    assert_eq!(ev["button"], "TutorialWin__auto_closebutton__");
    assert_eq!(ev["mouse"], "left");
    assert_eq!(ev["ui_time"], 812.25);
    assert_eq!(ev["event_id"], "lua-a:2");
    assert_eq!(ev["click_seq"], 2);
    assert_eq!(ev["click_epoch"], "lua-a");
    assert_eq!(ev["selected_target_name"], "Target One");
    assert!(ev["selected_entity_id"].is_null());
    assert_eq!(ev["target_present"], true);
    assert_eq!(r.click[1].1["mouse"], "");
    assert!(r.click[1].1["selected_target_name"].is_null());
    assert_eq!(r.bad_lines, vec!["click\tnope\t1\ta\tb\tleft".to_string()]);
    assert_eq!(r.click_rejected_lines, 1);
}

#[test]
fn long_widget_names_stay_exact_and_distinct_for_replay() {
    let shared = "x".repeat(161);
    let first = format!("{shared}A");
    let second = format!("{shared}B");
    let r = parse_ring_read(&[
        "epoch\te1".into(),
        format!("click\t1\t1\t{first}\t{first}\tleft\te1:1\tabsent\t"),
        format!("click\t2\t2\t{second}\t{second}\tleft\te1:2\tabsent\t"),
    ]);
    assert_eq!(r.click[0].1["window"], first);
    assert_eq!(r.click[1].1["window"], second);
    assert_ne!(r.click[0].1["button"], r.click[1].1["button"]);
    let chunk = pump_chunk(0, 0, 0, READ_MAX);
    assert!(chunk.contains("window = clean(top), button = clean(name)"));
    assert!(!chunk.contains("string.sub(clean(top), 1, 160)"));
    assert!(chunk.contains(&format!(
        "#name > {CLICK_NAME_CAP} or #top > {CLICK_NAME_CAP}"
    )));
}

#[test]
fn old_format_and_invalid_click_identity_have_visible_rejection_counts() {
    let r = parse_ring_read(&s(&[
        "epoch\te1",
        "click\t1\t1\tOldWin\tOldButton\tleft",
        "click\t2\t1\tWin\tButton\tleft\te1:3\tabsent\t",
        "click\t3\t1\tWin\tButton\tleft\te1:3\tabsent\t",
        "click\t4\t1\tWin\t\tleft\te1:4\tabsent\t",
        "rejected\tclick\t2",
    ]));
    assert_eq!(r.click.len(), 1);
    assert_eq!(r.click_rejected_lines, 3);
    assert_eq!(r.click_rejected_total, 2);
    assert_eq!(r.bad_lines.len(), 3);
    let chunk = pump_chunk(0, 0, 0, READ_MAX);
    assert!(chunk.contains("L.click_rejected = (L.click_rejected or 0) + 1"));
    assert!(chunk.contains("add(\"rejected\", \"click\", L.click_rejected or 0)"));
}

#[test]
fn combat_records_parse_with_their_stat_list() {
    let stats = format!("1{US}Health{US}-57{US}0{US}Normal{US}2{RS}2{US}Focus{US}-5{US}0{US}{US}0");
    let line = format!(
        "combat\t3\t1234.5\t1100\tPistol Shot\t1\tHit\tLabone\t1\tCellblock Guard\t0\t0\t{stats}"
    );
    let r = parse_ring_read(&s(&[
        "epoch\t0A1B2C3D-99.5",
        "install\tcombat\tinstalled",
        "install\tchat\tok",
        &line,
        "head\tcombat\t3",
        "head\tchat\t0",
    ]));
    assert_eq!(r.epoch, "0A1B2C3D-99.5");
    assert_eq!(
        r.install,
        vec![
            ("combat".to_string(), "installed".to_string()),
            ("chat".to_string(), "ok".to_string())
        ]
    );
    assert_eq!(r.combat_head, 3);
    let (seq, ev) = &r.combat[0];
    assert_eq!(*seq, 3);
    assert_eq!(ev["ability_id"], 1100);
    assert_eq!(ev["hit_name"], "Hit");
    assert_eq!(ev["source_is_player"], true);
    assert_eq!(ev["target"], "Cellblock Guard");
    assert_eq!(ev["mortal"], false);
    assert_eq!(ev["stats"][0]["stat"], "Health");
    assert_eq!(ev["stats"][0]["value"], -57);
    assert_eq!(ev["stats"][1]["result_name"], "");
    assert!(r.bad_lines.is_empty());
}

#[test]
fn chat_records_keep_text_with_spaces() {
    let r = parse_ring_read(&s(&[
        "chat\t7\t88\t9\tFeedback\t\t0\tYou must have a target",
        "head\tchat\t7",
    ]));
    let (seq, ev) = &r.chat[0];
    assert_eq!(*seq, 7);
    assert_eq!(ev["channel"], 9);
    assert_eq!(ev["text"], "You must have a target");
    assert_eq!(r.chat_head, 7);
}

#[test]
fn malformed_lines_are_kept_not_fatal() {
    let r = parse_ring_read(&s(&["combat\tnotanumber", "wat"]));
    assert_eq!(r.bad_lines.len(), 2);
    assert!(r.combat.is_empty());
}

#[test]
fn empty_stat_list_is_empty() {
    assert!(parse_stats("").is_empty());
}
