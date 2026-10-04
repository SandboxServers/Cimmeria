//! A bounded download-only updater. This module cannot execute or install bytes.
mod policy;
mod transport;
use super::{atomic, ensure_regular_or_absent, read, DesktopState, StorageError, MAX_REVISION};
pub use policy::Config;
use serde::{Deserialize, Serialize};
use std::io::Read;
pub use transport::{check, download};
use uuid::Uuid;

const RECORD: &str = "launcher-update.json";
const ARTIFACT: &str = "launcher-update.verified";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Error {
    Disabled,
    Busy,
    StaleRevision,
    StaleOffer,
    Policy,
    Feed,
    Platform,
    NotNewer,
    Signature,
    SignedVersion,
    Size,
    Transport,
    Timeout,
    Interrupted,
    Storage(StorageError),
}
impl From<StorageError> for Error {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Disabled,
    Idle,
    Checking,
    Available,
    UpToDate,
    Downloading,
    Verifying,
    Ready,
    Failed,
}
impl Phase {
    fn busy(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Verifying)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    id: Uuid,
    version: String,
    notes: String,
    url: String,
    signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    revision: u64,
    phase: Phase,
    owner: Option<Uuid>,
    offer: Option<Offer>,
    failure: Option<Error>,
}
impl Default for Record {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            phase: Phase::Idle,
            owner: None,
            offer: None,
            failure: None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct DisplayOffer {
    pub id: Uuid,
    pub version: String,
    pub notes: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub revision: u64,
    pub operation_revision: u64,
    pub phase: Phase,
    pub offer: Option<DisplayOffer>,
    pub failure: Option<Error>,
    pub requires_reopen: bool,
}
/// Native capability; the renderer cannot supply or reconstruct worker plans.
pub struct Ticket {
    owner: Uuid,
    revision: u64,
    offer: Option<Offer>,
}
impl Ticket {
    pub fn offer(&self) -> Result<&Offer, Error> {
        self.offer.as_ref().ok_or(Error::StaleOffer)
    }
}
impl DesktopState {
    fn update_record(&self) -> Result<Record, StorageError> {
        let record: Record = read(&self.state_root().join(RECORD))?.unwrap_or_default();
        if record.schema_version != 1 {
            return Err(StorageError::UnsupportedSchema);
        }
        if record.revision > MAX_REVISION
            || record.phase == Phase::Disabled
            || (record.phase.busy() != record.owner.is_some())
            || (matches!(
                record.phase,
                Phase::Available | Phase::Downloading | Phase::Verifying | Phase::Ready
            ) && record.offer.is_none())
        {
            return Err(StorageError::Corrupt);
        }
        Ok(record)
    }
    fn save_update(&mut self, mut record: Record) -> Result<Record, StorageError> {
        record.revision = record
            .revision
            .checked_add(1)
            .filter(|v| *v <= MAX_REVISION)
            .ok_or(StorageError::Corrupt)?;
        let result = atomic::write(self.state_root(), RECORD, &record);
        self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
        result?;
        Ok(record)
    }
    /// Call from all mutation admission paths under the same DesktopState mutex.
    pub fn ensure_updater_idle(&self) -> Result<(), StorageError> {
        if self.update_record()?.phase.busy() {
            return Err(StorageError::Busy);
        }
        Ok(())
    }
    /// Called once by DesktopState::open, never by a live worker's polling path.
    pub(crate) fn recover_launcher_update(&mut self) -> Result<(), StorageError> {
        let mut record = self.update_record()?;
        if record.phase.busy() {
            record.phase = Phase::Failed;
            record.owner = None;
            record.failure = Some(Error::Interrupted);
            self.save_update(record)?;
        }
        Ok(())
    }
    pub fn launcher_update_snapshot(&mut self, config: Option<&Config>) -> Result<Snapshot, Error> {
        let mut record = self.update_record()?;
        if let Some(config) = config {
            if record.phase == Phase::Ready && !self.requires_reopen() {
                let verified = self.staged_update_bytes().and_then(|bytes| {
                    config.verify(record.offer.as_ref().ok_or(Error::StaleOffer)?, &bytes)
                });
                if let Err(error) = verified {
                    record.phase = Phase::Failed;
                    record.failure = Some(error);
                    record = self.save_update(record)?;
                }
            }
        }
        Ok(Snapshot {
            schema_version: 1,
            revision: record.revision,
            operation_revision: self.operations.snapshot().revision,
            phase: if config.is_none() {
                Phase::Disabled
            } else {
                record.phase
            },
            offer: if config.is_none() {
                None
            } else {
                record.offer.map(|offer| DisplayOffer {
                    id: offer.id,
                    version: offer.version,
                    notes: offer.notes,
                })
            },
            failure: if config.is_none() {
                Some(Error::Disabled)
            } else {
                record.failure
            },
            requires_reopen: self.requires_reopen(),
        })
    }
    fn update_admission(&self, revision: u64, operation_revision: u64) -> Result<Record, Error> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let record = self.update_record()?;
        if record.phase.busy()
            || self
                .operations
                .snapshot()
                .operation
                .as_ref()
                .is_some_and(|op| !op.state.terminal())
        {
            return Err(Error::Busy);
        }
        if record.revision != revision || self.operations.snapshot().revision != operation_revision
        {
            return Err(Error::StaleRevision);
        }
        Ok(record)
    }
    pub fn begin_launcher_update_check(
        &mut self,
        config: Option<&Config>,
        revision: u64,
        operation_revision: u64,
    ) -> Result<Ticket, Error> {
        let config = config.ok_or(Error::Disabled)?;
        config.allow(&config.endpoint)?;
        let mut record = self.update_admission(revision, operation_revision)?;
        record.phase = Phase::Checking;
        record.owner = Some(Uuid::new_v4());
        record.offer = None;
        record.failure = None;
        let record = self.save_update(record)?;
        Ok(Ticket {
            owner: record.owner.unwrap(),
            revision: record.revision,
            offer: None,
        })
    }
    pub fn begin_launcher_update_prepare(
        &mut self,
        config: Option<&Config>,
        offer_id: Uuid,
        revision: u64,
        operation_revision: u64,
    ) -> Result<Ticket, Error> {
        let config = config.ok_or(Error::Disabled)?;
        let mut record = self.update_admission(revision, operation_revision)?;
        let offer = record
            .offer
            .as_ref()
            .filter(|offer| offer.id == offer_id)
            .ok_or(Error::StaleOffer)?;
        if record.phase != Phase::Available {
            return Err(Error::StaleOffer);
        }
        config.newer(&offer.version)?;
        config.allow(&reqwest::Url::parse(&offer.url).map_err(|_| Error::Policy)?)?;
        record.phase = Phase::Downloading;
        record.owner = Some(Uuid::new_v4());
        record.failure = None;
        let record = self.save_update(record)?;
        Ok(Ticket {
            owner: record.owner.unwrap(),
            revision: record.revision,
            offer: record.offer,
        })
    }
    fn owned_update(&self, ticket: &Ticket) -> Result<Record, Error> {
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        let record = self.update_record()?;
        if record.owner != Some(ticket.owner)
            || record.revision != ticket.revision
            || !record.phase.busy()
        {
            return Err(Error::StaleOffer);
        }
        Ok(record)
    }
    pub fn finish_launcher_update_check(
        &mut self,
        ticket: Ticket,
        outcome: Result<Option<Offer>, Error>,
    ) -> Result<(), Error> {
        let mut record = self.owned_update(&ticket)?;
        if record.phase != Phase::Checking {
            return Err(Error::StaleOffer);
        }
        record.owner = None;
        match outcome {
            Ok(offer) => {
                record.phase = if offer.is_some() {
                    Phase::Available
                } else {
                    Phase::UpToDate
                };
                record.offer = offer;
            }
            Err(error) => {
                record.phase = Phase::Failed;
                record.failure = Some(error);
            }
        }
        self.save_update(record)?;
        Ok(())
    }
    /// Persist verifying before CPU verification; no bytes are accepted over IPC.
    pub fn mark_launcher_update_verifying(&mut self, ticket: &mut Ticket) -> Result<(), Error> {
        let mut record = self.owned_update(ticket)?;
        if record.phase != Phase::Downloading {
            return Err(Error::StaleOffer);
        }
        record.phase = Phase::Verifying;
        ticket.revision = self.save_update(record)?.revision;
        Ok(())
    }
    pub fn finish_launcher_update_prepare(
        &mut self,
        config: &Config,
        ticket: Ticket,
        outcome: Result<Vec<u8>, Error>,
    ) -> Result<(), Error> {
        let mut record = self.owned_update(&ticket)?;
        if !matches!(record.phase, Phase::Downloading | Phase::Verifying) {
            return Err(Error::StaleOffer);
        }
        let result = outcome.and_then(|bytes| {
            config.verify(ticket.offer()?, &bytes)?;
            let result =
                atomic::write_bytes(self.state_root(), ARTIFACT, &bytes, policy::MAX_ARTIFACT);
            self.preferences_uncertain |= result == Err(StorageError::PersistenceUncertain);
            result.map_err(Error::from)
        });
        record.owner = None;
        match result {
            Ok(()) => {
                record.phase = Phase::Ready;
                record.failure = None;
            }
            Err(error) => {
                record.phase = Phase::Failed;
                record.failure = Some(error);
            }
        }
        if self.requires_reopen() {
            return Err(StorageError::PersistenceUncertain.into());
        }
        self.save_update(record)?;
        Ok(())
    }
    fn staged_update_bytes(&self) -> Result<Vec<u8>, Error> {
        let path = self.state_root().join(ARTIFACT);
        ensure_regular_or_absent(&path)?;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| StorageError::Io)?
            .take(policy::MAX_ARTIFACT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| StorageError::Io)?;
        if bytes.len() > policy::MAX_ARTIFACT {
            return Err(Error::Size);
        }
        Ok(bytes)
    }
}
#[cfg(test)]
mod tests;
