//! Communicator interface ClientMethods (indices 27–33).

/// System communication (localized string ID with tokens).
pub const ON_SYSTEM_COMMUNICATION: u16 = 27;
/// Player chat message (speaker, channel, text).
pub const ON_PLAYER_COMMUNICATION: u16 = 28;
/// Localized chat message (with string tokens).
pub const ON_LOCALIZED_COMMUNICATION: u16 = 29;
/// Tell/whisper sent confirmation.
pub const ON_TELL_SENT: u16 = 30;
/// Joined a chat channel.
pub const ON_CHAT_JOINED: u16 = 31;
/// Left a chat channel.
pub const ON_CHAT_LEFT: u16 = 32;
/// Player nickname changed.
pub const ON_NICK_CHANGED: u16 = 33;
