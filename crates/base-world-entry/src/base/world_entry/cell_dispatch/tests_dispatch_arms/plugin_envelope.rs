//! The `Plugin` arm (#962 step 5, plugin ADR §3.4): an envelope reaches the
//! base plugin that consumes its payload type, with the dispatcher's own
//! maps; with no consumer it is dropped with a WARN, never silently.

use std::sync::Mutex as StdMutex;

use super::super::*;
use super::one_session;
use crate::cell::messages::PluginMsg;
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_base_session::base::plugin::{
    BaseCtx, BasePlugin, BasePluginBuilder, BasePlugins, BoxFuture, PluginMsgKind, PluginOwnership,
};
use tracing::Level;

/// A feature payload: the entity it is for.
struct Nudge(u32);

const OWNED: PluginOwnership = PluginOwnership {
    base_methods: &[],
    cell_messages: &[PluginMsgKind::of::<Nudge>()],
};

/// `(entity id, sessions the consumer's context saw)` rows.
static SEEN: StdMutex<Vec<(u32, usize)>> = StdMutex::new(Vec::new());

fn consume(msg: PluginMsg, ctx: BaseCtx<'_>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        let Ok(Nudge(entity_id)) = msg.downcast::<Nudge>() else {
            panic!("routed by type");
        };
        let sessions = ctx.connected.lock().unwrap().len();
        SEEN.lock().unwrap().push((entity_id, sessions));
    })
}

struct NudgePlugin;
impl BasePlugin for NudgePlugin {
    fn name(&self) -> &'static str {
        "nudge"
    }
    fn build(&self, plugin: &mut BasePluginBuilder<'_>) {
        plugin.on_cell_message::<Nudge>(consume);
    }
}

#[tokio::test]
async fn an_envelope_reaches_its_consumer_with_the_dispatch_maps() {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_id = 4_601;
    let (_addr, connected, entity_to_addr) = one_session(entity_id, false);
    let plugins = BasePlugins::build_with(&[&NudgePlugin], OWNED).unwrap();
    plugins.check_complete().unwrap();

    route_cell_message(
        CellToBaseMsg::Plugin(PluginMsg::new(Nudge(entity_id))),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
        &plugins,
    )
    .await;

    let seen: Vec<(u32, usize)> = SEEN
        .lock()
        .unwrap()
        .iter()
        .copied()
        .filter(|(e, _)| *e == entity_id)
        .collect();
    assert_eq!(seen, vec![(entity_id, 1)]);
}

/// The dispatcher without a plugin table (`handle_cell_message`) drops an
/// envelope with the no-consumer WARN at `base.plugin`.
#[tokio::test]
async fn an_envelope_with_no_consumer_is_dropped_with_a_warn() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_id = 4_602;
    let (_addr, connected, entity_to_addr) = one_session(entity_id, false);

    handle_cell_message(
        CellToBaseMsg::Plugin(PluginMsg::new(Nudge(entity_id))),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(SEEN.lock().unwrap().iter().all(|(e, _)| *e != entity_id));
    let warn = capture
        .find_message(Level::WARN, "cell message has no base plugin consumer")
        .unwrap_or_else(|| panic!("no-consumer WARN missing: {:#?}", capture.all()));
    assert_eq!(warn.target, "base.plugin");
    assert!(
        warn.fields
            .get("type_name")
            .is_some_and(|t| t.contains("Nudge")),
        "{warn:?}"
    );
}
