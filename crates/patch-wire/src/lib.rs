//! # cimmeria-patch-wire
//!
//! Codecs for the entity-method arguments that the server and the injected
//! client-patch DLL (`cimmeria-client-patches`) both have to agree on. The
//! first family is the Black Market ([`black_market`]): the six
//! `SGWBlackMarketManager` client methods the stock client drops on the
//! floor, and the six cell methods it never learned to send.
//!
//! ## Why a separate crate
//!
//! The DLL runs inside the 2009 32-bit `SGW.exe`, so it cannot link
//! `cimmeria-wire` (tokio, the Mercury transport, sqlx through the
//! workspace-hack). This crate is plain `std` with no dependencies at all,
//! so both sides can link the same encoder and decoder. An argument-order
//! mismatch between them then fails a unit test here, instead of showing up
//! as garbage in the auction window.
//!
//! ## What a payload is
//!
//! Every type here encodes the method's **arguments only**, in `.def`
//! order: no Mercury message id, no word-length field, no entity id and no
//! extended sub-index byte. Framing stays with whoever sends the message
//! (the server's Mercury layer, or `ServerConnection::startEntityMessage` in
//! the client).
//!
//! ## Wire conventions
//!
//! CME's BigWorld prefixes an `ARRAY` with a little-endian `u32` element
//! count and a narrow `STRING` with a little-endian `u32` byte length
//! followed by the bytes, which both sides treat as UTF-8. Integers are
//! little-endian. There is no padding.
//!
//! ## Decoding is total
//!
//! A decoder never panics and never allocates on the say-so of the input:
//! a truncated, oversized or malformed payload returns a [`DecodeError`].
//! Every array count and string length is checked against a documented cap
//! (see [`black_market`]) and against the bytes actually left before
//! anything is allocated. Decoding works over any [`ByteSource`], so the
//! DLL can pull bytes straight from the client's `BinaryIStream` and the
//! server can decode a `&[u8]` with [`Decode::decode`].

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]

pub mod black_market;
mod codec;
mod source;

pub use codec::{Decode, DecodeError, Encode, EncodeError, UnknownEnumValue};
pub use source::{ByteSource, SliceSource, SourceExhausted};
