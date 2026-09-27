//! Deciding whether a dropped method is ours, and decoding it.
//!
//! Portable: it reads client memory only through
//! [`MemoryReader`](crate::memory::MemoryReader) and the stream only
//! through [`ByteSource`], so the tests run it against fake memory and a
//! byte slice.

use cimmeria_patch_wire::black_market::{ClientCall, ClientMethod};
use cimmeria_patch_wire::{ByteSource, DecodeError};

use crate::addresses::{
    ENTITY_ID, GAME_ENTITY_MANAGER, GEM_LOCAL_PLAYER_ID, METHOD_NAME_BUFFER, METHOD_NAME_CAPACITY,
    METHOD_NAME_LEN, MSVC_SSO_CAPACITY,
};
use crate::memory::MemoryReader;

/// The name stored in the `MethodDescription` at `md`, if it is at most
/// `max_len` bytes. Longer names are not read at all.
///
/// The name is an MSVC `std::string`: length at `+0x14`, capacity at
/// `+0x18`, and at `+0x04` either the characters (capacity below 16) or a
/// pointer to them. A length above the capacity means the object is not
/// what it should be, and is refused.
pub fn method_name<M: MemoryReader>(mem: &M, md: usize, max_len: usize) -> Option<Vec<u8>> {
    let len = mem.read_u32(md.checked_add(METHOD_NAME_LEN)?)?;
    let capacity = mem.read_u32(md.checked_add(METHOD_NAME_CAPACITY)?)?;
    if len == 0 || len > capacity || len as usize > max_len {
        return None;
    }
    let chars = if capacity < MSVC_SSO_CAPACITY {
        md.checked_add(METHOD_NAME_BUFFER)?
    } else {
        mem.read_ptr(md.checked_add(METHOD_NAME_BUFFER)?)?
    };
    mem.read_bytes(chars, len as usize)
}

/// The Black Market method the `MethodDescription` at `md` describes, if
/// it is one.
pub fn black_market_method<M: MemoryReader>(mem: &M, md: usize) -> Option<ClientMethod> {
    ClientMethod::from_name(&method_name(mem, md, ClientMethod::MAX_NAME_LEN)?)
}

/// The local player's entity id, once the player is in the world.
pub fn local_player_id<M: MemoryReader>(mem: &M) -> Option<u32> {
    let gem = mem.read_ptr(GAME_ENTITY_MANAGER)?;
    match mem.read_u32(gem.checked_add(GEM_LOCAL_PLAYER_ID)?)? {
        0 => None,
        id => Some(id),
    }
}

/// Whether the entity object at `entity` is the local player.
pub fn is_local_player<M: MemoryReader>(mem: &M, entity: usize) -> bool {
    if entity == 0 {
        return false;
    }
    let Some(player) = local_player_id(mem) else {
        return false;
    };
    entity
        .checked_add(ENTITY_ID)
        .and_then(|a| mem.read_u32(a))
        .is_some_and(|id| id == player)
}

/// What to do with one dropped method call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// Not a Black Market method: the client drops it as before.
    NotOurs,
    /// A Black Market method for an entity other than the local player:
    /// the client drops it as before.
    NotLocalPlayer(ClientMethod),
    /// No stream was recorded, or it could not be opened.
    NoStream(ClientMethod),
    /// The arguments did not decode.
    Malformed(ClientMethod, DecodeError),
    /// Decoded; queue it for the main thread.
    Decoded(ClientCall),
}

/// Decide about the method `md` resolved to, for `entity`, and decode it
/// from the stream `open_stream` yields if it is ours.
///
/// The stream is opened only once the name and the entity have matched,
/// and it must hold exactly the arguments: bytes left over mean this build
/// and the server disagree about the layout, and the call is refused
/// rather than delivered half-understood.
pub fn claim<M, S>(
    mem: &M,
    md: usize,
    entity: usize,
    open_stream: impl FnOnce() -> Option<S>,
) -> Claim
where
    M: MemoryReader,
    S: ByteSource,
{
    let Some(method) = black_market_method(mem, md) else {
        return Claim::NotOurs;
    };
    if !is_local_player(mem, entity) {
        return Claim::NotLocalPlayer(method);
    }
    let Some(mut stream) = open_stream() else {
        return Claim::NoStream(method);
    };
    let before = stream.remaining();
    match ClientCall::decode_from(method, &mut stream) {
        Ok(call) => match stream.remaining() {
            0 => Claim::Decoded(call),
            trailing => Claim::Malformed(
                method,
                DecodeError::TrailingBytes {
                    consumed: before.saturating_sub(trailing),
                    trailing,
                },
            ),
        },
        Err(e) => Claim::Malformed(method, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::FakeMemory;
    use cimmeria_patch_wire::black_market::{OnBMAuctionRemove, OnBMOpen};
    use cimmeria_patch_wire::SliceSource;

    const MD: usize = 0x2000_0000;
    const HEAP_CHARS: usize = 0x2100_0000;
    const GEM: usize = 0x2200_0000;
    const PLAYER: usize = 0x2300_0000;
    const OTHER: usize = 0x2400_0000;
    const PLAYER_ID: u32 = 0x0001_0203;

    /// A `MethodDescription` whose name is `name`, laid out the way MSVC
    /// does it: inline below 16 bytes of capacity, on the heap above.
    fn method_description(mem: &mut FakeMemory, name: &str) {
        let len = name.len() as u32;
        let mut buffer = [0u8; 16];
        let capacity = if name.len() < 16 {
            buffer[..name.len()].copy_from_slice(name.as_bytes());
            15
        } else {
            buffer[..4].copy_from_slice(&(HEAP_CHARS as u32).to_le_bytes());
            mem.put(HEAP_CHARS, name.as_bytes());
            31
        };
        mem.put(MD + METHOD_NAME_BUFFER, &buffer)
            .put_u32(MD + METHOD_NAME_LEN, len)
            .put_u32(MD + METHOD_NAME_CAPACITY, capacity);
    }

    /// The game entity manager, the local player, and one other entity.
    fn world(mem: &mut FakeMemory) {
        mem.put_u32(GAME_ENTITY_MANAGER, GEM as u32)
            .put_u32(GEM + GEM_LOCAL_PLAYER_ID, PLAYER_ID)
            .put_u32(PLAYER + ENTITY_ID, PLAYER_ID)
            .put_u32(OTHER + ENTITY_ID, PLAYER_ID + 1);
    }

    fn named(name: &str) -> FakeMemory {
        let mut mem = FakeMemory::default();
        method_description(&mut mem, name);
        world(&mut mem);
        mem
    }

    /// Short names live inside the object, long ones behind a pointer; all
    /// six Black Market names are recognised either way.
    #[test]
    fn every_black_market_name_is_recognised_inline_or_on_the_heap() {
        for m in ClientMethod::ALL {
            let mem = named(m.name());
            assert_eq!(black_market_method(&mem, MD), Some(m), "{}", m.name());
        }
        // The six split across both layouts.
        assert!(ClientMethod::ALL.iter().any(|m| m.name().len() < 16));
        assert!(ClientMethod::ALL.iter().any(|m| m.name().len() >= 16));
    }

    #[test]
    fn other_names_are_not_ours() {
        for name in [
            "onDialogDisplay",
            "onBMOpenX",
            "onbmopen",
            "onBMAuctionRemovE",
        ] {
            assert_eq!(black_market_method(&named(name), MD), None, "{name}");
        }
    }

    /// A name longer than any of ours is refused on its length alone.
    #[test]
    fn a_name_longer_than_any_of_ours_is_refused_on_length() {
        let mut mem = FakeMemory::default();
        mem.put_u32(MD + METHOD_NAME_LEN, 40)
            .put_u32(MD + METHOD_NAME_CAPACITY, 47)
            .put_u32(MD + METHOD_NAME_BUFFER, HEAP_CHARS as u32)
            .put(HEAP_CHARS, &[b'x'; 40]);
        assert_eq!(method_name(&mem, MD, 64).map(|n| n.len()), Some(40));
        assert_eq!(method_name(&mem, MD, ClientMethod::MAX_NAME_LEN), None);
    }

    #[test]
    fn inconsistent_or_unreadable_strings_are_refused() {
        // Length above capacity.
        let mut mem = FakeMemory::default();
        mem.put(MD + METHOD_NAME_BUFFER, b"onBMOpen\0\0\0\0\0\0\0\0")
            .put_u32(MD + METHOD_NAME_LEN, 8)
            .put_u32(MD + METHOD_NAME_CAPACITY, 4);
        assert_eq!(black_market_method(&mem, MD), None);
        // Heap layout with a null pointer.
        let mut mem = FakeMemory::default();
        mem.put_u32(MD + METHOD_NAME_BUFFER, 0)
            .put_u32(MD + METHOD_NAME_LEN, 17)
            .put_u32(MD + METHOD_NAME_CAPACITY, 31);
        assert_eq!(black_market_method(&mem, MD), None);
        // Nothing mapped at all.
        assert_eq!(black_market_method(&FakeMemory::default(), MD), None);
    }

    #[test]
    fn local_player_check() {
        let mut mem = FakeMemory::default();
        world(&mut mem);
        assert!(is_local_player(&mem, PLAYER));
        assert!(!is_local_player(&mem, OTHER));
        assert!(!is_local_player(&mem, 0));
        assert!(!is_local_player(&mem, 0x2500_0000), "unmapped entity");
    }

    /// Before the world loads the manager or its player id is still zero:
    /// nothing is the local player, not even an entity whose id is zero.
    #[test]
    fn no_local_player_before_world_entry() {
        let mut mem = FakeMemory::default();
        mem.put_u32(OTHER + ENTITY_ID, 0);
        assert!(!is_local_player(&mem, OTHER), "manager unmapped");
        mem.put_u32(GAME_ENTITY_MANAGER, 0);
        assert!(!is_local_player(&mem, OTHER), "manager null");
        let mut mem = FakeMemory::default();
        mem.put_u32(GAME_ENTITY_MANAGER, GEM as u32)
            .put_u32(GEM + GEM_LOCAL_PLAYER_ID, 0)
            .put_u32(OTHER + ENTITY_ID, 0);
        assert!(!is_local_player(&mem, OTHER), "player id not set");
    }

    fn stream<'a>(bytes: &'a [u8]) -> impl FnOnce() -> Option<SliceSource<'a>> + 'a {
        move || Some(SliceSource::new(bytes))
    }

    #[test]
    fn a_local_player_call_is_decoded() {
        let mem = named("onBMOpen");
        assert_eq!(
            claim(&mem, MD, PLAYER, stream(&[0x2A, 0, 0, 0])),
            Claim::Decoded(ClientCall::Open(OnBMOpen { entity_id: 42 }))
        );
        let mem = named("onBMAuctionRemove");
        assert_eq!(
            claim(&mem, MD, PLAYER, stream(&[7, 0, 0, 0])),
            Claim::Decoded(ClientCall::AuctionRemove(OnBMAuctionRemove {
                sequence_id: 7
            }))
        );
    }

    /// Another method, or another entity, is left alone, and its stream is
    /// never opened, so the client's own handling is untouched.
    #[test]
    fn calls_that_are_not_ours_never_open_the_stream() {
        let never = || -> Option<SliceSource<'static>> { panic!("stream opened") };
        assert_eq!(
            claim(&named("onDialogDisplay"), MD, PLAYER, never),
            Claim::NotOurs
        );
        assert_eq!(
            claim(&named("onBMOpen"), MD, OTHER, never),
            Claim::NotLocalPlayer(ClientMethod::OnBMOpen)
        );
    }

    #[test]
    fn a_missing_stream_is_reported() {
        let none = || -> Option<SliceSource<'static>> { None };
        assert_eq!(
            claim(&named("onBMOpen"), MD, PLAYER, none),
            Claim::NoStream(ClientMethod::OnBMOpen)
        );
    }

    #[test]
    fn truncated_or_oversized_arguments_are_malformed() {
        let mem = named("onBMError");
        assert!(matches!(
            claim(&mem, MD, PLAYER, stream(&[1, 0])),
            Claim::Malformed(ClientMethod::OnBMError, DecodeError::Truncated { .. })
        ));
        assert!(matches!(
            claim(&mem, MD, PLAYER, stream(&[1, 0, 0, 0, 9])),
            Claim::Malformed(
                ClientMethod::OnBMError,
                DecodeError::TrailingBytes {
                    consumed: 4,
                    trailing: 1
                }
            )
        ));
    }
}
