//! The router's plugin lookup (#962): a plugin-owned cell method reaches its
//! plugin through `dispatch_cell_method`, and with the plugin missing it
//! reaches the router's "Unhandled cell method call" WARN instead of any
//! other handler (the negative log for a missing registration).

use cimmeria_cell_pets::PetsPlugin;
use cimmeria_cell_world::cell::plugin::{CellPlugins, PLUGIN_OWNED_CELL_METHODS};
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::dispatch_cell_method;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{make_space_manager_with_player, LogCapture};

fn count(
    capture: &crate::test_support::LogCaptureGuard,
    pred: impl Fn(&crate::test_support::Captured) -> bool,
) -> usize {
    capture.all().into_iter().filter(|c| pred(c)).count()
}

/// With the pets plugin installed, 88-90 reach the pet command parser (the
/// `malformed_args` refusal only it logs) and never the unhandled warn.
#[tokio::test]
async fn plugin_owned_methods_route_to_the_installed_plugin() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager_with_player(1);
    mgr.install_plugins(CellPlugins::build(&[&PetsPlugin]).unwrap());
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8);
    let engine = ChainEngine::new();

    for &index in PLUGIN_OWNED_CELL_METHODS {
        dispatch_cell_method(1, index, &[], &tx, &mut mgr, &engine).await;
    }

    let malformed = count(&capture, |c| {
        c.level == Level::WARN
            && c.target == "pets.command"
            && c.has_field("reason", "malformed_args")
    });
    assert_eq!(malformed, 3, "captured: {:#?}", capture.all());
    assert!(
        capture
            .find_message(Level::WARN, "Unhandled cell method call")
            .is_none(),
        "a plugin-owned method must not reach the unhandled warn: {:#?}",
        capture.all()
    );
}

/// #962 test rule: a missing registration never silently no-ops. With no
/// plugin installed, each plugin-owned index logs the router's unhandled
/// WARN with its `method_index`, and nothing else handles it.
#[tokio::test]
async fn a_missing_plugin_logs_unhandled_for_each_plugin_owned_method() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager_with_player(1);
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(8);
    let engine = ChainEngine::new();

    for &index in PLUGIN_OWNED_CELL_METHODS {
        dispatch_cell_method(1, index, &[0; 12], &tx, &mut mgr, &engine).await;
    }

    for &index in PLUGIN_OWNED_CELL_METHODS {
        let hits = count(&capture, |c| {
            c.level == Level::WARN
                && c.message_contains("Unhandled cell method call")
                && c.has_field("method_index", &index.to_string())
        });
        assert_eq!(hits, 1, "method {index}: captured {:#?}", capture.all());
    }
    assert_eq!(
        count(&capture, |c| c.target == "pets.command"),
        0,
        "no pet handler may run without the plugin"
    );
    assert!(
        rx.try_recv().is_err(),
        "nothing may be sent for an unhandled method"
    );
}
