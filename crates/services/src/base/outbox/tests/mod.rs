//! Outbox tests, split by what they exercise:
//!
//!   * [`payload`] — pure serde shape + persistence-string contracts.
//!   * [`row_to_message`] — the row → BaseToCellMsg dispatch decoder.
//!   * [`live_db`] — end-to-end SQL paths against a real Postgres,
//!     gated on `DATABASE_URL`. These also live here (not in a
//!     separate integration crate) so they can poke `pub(crate)`
//!     internals like `drain_undelivered` and `OutboxRow`.

mod live_db;
mod payload;
mod row_to_message;
