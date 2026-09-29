//! The worker's end of the event channel, which wakes the UI on every send.
//!
//! egui paints only when there is input or when something calls
//! `Context::request_repaint`. The worker's tasks finish on tokio threads,
//! so a bare channel send leaves the event sitting in the queue until the
//! mouse next moves over the window: a launcher left alone showed
//! "Checking…" and "Fetching manifest…" for as long as nobody touched it.
//! Every send therefore calls the [`Waker`] the app supplied, which asks
//! egui for one repaint; the frame then drains the channel. Nothing wakes
//! the UI while no work is running, so an idle launcher stays idle.

use std::sync::Arc;

use tokio::sync::mpsc;

use super::Event;

/// Called after each event is queued. The app passes a closure over its
/// `egui::Context` that calls `request_repaint`, which is safe to call
/// from any thread.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// A waker that does nothing, for workers with no UI (tests).
#[cfg(test)]
pub fn no_waker() -> Waker {
    Arc::new(|| {})
}

/// Sending half of the worker's event channel. Every worker task sends
/// through this type, never a bare `UnboundedSender`, so no send path can
/// forget the wake.
#[derive(Clone)]
pub struct EventSender {
    tx: mpsc::UnboundedSender<Event>,
    wake: Waker,
}

impl std::fmt::Debug for EventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSender")
            .field("closed", &self.tx.is_closed())
            .finish_non_exhaustive()
    }
}

impl EventSender {
    pub fn new(tx: mpsc::UnboundedSender<Event>, wake: Waker) -> Self {
        Self { tx, wake }
    }

    /// Queue `ev` and wake the UI. Returns false when the app has gone
    /// away (the receiver is dropped); then there is nothing to wake.
    /// Callers ignore the result, as they did the channel's own `send`.
    pub fn send(&self, ev: Event) -> bool {
        if self.tx.send(ev).is_err() {
            return false;
        }
        (self.wake)();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn counting_waker() -> (Waker, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        (
            Arc::new(move || {
                c.fetch_add(1, Ordering::SeqCst);
            }),
            count,
        )
    }

    #[test]
    fn every_send_wakes_the_ui_once() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let (wake, count) = counting_waker();
        let sender = EventSender::new(tx, wake);
        assert!(sender.send(Event::ManifestError("offline".into())));
        assert!(sender.clone().send(Event::InstallComplete));
        assert_eq!(count.load(Ordering::SeqCst), 2);
        assert!(matches!(rx.try_recv(), Ok(Event::ManifestError(_))));
        assert!(matches!(rx.try_recv(), Ok(Event::InstallComplete)));
    }

    // Bug shape: the manifest fetch finished on a tokio thread and the
    // window kept showing "Fetching manifest…" until the mouse moved. The
    // worker must wake the UI when a background job posts its result.
    #[test]
    fn a_finished_manifest_fetch_wakes_the_ui() {
        let rt = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let (wake, count) = counting_waker();
        let mut worker = crate::worker::Worker::new(rt.clone(), wake);
        // The worker's client is https-only, so a plain-http URL fails
        // at once without touching the network.
        worker.fetch_manifest_now("http://127.0.0.1:9/manifest.json".into());
        let ev = rt.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(5), worker.events_rx.recv())
                .await
                .expect("no event before timeout")
                .expect("channel closed")
        });
        assert!(matches!(ev, Event::ManifestError(_)), "got {ev:?}");
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    /// With the app gone there is no frame to request.
    #[test]
    fn a_send_to_a_closed_channel_does_not_wake() {
        let (tx, rx) = mpsc::unbounded_channel();
        drop(rx);
        let (wake, count) = counting_waker();
        let sender = EventSender::new(tx, wake);
        assert!(!sender.send(Event::InstallComplete));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
}
