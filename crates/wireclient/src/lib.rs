//! # cimmeria-wireclient
//!
//! Headless wire-level test client for the Cimmeria SGW server emulator.
//!
//! Wireclient is the **Tier 3** end-to-end test surface above
//! [`cimmeria_mercury`]'s Tier 1 byte-fan-out fake (`test_transport`) and
//! Tier 2 paired-channel loopback (`test_harness`). Where those tiers exercise
//! the wire layer in isolation, wireclient drives the *full* protocol the way
//! the original Flash client would have:
//!
//! 1. POST `/SGWLogin/UserAuth` with credentials → receive `SID` cookie + shard list.
//! 2. POST `/SGWLogin/ServerSelection` with the cookie → receive a 32-byte
//!    AES-256 session key, a 20-character ticket, and the BaseApp `IP:port`.
//! 3. Open a UDP socket to the BaseApp, send `baseAppLogin` (msg `0x00`,
//!    `WORD_LENGTH = 25`), receive the encrypted `BASEMSG_REPLY_MESSAGE` +
//!    `tickSync` bundle, and transition the [`MercuryEncryption`] state.
//! 4. Send `enableEntities`, walk the world-entry handshake, and from there
//!    drive whatever the test script demands.
//!
//! ## Why
//!
//! Issue #281 spells out the gap this layer closes: ~half of the integration
//! bugs caught in PR reviews #131+ have lived in the slice between *"server
//! accepts a call"* and *"a real client could have sent that call."* Every
//! existing test type injects events at or below the dispatcher; nothing
//! proves the wire path leading up to the event would have fired.
//!
//! Wireclient enforces the client-authoritative invariants — equipped weapon,
//! ammo, cooldown, range, LOS — *client-side* before sending, so it can only
//! send sequences a real client could have sent. That makes it a stronger
//! oracle than any server-side test for shape regressions.
//!
//! ## Trace-driven testing
//!
//! Wireclient is paired with the [`session_trace`] module, which loads a
//! decrypted capture produced by `tools/pcap_dissect.py` against a recorded
//! `.pcap` + AES `keys.txt`. The Castle Cellblock corpus
//! (`docs/architecture/wireclient.md` § Test corpora) is the canonical
//! ground-truth fixture: every client→server packet the real client sent
//! during a full 32-minute Castle Cellblock playthrough.
//!
//! ## Scope of this crate today (Phase 1)
//!
//! - [`auth`] — SOAP Phase 1 + Phase 2 client, the same shape `login_smoke`
//!   exercises server-side.
//! - [`handshake`] — Mercury phase-3 `baseAppLogin` builder + reply parser
//!   for the three packets that establish encryption.
//! - [`session_trace`] — deserialisable trace types matching the JSONL output
//!   of `tools/pcap_dissect.py --json`.
//! - [`Client`] — top-level driver; today it stitches auth → handshake;
//!   future phases add the entity mirror, step driver, combat enforcement,
//!   and the Castle Cellblock script.
//!
//! Phases 2–7 (entity mirror, step driver, combat, minigame force-victory
//! hook, nextest profile, CI) are tracked in issue #281 and arrive in
//! follow-up PRs.

pub mod auth;
pub mod error;
pub mod handshake;
pub mod session_trace;

mod client;

pub use client::Client;
pub use error::{Error, Result};
