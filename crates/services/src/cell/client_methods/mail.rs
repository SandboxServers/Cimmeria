//! SGWMailManager interface ClientMethods (indices 76–79).

/// Mail header list (inbox/archive).
pub const ON_MAIL_HEADER_INFO: u16 = 76;
/// Remove a mail header from the list.
pub const ON_MAIL_HEADER_REMOVE: u16 = 77;
/// Mail body text loaded.
pub const ON_MAIL_READ: u16 = 78;
/// Result of sending mail (success/failure + failed recipients).
pub const SEND_MAIL_RESULT: u16 = 79;
