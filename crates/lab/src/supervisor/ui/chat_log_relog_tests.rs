//! Lua lifecycle guards for chat capture across window recreation and relog.

use super::*;

#[test]
fn chat_capture_rebinds_once_and_keeps_gap_for_chat_reader() {
    use crate::supervisor::events::lua_rings;

    let lua = mlua::Lua::new();
    lua.load(
        r#"
Events = { MessageReceived = 1 }
function getSystemTime() return 123 end
local calls = 0
ChatMod = { onMessageReceived = function() calls = calls + 1 end }
local function window()
  local w = { subscriptions = 0, handler = nil }
  function w:unsubscribe(_) self.handler = nil end
  function w:subscribe(_, _)
    if self.fail_subscribe then error('subscribe refused') end
    self.subscriptions = self.subscriptions + 1
    self.handler = ChatMod.onMessageReceived
  end
  function w:destroy() self.handler = nil end
  function w:emit(speaker, flags, channel, name, text)
    if self.handler then self.handler(self, speaker, flags, channel, name, text) end
  end
  return w
end
Inst1ChatWin = window()
Inst1ChatWin:subscribe(Events.MessageReceived, 'ChatMod.onMessageReceived')
function make_window() return window() end
function original_calls() return calls end
__jenc = function(x) return x end
"#,
    )
    .exec()
    .unwrap();

    let install = |lua: &mlua::Lua, max| {
        // Run the exact production install block; the remainder reads
        // unrelated combat and click state and is not needed here.
        let chunk = lua_rings::pump_chunk(0, 0, 0, max);
        let end = chunk.find("local function wname").unwrap();
        lua.load(&chunk[..end]).exec().unwrap();
    };
    let status = || -> String {
        lua.globals()
            .get::<mlua::Table>("CimmeriaLab")
            .unwrap()
            .get("chat_install_status")
            .unwrap()
    };
    install(&lua, 0);
    let first_gap = status();
    assert!(first_gap.starts_with("gap_installed@"));
    lua.load("Inst1ChatWin:emit('', 0, 9, 'Feedback', 'before logout')")
        .exec()
        .unwrap();
    lua.load("Inst1ChatWin:destroy()").exec().unwrap();
    lua.load("Inst1ChatWin = make_window(); Inst1ChatWin:subscribe(Events.MessageReceived, 'ChatMod.onMessageReceived')")
            .exec()
            .unwrap();
    // In the same Lua state the new stock subscription resolves our
    // existing wrapper, so this line is captured before re-subscription.
    lua.load("Inst1ChatWin:emit('', 0, 9, 'Feedback', 'Welcome after relog')")
        .exec()
        .unwrap();
    install(&lua, 0);
    let relog_gap = status();
    assert!(relog_gap.starts_with("gap_resubscribed@"));
    // Combat and wait readers also pump with max > 0. They cannot clear
    // the warning before client_chat_log has seen it.
    install(&lua, lua_rings::READ_MAX);
    assert_eq!(status(), relog_gap);
    lua.load("Inst1ChatWin:emit('', 0, 9, 'Feedback', 'second line')")
        .exec()
        .unwrap();
    install(&lua, lua_rings::READ_MAX);
    assert_eq!(status(), relog_gap);

    let records: mlua::Table = lua
        .globals()
        .get::<mlua::Table>("CimmeriaLab")
        .unwrap()
        .get("chat")
        .unwrap();
    let items: mlua::Table = records.get("items").unwrap();
    assert_eq!(items.raw_len(), 3);
    assert_eq!(
        items
            .get::<mlua::Table>(2)
            .unwrap()
            .get::<String>("text")
            .unwrap(),
        "Welcome after relog"
    );
    assert_eq!(
        items
            .get::<mlua::Table>(3)
            .unwrap()
            .get::<String>("text")
            .unwrap(),
        "second line"
    );
    let window: mlua::Table = lua.globals().get("Inst1ChatWin").unwrap();
    assert_eq!(window.get::<u32>("subscriptions").unwrap(), 2);
    assert_eq!(
        lua.load("original_calls()").eval::<u32>().unwrap(),
        3,
        "the stock handler receives each line exactly once"
    );
    let acked: mlua::Table = lua.load(acknowledge_gap_chunk(&first_gap)).eval().unwrap();
    assert!(
        !acked.get::<bool>("acked").unwrap(),
        "stale reader cleared the new gap"
    );
    install(&lua, 0);
    assert_eq!(status(), relog_gap);
    let acked: mlua::Table = lua.load(acknowledge_gap_chunk(&relog_gap)).eval().unwrap();
    assert!(acked.get::<bool>("acked").unwrap());
    install(&lua, 0);
    assert_eq!(status(), "ok");
}

#[test]
fn rebuilt_lua_state_reports_welcome_emitted_before_hook_install() {
    use crate::supervisor::events::lua_rings;

    let lua = mlua::Lua::new();
    lua.load(
        r#"
Events = { MessageReceived = 1 }
function getSystemTime() return 123 end
local shown = {}
function stock(this, speaker, flags, channel, name, text) shown[#shown + 1] = text end
function shown_count() return #shown end
function window()
  local w = {}
  function w:unsubscribe(_) self.handler = nil end
  function w:subscribe(_, _) self.handler = ChatMod.onMessageReceived end
  function w:emit(text) if self.handler then self.handler(self, '', 0, 9, 'Feedback', text) end end
  return w
end
ChatMod = { onMessageReceived = stock }
Inst1ChatWin = window()
Inst1ChatWin:subscribe(Events.MessageReceived, 'ChatMod.onMessageReceived')
"#,
    )
    .exec()
    .unwrap();
    let chunk = lua_rings::pump_chunk(0, 0, 0, 0);
    let end = chunk.find("local function wname").unwrap();
    lua.load(&chunk[..end]).exec().unwrap();
    lua.load("Inst1ChatWin:emit('before logout')")
        .exec()
        .unwrap();

    // Relog rebuilt the UI Lua globals. The stock window receives the
    // onClientReady welcome before the keeper's first post-load pump.
    lua.load(
        "CimmeriaLab = nil; ChatMod = { onMessageReceived = stock }; \
         Inst1ChatWin = window(); \
         Inst1ChatWin:subscribe(Events.MessageReceived, 'ChatMod.onMessageReceived'); \
         Inst1ChatWin:emit('Welcome after relog')",
    )
    .exec()
    .unwrap();
    lua.load(&chunk[..end]).exec().unwrap();
    lua.load("Inst1ChatWin:emit('later feedback')")
        .exec()
        .unwrap();
    let state: mlua::Table = lua.globals().get("CimmeriaLab").unwrap();
    let items: mlua::Table = state
        .get::<mlua::Table>("chat")
        .unwrap()
        .get("items")
        .unwrap();
    assert_eq!(items.raw_len(), 1, "the pre-install welcome was fabricated");
    assert_eq!(
        items
            .get::<mlua::Table>(1)
            .unwrap()
            .get::<String>("text")
            .unwrap(),
        "later feedback"
    );
    assert_eq!(lua.load("shown_count()").eval::<usize>().unwrap(), 3);
    let status: String = state.get("chat_install_status").unwrap();
    assert!(status.starts_with("gap_installed@"), "{status}");
    assert!(capture_warning(&[("chat".into(), status)])
        .unwrap()
        .contains("onClientReady welcome"));
}

#[test]
fn chat_resubscribe_failure_reports_lua_error_and_retries() {
    use crate::supervisor::events::lua_rings;

    let lua = mlua::Lua::new();
    lua.load(
        r#"
Events = { MessageReceived = 1 }
function getSystemTime() return 123 end
ChatMod = { onMessageReceived = function() end }
Inst1ChatWin = {
  unsubscribe = function() end,
  subscribe = function() error('subscription denied by CEGUI') end,
}
"#,
    )
    .exec()
    .unwrap();
    let chunk = lua_rings::pump_chunk(0, 0, 0, 0);
    let end = chunk.find("local function wname").unwrap();
    lua.load(&chunk[..end]).exec().unwrap();
    let state: mlua::Table = lua.globals().get("CimmeriaLab").unwrap();
    let status: String = state.get("chat_install_status").unwrap();
    assert!(status.starts_with("failed: "), "{status}");
    assert!(status.contains("subscription denied by CEGUI"), "{status}");
    assert!(capture_warning(&[("chat".into(), status)])
        .unwrap()
        .contains("capture.lua_install"));
    lua.load("Inst1ChatWin.subscribe = function() end")
        .exec()
        .unwrap();
    lua.load(&chunk[..end]).exec().unwrap();
    let status: String = state.get("chat_install_status").unwrap();
    assert!(status.starts_with("gap_installed@"), "{status}");
}
