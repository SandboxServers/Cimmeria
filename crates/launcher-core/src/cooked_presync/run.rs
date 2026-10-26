//! The presync run: the six steps of the contract's `run` order. A transient
//! refusal waits and retries within limits, a version change restarts once
//! from the index, and the deadline and the cancel stop the run at the step
//! in progress. Every failure names its step and the fields the failure table
//! gives, so the engine can tell the player what failed and why (D-LP26).

use std::future::Future;
use std::path::Path;
use std::time::{Duration, Instant};

use cimmeria_cooked_pak::catalog::{CategoryBody, Index, IndexEntry};
use cimmeria_cooked_pak::decide::Action;
use tokio_util::sync::CancellationToken;

use super::apply::{self, ApplyError, Plan, Staged};
use super::fetch::{FetchError, Fetcher};

pub use super::report::{
    CategoryAction, CategoryReport, Failure, Outcome, Progress, Report, Stage, Step,
};

/// The whole run's limit (D-LP25).
pub const DEADLINE: Duration = Duration::from_secs(120);
/// How many transient-refusal waits one run may take (D-LP20).
pub const MAX_WAITS: u32 = 3;
/// The longest single `Retry-After` wait (D-LP20).
pub const MAX_WAIT: Duration = Duration::from_secs(30);
/// The wait for a 503 or 429 that carries no `Retry-After`.
const DEFAULT_RETRY_AFTER_S: u32 = 5;

/// Why a step stopped before the run's end.
enum Halt {
    /// The server or the settings skip the presync.
    Skipped(&'static str),
    /// A step failed.
    Failed(Box<Failure>),
    /// The cancel token stopped the run; the failure names the step.
    Cancelled(Box<Failure>),
    /// The body's version changed; the run restarts once from the index.
    Restart,
}

/// Why [`guarded`] stopped a future before it finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// The run's deadline passed.
    Deadline,
    /// The cancel token was cancelled.
    Cancelled,
}

/// Runs `future` until it finishes, the deadline passes or the cancel fires.
/// A cancel that is already set wins over a future that is ready.
async fn guarded<T>(
    future: impl Future<Output = T>,
    deadline: Instant,
    cancel: &CancellationToken,
) -> Result<T, Stop> {
    tokio::select! {
        biased;
        () = cancel.cancelled() => Err(Stop::Cancelled),
        () = tokio::time::sleep_until(deadline.into()) => Err(Stop::Deadline),
        value = future => Ok(value),
    }
}

/// Whether a refusal is transient: a 503 that is still building the catalog,
/// or a 429. Both may wait `Retry-After` and retry.
fn is_transient(error: &FetchError) -> bool {
    matches!(
        error,
        FetchError::CatalogUnavailable { .. } | FetchError::RateLimited { .. }
    )
}

/// The report line for one category, with the versions the plan decided on.
fn category_report(entry: &IndexEntry, plan: &Plan, action: CategoryAction) -> CategoryReport {
    CategoryReport {
        category_id: entry.category_id,
        pak: entry.pak.clone(),
        action,
        cache_version: plan.cache_version,
        source_version: plan.source_version,
        served_version: entry.version,
        changed: entry.changed,
        bytes: 0,
    }
}

/// The state of one run, from its first step to its report.
struct Run<'a> {
    fetcher: &'a Fetcher,
    deadline: Instant,
    cancel: &'a CancellationToken,
    progress: &'a (dyn Fn(Progress) + Send + Sync),
    started: Instant,
    /// The step in progress, which a failure names.
    step: Step,
    /// The category in progress and its PAK file name.
    place: Option<(u32, String)>,
    /// Transient-refusal waits taken so far.
    waits: u32,
    /// Index fetches made so far.
    attempts: u32,
    /// Restarts from the index so far (at most one).
    restarts: u32,
    /// Body bytes read so far.
    bytes: u64,
    categories_done: u32,
    categories_total: u32,
    bytes_total: u64,
    /// Categories the plan wants written, not yet committed.
    pending: Vec<CategoryReport>,
    /// Categories the report lists.
    categories: Vec<CategoryReport>,
}

/// Runs the presync against `fetcher`, writing the stale categories of the
/// cache under `firesky` from `source_dir`. It returns at once with a
/// [`Report`]: a failure, a skip or a cancel never leaves a temp behind or a
/// PAK half-written.
pub async fn run(
    fetcher: &Fetcher,
    firesky: &Path,
    source_dir: &Path,
    deadline: Instant,
    cancel: &CancellationToken,
    progress: &(dyn Fn(Progress) + Send + Sync),
) -> Report {
    let mut run = Run {
        fetcher,
        deadline,
        cancel,
        progress,
        started: Instant::now(),
        step: Step::Prepare,
        place: None,
        waits: 0,
        attempts: 0,
        restarts: 0,
        bytes: 0,
        categories_done: 0,
        categories_total: 0,
        bytes_total: 0,
        pending: Vec::new(),
        categories: Vec::new(),
    };
    let result = run.execute(firesky, source_dir).await;
    run.finish(result)
}

impl Run<'_> {
    /// The steps of the run's `run` order, up to the outcome.
    async fn execute(&mut self, firesky: &Path, source_dir: &Path) -> Result<Outcome, Halt> {
        self.emit(Stage::Checking);
        apply::ensure_not_linked(firesky)
            .map_err(|error| Halt::Failed(Box::new(Failure::new(Step::Prepare, error.reason()))))?;
        let cache = apply::cache_dir(firesky);
        apply::sweep_temps(&cache);
        loop {
            let index = self.index().await?;
            let writes = self.plan(&index, &cache, source_dir)?;
            if writes.is_empty() {
                return Ok(Outcome::UpToDate);
            }
            match self.download_and_stage(&writes, &cache, source_dir).await {
                Ok(staged) => {
                    self.commit(staged, &cache).await?;
                    return Ok(Outcome::Synced);
                }
                Err(Halt::Restart) => self.restarts += 1,
                Err(halt) => return Err(halt),
            }
        }
    }

    /// Fetches the index, waiting out transient refusals within the limits.
    async fn index(&mut self) -> Result<Index, Halt> {
        self.step = Step::Index;
        self.place = None;
        let fetcher = self.fetcher;
        loop {
            self.attempts += 1;
            match self.guard(fetcher.index()).await? {
                Ok(index) => return Ok(index),
                Err(error @ (FetchError::ServerUnsupported | FetchError::Disabled)) => {
                    return Err(Halt::Skipped(error.reason()));
                }
                Err(error) if is_transient(&error) => self.backoff(&error).await?,
                Err(error) => return Err(Halt::Failed(Box::new(self.fetch_failure(&error)))),
            }
        }
    }

    /// Compares every entry with the cache and the source. Any source newer
    /// than the served version fails here, before a download. Returns the
    /// categories to write, ascending by id.
    fn plan(
        &mut self,
        index: &Index,
        cache: &Path,
        source_dir: &Path,
    ) -> Result<Vec<IndexEntry>, Halt> {
        self.step = Step::Plan;
        self.place = None;
        self.categories.clear();
        self.pending.clear();
        let mut entries = index.categories.clone();
        entries.sort_by_key(|entry| entry.category_id);
        let mut writes = Vec::new();
        for entry in entries {
            let plan = apply::plan(&entry, cache, source_dir);
            match plan.action {
                Action::SourceNewer => {
                    self.place = Some((entry.category_id, entry.pak.clone()));
                    let mut failure = self.failure("source_newer");
                    failure.source_version = plan.source_version;
                    failure.served_version = Some(entry.version);
                    return Err(Halt::Failed(Box::new(failure)));
                }
                Action::UpToDate => {
                    self.categories
                        .push(category_report(&entry, &plan, CategoryAction::UpToDate))
                }
                Action::Write => {
                    self.pending
                        .push(category_report(&entry, &plan, CategoryAction::Written));
                    writes.push(entry);
                }
            }
        }
        self.categories_done = 0;
        self.categories_total = u32::try_from(writes.len()).unwrap_or(u32::MAX);
        self.bytes_total = writes.iter().map(|entry| entry.body_bytes).sum();
        Ok(writes)
    }

    /// Fetches and stages every category to write, in ascending id order.
    async fn download_and_stage(
        &mut self,
        writes: &[IndexEntry],
        cache: &Path,
        source_dir: &Path,
    ) -> Result<Vec<Staged>, Halt> {
        let mut staged = Vec::with_capacity(writes.len());
        for entry in writes {
            self.place = Some((entry.category_id, entry.pak.clone()));
            self.step = Step::Download;
            self.emit(Stage::Downloading);
            let (body, bytes) = self.body(entry).await?;
            self.bytes += bytes;
            self.set_bytes(entry.category_id, bytes);
            self.step = Step::Stage;
            self.emit(Stage::Writing);
            staged.push(self.stage(entry, body, cache, source_dir).await?);
            self.categories_done += 1;
        }
        Ok(staged)
    }

    /// Fetches one category body. A version change restarts the run once;
    /// a second one fails.
    async fn body(&mut self, entry: &IndexEntry) -> Result<(CategoryBody, u64), Halt> {
        let fetcher = self.fetcher;
        loop {
            match self
                .guard(fetcher.body(entry.category_id, entry.version))
                .await?
            {
                Ok(found) => return Ok(found),
                Err(error) if is_transient(&error) => self.backoff(&error).await?,
                Err(FetchError::VersionChanged) if self.restarts == 0 => {
                    return Err(Halt::Restart);
                }
                Err(error) => return Err(Halt::Failed(Box::new(self.fetch_failure(&error)))),
            }
        }
    }

    /// Composes and writes one category's temp on a blocking thread. The
    /// check first means a cancel from the `Writing` callback starts no task.
    async fn stage(
        &mut self,
        entry: &IndexEntry,
        body: CategoryBody,
        cache: &Path,
        source_dir: &Path,
    ) -> Result<Staged, Halt> {
        self.check()?;
        let (id, pak) = (entry.category_id, entry.pak.clone());
        let (cache, source) = (cache.to_path_buf(), source_dir.to_path_buf());
        let task =
            tokio::task::spawn_blocking(move || apply::stage(id, &body, &pak, &cache, &source));
        match self.guard(task).await? {
            Ok(Ok(staged)) => Ok(staged),
            Ok(Err(error)) => Err(Halt::Failed(Box::new(self.apply_failure(&error)))),
            Err(_) => Err(Halt::Failed(Box::new(self.failure("write_failed")))),
        }
    }

    /// Renames the staged temps over their PAKs. It is not raced against the
    /// deadline or the cancel: a commit that has begun finishes (D-LP23).
    async fn commit(&mut self, staged: Vec<Staged>, cache: &Path) -> Result<(), Halt> {
        self.step = Step::Commit;
        self.place = None;
        self.check()?;
        self.emit(Stage::Committing);
        let cache = cache.to_path_buf();
        match tokio::task::spawn_blocking(move || apply::commit(staged, &cache)).await {
            Ok(Ok(_)) => {
                self.categories.append(&mut self.pending);
                Ok(())
            }
            Ok(Err(error)) => {
                // The renames made before the failure are whole and new, in
                // ascending order; the failing category is marked Failed.
                let pending = std::mem::take(&mut self.pending);
                for (position, mut report) in pending.into_iter().enumerate() {
                    if position < error.committed as usize {
                        self.categories.push(report);
                    } else if report.category_id == error.category_id {
                        report.action = CategoryAction::Failed;
                        self.categories.push(report);
                    }
                }
                self.place = Some((error.category_id, error.pak.clone()));
                Err(Halt::Failed(Box::new(self.apply_failure(&error.error))))
            }
            Err(_) => Err(Halt::Failed(Box::new(self.failure("write_failed")))),
        }
    }

    /// Waits out one transient refusal, when `Retry-After` (503 and 429 default
    /// to 5 s) is within the limits, fits before the deadline and the run has
    /// waits left. Otherwise the refusal fails the step.
    async fn backoff(&mut self, error: &FetchError) -> Result<(), Halt> {
        let wait_s = error.retry_after_s().unwrap_or(DEFAULT_RETRY_AFTER_S);
        let wait = Duration::from_secs(u64::from(wait_s));
        let fits = Instant::now() + wait < self.deadline;
        if wait > MAX_WAIT || self.waits >= MAX_WAITS || !fits {
            return Err(Halt::Failed(Box::new(self.fetch_failure(error))));
        }
        self.waits += 1;
        self.guard(tokio::time::sleep(wait)).await
    }

    /// Runs `future` under the deadline and the cancel, at the step in progress.
    async fn guard<T>(&self, future: impl Future<Output = T>) -> Result<T, Halt> {
        guarded(future, self.deadline, self.cancel)
            .await
            .map_err(|stop| self.stopped(stop))
    }

    /// Checks the cancel and the deadline at a step boundary.
    fn check(&self) -> Result<(), Halt> {
        if self.cancel.is_cancelled() {
            return Err(self.stopped(Stop::Cancelled));
        }
        if Instant::now() >= self.deadline {
            return Err(self.stopped(Stop::Deadline));
        }
        Ok(())
    }

    /// The halt for a stop: `deadline` with the run's limit, or `cancelled`.
    fn stopped(&self, stop: Stop) -> Halt {
        match stop {
            Stop::Cancelled => Halt::Cancelled(Box::new(self.failure("cancelled"))),
            Stop::Deadline => {
                let mut failure = self.failure("deadline");
                failure.deadline_s = Some(self.deadline_s());
                Halt::Failed(Box::new(failure))
            }
        }
    }

    /// The whole run's limit in whole seconds, rounded up.
    fn deadline_s(&self) -> u32 {
        let total = self.deadline.saturating_duration_since(self.started);
        let whole = u32::try_from(total.as_secs()).unwrap_or(u32::MAX);
        whole.saturating_add(u32::from(total.subsec_nanos() > 0))
    }

    /// A failure at the step in progress, with its category when there is one.
    fn failure(&self, reason: &'static str) -> Failure {
        let mut failure = Failure::new(self.step, reason);
        if let Some((category_id, pak)) = &self.place {
            failure.category_id = Some(*category_id);
            failure.pak = Some(pak.clone());
        }
        failure
    }

    /// A failure from a fetch error, with its HTTP, I/O and `Retry-After` fields.
    fn fetch_failure(&self, error: &FetchError) -> Failure {
        let mut failure = self.failure(error.reason());
        failure.http_status = error.http_status();
        failure.io_kind = error.io().map(|detail| detail.kind);
        failure.os_code = error.io().and_then(|detail| detail.os_code);
        failure.retry_after_s = error.retry_after_s();
        failure
    }

    /// A failure from an apply error, with its I/O fields.
    fn apply_failure(&self, error: &ApplyError) -> Failure {
        let mut failure = self.failure(error.reason());
        failure.io_kind = error.io().map(|detail| detail.kind);
        failure.os_code = error.io().and_then(|detail| detail.os_code);
        failure
    }

    /// Records a category's body size on its pending line.
    fn set_bytes(&mut self, category_id: u32, bytes: u64) {
        if let Some(report) = self
            .pending
            .iter_mut()
            .find(|item| item.category_id == category_id)
        {
            report.bytes = bytes;
        }
    }

    /// Reports the run's position to the progress callback.
    fn emit(&self, stage: Stage) {
        (self.progress)(Progress {
            stage,
            categories_done: self.categories_done,
            categories_total: self.categories_total,
            bytes_done: self.bytes,
            bytes_total: self.bytes_total,
        });
    }

    /// Ends the run: a failed or cancelled category is listed as Failed, and
    /// the totals and the sorted categories go into the report.
    fn finish(mut self, result: Result<Outcome, Halt>) -> Report {
        let mut report = match result {
            Ok(Outcome::Synced) => Report::bare(Outcome::Synced, "synced", None),
            Ok(outcome) => Report::bare(outcome, "versions_match", None),
            Err(Halt::Skipped(reason)) => Report::skipped(reason),
            Err(Halt::Failed(failure)) => self.failed(*failure, Outcome::Failed),
            Err(Halt::Cancelled(failure)) => self.failed(*failure, Outcome::Cancelled),
            Err(Halt::Restart) => Report::failed(Failure::new(Step::Download, "version_changed")),
        };
        self.categories.sort_by_key(|item| item.category_id);
        report.categories = self.categories;
        report.bytes_downloaded = self.bytes;
        report.duration_ms = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        report.attempts = self.attempts;
        report
    }

    /// The report of a failed or cancelled run. The category that failed is
    /// listed as Failed; the others that were not committed are not listed.
    fn failed(&mut self, failure: Failure, outcome: Outcome) -> Report {
        if let Some(category_id) = failure.category_id {
            let position = self
                .pending
                .iter()
                .position(|item| item.category_id == category_id);
            if let Some(position) = position {
                let mut report = self.pending.remove(position);
                report.action = CategoryAction::Failed;
                self.categories.push(report);
            }
        }
        let mut report = Report::failed(failure);
        report.outcome = outcome;
        report
    }
}

#[cfg(test)]
#[path = "tests/run.rs"]
mod tests;
