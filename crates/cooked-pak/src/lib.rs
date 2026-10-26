//! Cooked-data PAK reader, writer and category digest for the launcher's
//! cache presync.
//!
//! A cooked PAK is an ordinary ZIP: one `_<key>` entry per record, plus a
//! `MetaData` entry holding the category version as 4 bytes LE. The client
//! reads its own `data/cache/*.pak` copies this way, and the server reads the
//! same layout through `cimmeria-resources`. This crate is the one shared
//! reader, writer and digest for the launcher and the admin tooling.
//! `catalog` is the index and the `CPS1` body codec, `compose` layers a body's changed records over a stock PAK, and `decide` picks what to do with one category.
//!
//! No CME bytes live in this crate. Its tests build their PAKs in memory
//! from made-up entries.

pub mod catalog;
pub mod compose;
pub mod decide;
pub mod digest;
pub mod pak;
