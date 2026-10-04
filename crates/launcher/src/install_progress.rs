//! Progress is observational: desktop consumers retain only the newest value.
use crate::install::Progress;
use tokio::sync::{mpsc, watch};

pub trait ProgressReporter: Send + Sync {
    fn report(&self, value: Progress);
}

impl ProgressReporter for mpsc::UnboundedSender<Progress> {
    fn report(&self, value: Progress) {
        let _ = self.send(value);
    }
}

#[derive(Clone)]
pub enum ProgressSink {
    /// Preserve the existing egui worker's event stream.
    Legacy(mpsc::UnboundedSender<Progress>),
    /// Bounded to one observation even when the UI is stalled or disconnected.
    Latest(watch::Sender<Option<Progress>>),
}
impl From<mpsc::UnboundedSender<Progress>> for ProgressSink {
    fn from(sender: mpsc::UnboundedSender<Progress>) -> Self {
        Self::Legacy(sender)
    }
}
impl ProgressSink {
    pub fn latest() -> (Self, watch::Receiver<Option<Progress>>) {
        let (sender, receiver) = watch::channel(None);
        (Self::Latest(sender), receiver)
    }
}
impl ProgressReporter for ProgressSink {
    fn report(&self, value: Progress) {
        match self {
            Self::Legacy(sender) => sender.report(value),
            Self::Latest(sender) => {
                sender.send_replace(Some(value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn progress(downloaded: u64) -> Progress {
        Progress::Downloading {
            label: "seed".into(),
            downloaded,
            total: 10_000,
        }
    }
    #[test]
    fn stalled_consumer_retains_only_latest_progress() {
        let (sink, mut receiver) = ProgressSink::latest();
        for n in 0..10_000 {
            sink.report(progress(n));
        }
        assert!(matches!(
            *receiver.borrow_and_update(),
            Some(Progress::Downloading {
                downloaded: 9999,
                ..
            })
        ));
        assert!(!receiver.has_changed().unwrap());
        drop(receiver);
        sink.report(progress(10_000)); // Observation loss must not fail installation.
    }
    #[test]
    fn legacy_adapter_preserves_event_order() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let sink = ProgressSink::from(sender);
        sink.report(progress(1));
        sink.report(progress(2));
        for n in [1, 2] {
            assert!(
                matches!(receiver.try_recv().unwrap(), Progress::Downloading {downloaded,..} if downloaded==n)
            );
        }
    }
}
