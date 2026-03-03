//! Mercury message ID registry.
//!
//! These are the known application-level message type IDs that ride on top
//! of the Mercury transport layer. Each message ID corresponds to a specific
//! handler on the receiving service.
//!
//! **NOTE:** These values are placeholders pending full reverse-engineering
//! mapping from Ghidra analysis of the original client and server binaries.
//! The actual message IDs will be updated as wire captures and disassembly
//! confirm the real values. See `docs/reverse-engineering/findings/` for
//! ongoing RE documentation.

/// Mercury message type identifiers.
///
/// These are transmitted as `u8` values in bundle messages and Unified
/// frames. The numeric values must match the C++ `Mercury::MessageID`
/// enum exactly for wire compatibility.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MsgId {
    /// Client login request (client -> AuthenticationServer).
    LoginRequest = 1,

    /// Login response with session token (AuthenticationServer -> client).
    LoginResponse = 2,

    /// Create a new entity on the receiving service.
    CreateEntity = 3,

    /// Update properties on an existing entity.
    UpdateEntity = 4,

    /// Destroy an entity on the receiving service.
    DestroyEntity = 5,

    /// Position/orientation update for a spatial entity.
    PositionUpdate = 6,

    /// Remote procedure call on an entity method.
    RpcCall = 7,

    /// Keepalive ping/pong for idle channel maintenance.
    Keepalive = 8,
}

impl MsgId {
    /// Convert a raw u8 to a `MsgId`, if it is a known value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::LoginRequest),
            2 => Some(Self::LoginResponse),
            3 => Some(Self::CreateEntity),
            4 => Some(Self::UpdateEntity),
            5 => Some(Self::DestroyEntity),
            6 => Some(Self::PositionUpdate),
            7 => Some(Self::RpcCall),
            8 => Some(Self::Keepalive),
            _ => None,
        }
    }

    /// Return the raw u8 value.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl std::fmt::Display for MsgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoginRequest => write!(f, "LoginRequest"),
            Self::LoginResponse => write!(f, "LoginResponse"),
            Self::CreateEntity => write!(f, "CreateEntity"),
            Self::UpdateEntity => write!(f, "UpdateEntity"),
            Self::DestroyEntity => write!(f, "DestroyEntity"),
            Self::PositionUpdate => write!(f, "PositionUpdate"),
            Self::RpcCall => write!(f, "RpcCall"),
            Self::Keepalive => write!(f, "Keepalive"),
        }
    }
}

impl TryFrom<u8> for MsgId {
    type Error = cimmeria_common::CimmeriaError;

    fn try_from(value: u8) -> cimmeria_common::Result<Self> {
        Self::from_u8(value).ok_or(cimmeria_common::CimmeriaError::InvalidMessageId(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_ids() {
        let ids = [
            MsgId::LoginRequest,
            MsgId::LoginResponse,
            MsgId::CreateEntity,
            MsgId::UpdateEntity,
            MsgId::DestroyEntity,
            MsgId::PositionUpdate,
            MsgId::RpcCall,
            MsgId::Keepalive,
        ];

        for id in &ids {
            let raw = id.as_u8();
            let recovered = MsgId::from_u8(raw).unwrap();
            assert_eq!(*id, recovered);
        }
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(MsgId::from_u8(0).is_none());
        assert!(MsgId::from_u8(255).is_none());
    }

    #[test]
    fn try_from_u8() {
        let id: MsgId = 1u8.try_into().unwrap();
        assert_eq!(id, MsgId::LoginRequest);

        let err: cimmeria_common::Result<MsgId> = 99u8.try_into();
        assert!(err.is_err());
    }

    #[test]
    fn display_formatting() {
        assert_eq!(MsgId::LoginRequest.to_string(), "LoginRequest");
        assert_eq!(MsgId::Keepalive.to_string(), "Keepalive");
    }
}
