use serde::{Deserialize, Serialize};

/// Chat channel types available in the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    /// Local/say chat visible within proximity.
    Say,
    /// Shout/yell visible to the whole zone.
    Shout,
    /// Private message to a specific player.
    Tell,
    /// Group/party chat.
    Group,
    /// Guild chat.
    Guild,
    /// Global trade chat.
    Trade,
    /// System messages (server announcements, etc.).
    System,
}

/// A chat message ready to be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub channel: ChatChannel,
    pub sender_name: String,
    pub sender_entity_id: i32,
    pub target_name: Option<String>,
    pub content: String,
}

/// Send a chat message, routing it to the appropriate recipients based on channel.
pub fn send_chat(message: ChatMessage) {
    tracing::debug!(
        channel = ?message.channel,
        sender = %message.sender_name,
        "Chat message dispatched"
    );
    todo!("send_chat - route to Mercury broadcast/unicast based on channel")
}

/// Check if a message should be filtered (profanity, spam, etc.).
pub fn filter_message(_content: &str) -> bool {
    // Placeholder: no filtering yet
    false
}
