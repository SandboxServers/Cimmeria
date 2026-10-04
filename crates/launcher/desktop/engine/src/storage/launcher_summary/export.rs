//! The exporter: one task that delivers queued summaries to the configured
//! endpoint. A cycle mints a short-lived token, posts one batch and applies the
//! server's verdicts.
//!
//! The state mutex is taken only on a blocking thread and never held across a
//! request or a sleep. Consent, the endpoint and the queue generation are checked
//! again after the mint, after every wait and before verdicts are applied, so a
//! change in between sends and applies nothing.
//!
//! Withdrawing consent also cancels the batch's token. A mint or POST still in
//! flight is dropped, which aborts it, and a backoff wait ends at once; the
//! cycle ends as `ConsentWithdrawn`. Bytes the server had already received
//! cannot be recalled; their answer is never read or applied.
//!
//! The token lives for one attempt and is never stored or logged. The requests
//! themselves are in `exchange.rs`. Nothing here can fail, delay or change the
//! launcher's own work.
use super::{
    batch::{Batch, ExportTarget, Take},
    endpoint::SummaryEndpoint,
    exchange::{mint, post},
    schema::{LauncherVersion, SummaryResult, MAX_BODY_BYTES},
    tracker::{lock, Gate},
    SummaryConfig,
};
use crate::storage::DesktopState;
use std::{
    future::Future,
    panic::{catch_unwind, AssertUnwindSafe},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    time::Duration,
};
use tokio::{sync::Notify, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub(super) struct ExportTuning {
    pub mint_timeout: Duration,
    pub post_timeout: Duration,
    /// Retries after the first attempt of one cycle. Each one mints again.
    pub max_retries: u32,
    /// The longest wait a server may ask for, and the wait when it names none.
    pub retry_after_cap: Duration,
    /// The wait before the n-th retry, from 1, after a transient failure.
    pub backoff: fn(u32) -> Duration,
}
impl ExportTuning {
    pub const PRODUCTION: Self = Self {
        mint_timeout: Duration::from_secs(2),
        post_timeout: Duration::from_secs(2),
        max_retries: 2,
        retry_after_cap: Duration::from_secs(60),
        backoff: jittered_backoff,
    };
}

// 250 ms doubling to 1 s, plus up to as much again at random: 250 ms to 2 s.
fn jittered_backoff(retry: u32) -> Duration {
    let base = 250_u64 << retry.saturating_sub(1).min(2);
    let random = u64::from(Uuid::new_v4().as_bytes()[0]);
    Duration::from_millis(base + base * random / 256)
}

pub(super) type Sleeper =
    Box<dyn Fn(Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

/// Everything a cycle needs besides the state. Tests replace the tuning, the
/// sleeper and the id source; the endpoint and the client are production's.
pub(super) struct ExportEnv {
    pub endpoint: SummaryEndpoint,
    pub launcher_version: LauncherVersion,
    pub tuning: ExportTuning,
    pub sleep: Sleeper,
    /// The mint's `install_id`: random for every mint and never stored.
    pub mint_id: Box<dyn Fn() -> Uuid + Send + Sync>,
    pub http: reqwest::Client,
    // Latched by `StoppedForRun`: no further request in this process run.
    stopped: AtomicBool,
}
impl ExportEnv {
    pub fn production(target: &ExportTarget) -> Option<Self> {
        let http = reqwest::Client::builder()
            // Only the configured endpoint is ever contacted.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .ok()?;
        Some(Self {
            endpoint: target.endpoint.clone(),
            launcher_version: target.launcher_version,
            tuning: ExportTuning::PRODUCTION,
            sleep: Box::new(|wait| Box::pin(tokio::time::sleep(wait))),
            mint_id: Box::new(Uuid::new_v4),
            http,
            stopped: AtomicBool::new(false),
        })
    }

    pub(super) fn stop(&self) -> Step {
        self.stopped.store(true, Ordering::SeqCst);
        Step::Done(CycleOutcome::StoppedForRun)
    }
}

/// How one cycle ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::storage) enum CycleOutcome {
    /// No endpoint is configured, or not the one this exporter was started for.
    SkippedNoEndpoint,
    /// The gate is closed: no consent, a withdrawal in doubt, or a state that
    /// must be reopened.
    SkippedNoConsent,
    Empty,
    /// The server answered for the batch and every answered row left the queue.
    /// A body refused as a whole (`400`, `413`) counts all its rows as rejected.
    Delivered {
        accepted: usize,
        duplicate: usize,
        rejected: usize,
    },
    /// The gate closed or the queue was replaced mid-cycle. Nothing was applied.
    ConsentWithdrawn,
    /// The server has no summary routes. No further request in this process run.
    StoppedForRun,
    /// The retry budget ran out. The rows wait for the next trigger.
    GaveUp,
    StateGone,
}

pub(super) enum Step {
    Done(CycleOutcome),
    /// A transient failure. `Some` is the wait the server asked for.
    Retry(Option<Duration>),
}

pub(super) enum Verdicts {
    Each(Vec<SummaryResult>),
    /// The whole body was refused for good.
    Refused,
}

/// Configures summaries and, when an endpoint is configured and a runtime is
/// running, starts the one exporter task. Without an endpoint nothing is
/// spawned and any queue file is removed. The task holds no strong state handle:
/// dropping the state ends it and releases the state directory.
pub fn start(state: &Arc<Mutex<DesktopState>>, config: SummaryConfig) {
    // Called under the host's own guard, so a panic must not leave this function.
    let _ = catch_unwind(AssertUnwindSafe(|| {
        spawn(state, config, ExportEnv::production)
    }));
}

pub(super) fn spawn(
    state: &Arc<Mutex<DesktopState>>,
    config: SummaryConfig,
    env: impl FnOnce(&ExportTarget) -> Option<ExportEnv>,
) -> Option<JoinHandle<()>> {
    let runtime = tokio::runtime::Handle::try_current().ok();
    let target = {
        let mut owner = state.lock().ok()?;
        owner.configure_summaries(config);
        // Without a runtime nothing is claimed, so a later call can still start it.
        runtime.as_ref()?;
        owner.summary_claim_exporter()?
    };
    let env = env(&target)?;
    Some(runtime?.spawn(run_task(
        Arc::downgrade(state),
        env,
        target.wake,
        target.shutdown,
    )))
}

impl DesktopState {
    // `None` without an endpoint, or when this state already has its exporter.
    fn summary_claim_exporter(&mut self) -> Option<ExportTarget> {
        self.summary_guarded(|state| {
            let target = state.summary_export_target()?;
            let claimed = std::mem::replace(&mut lock(&state.summaries).exporting, true);
            (!claimed).then_some(target)
        })
        .flatten()
    }
}

async fn run_task(
    state: Weak<Mutex<DesktopState>>,
    env: ExportEnv,
    wake: Arc<Notify>,
    shutdown: CancellationToken,
) {
    loop {
        loop {
            match run_cycle(&state, &env).await {
                CycleOutcome::StateGone | CycleOutcome::StoppedForRun => return,
                // More rows may be waiting than one batch holds.
                CycleOutcome::Delivered { .. } => (),
                _ => break,
            }
        }
        tokio::select! {
            () = shutdown.cancelled() => return,
            () = wake.notified() => (),
        }
    }
}

pub(super) async fn run_cycle(state: &Weak<Mutex<DesktopState>>, env: &ExportEnv) -> CycleOutcome {
    if env.stopped.load(Ordering::SeqCst) {
        return CycleOutcome::StoppedForRun;
    }
    let endpoint = env.endpoint.clone();
    let taken = locked(state, move |owner| {
        if !configured(owner, &endpoint) {
            return Take::Closed(Gate::NoEndpoint);
        }
        owner.summary_take_batch()
    })
    .await;
    let batch = match taken {
        None => return CycleOutcome::StateGone,
        Some(Take::Closed(Gate::NoEndpoint)) => return CycleOutcome::SkippedNoEndpoint,
        Some(Take::Closed(_)) => return CycleOutcome::SkippedNoConsent,
        Some(Take::Empty) => return CycleOutcome::Empty,
        Some(Take::Ready(batch)) => Arc::new(batch),
    };
    // The queue keeps a batch well under the server's limit; this cannot fail.
    let body = match serde_json::to_vec(&batch.request()) {
        Ok(body) if body.len() <= MAX_BODY_BYTES => body,
        _ => return CycleOutcome::GaveUp,
    };
    let mut retry = 0;
    loop {
        let wait = match attempt(state, env, &batch, &body).await {
            Step::Done(outcome) => return outcome,
            Step::Retry(wait) => wait,
        };
        if retry == env.tuning.max_retries {
            return CycleOutcome::GaveUp;
        }
        retry += 1;
        let wait = wait.unwrap_or_else(|| (env.tuning.backoff)(retry));
        // A withdrawal ends the wait early; the check below decides.
        tokio::select! {
            () = batch.cancel.cancelled() => (),
            () = (env.sleep)(wait) => (),
        }
        if let Some(outcome) = stale(state, env, &batch).await {
            return outcome;
        }
    }
}

// One request, unless or until consent is withdrawn. The cancelled token wins
// over an answer that is ready at the same moment, and dropping the request
// future aborts the request: nothing more is sent and no answer is read.
async fn unless_withdrawn<T>(
    batch: &Batch,
    request: impl Future<Output = Result<T, Step>>,
) -> Result<T, Step> {
    tokio::select! {
        biased;
        () = batch.cancel.cancelled() => Err(Step::Done(CycleOutcome::ConsentWithdrawn)),
        answered = request => answered,
    }
}

// Mint, check again, post, apply. The token is a local of this one attempt.
async fn attempt(
    state: &Weak<Mutex<DesktopState>>,
    env: &ExportEnv,
    batch: &Arc<Batch>,
    body: &[u8],
) -> Step {
    let token = match unless_withdrawn(batch, mint(env)).await {
        Ok(token) => token,
        Err(step) => return step,
    };
    // The gate, the endpoint or the queue may have changed while the mint was
    // in flight, with or without a withdrawal.
    if let Some(outcome) = stale(state, env, batch).await {
        return Step::Done(outcome);
    }
    let posted = post(env, token, body.to_vec(), batch.summaries.len());
    let verdicts = match unless_withdrawn(batch, posted).await {
        Ok(verdicts) => verdicts,
        Err(step) => return step,
    };
    let count = |wanted: SummaryResult| match &verdicts {
        Verdicts::Each(results) => results.iter().filter(|result| **result == wanted).count(),
        Verdicts::Refused if wanted == SummaryResult::Rejected => batch.summaries.len(),
        Verdicts::Refused => 0,
    };
    let delivered = CycleOutcome::Delivered {
        accepted: count(SummaryResult::Accepted),
        duplicate: count(SummaryResult::Duplicate),
        rejected: count(SummaryResult::Rejected),
    };
    let (endpoint, batch) = (env.endpoint.clone(), batch.clone());
    // Both calls refuse a batch whose generation is no longer the queue's.
    let applied = locked(state, move |owner| {
        configured(owner, &endpoint)
            && match &verdicts {
                Verdicts::Each(results) => owner.summary_apply_results(&batch, results),
                Verdicts::Refused => owner.summary_reject_batch(&batch),
            }
    })
    .await;
    Step::Done(match applied {
        None => CycleOutcome::StateGone,
        Some(false) => CycleOutcome::ConsentWithdrawn,
        Some(true) => delivered,
    })
}

// One lock section, on a blocking thread. `None` when the state is gone or its
// mutex is poisoned. A panic is contained while the guard is held, so the
// exporter can never poison the launcher's state.
async fn locked<R: Send + 'static>(
    state: &Weak<Mutex<DesktopState>>,
    run: impl FnOnce(&mut DesktopState) -> R + Send + 'static,
) -> Option<R> {
    let state = state.upgrade()?;
    tokio::task::spawn_blocking(move || {
        let mut owner = state.lock().ok()?;
        catch_unwind(AssertUnwindSafe(|| run(&mut owner))).ok()
    })
    .await
    .ok()
    .flatten()
}

// Only the endpoint configured right now may be contacted.
fn configured(owner: &DesktopState, endpoint: &SummaryEndpoint) -> bool {
    owner
        .summary_export_target()
        .is_some_and(|target| &target.endpoint == endpoint)
}

// `Some` when the batch may no longer be sent or applied.
async fn stale(
    state: &Weak<Mutex<DesktopState>>,
    env: &ExportEnv,
    batch: &Batch,
) -> Option<CycleOutcome> {
    let (endpoint, generation) = (env.endpoint.clone(), batch.generation);
    let current = locked(state, move |owner| {
        configured(owner, &endpoint) && owner.summary_batch_current(generation)
    })
    .await;
    match current {
        None => Some(CycleOutcome::StateGone),
        Some(false) => Some(CycleOutcome::ConsentWithdrawn),
        Some(true) => None,
    }
}
