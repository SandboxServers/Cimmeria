//! `EMailFlags` and `EMailResultCodes` (`entities/defs/enumerations.xml`).
//!
//! Both enums are the client's own schema. The values are pinned by a test
//! that parses `enumerations.xml` and compares each constant with the token
//! it finds there (`tests.rs`), never with a hard-coded copy of itself.

/// `EMailFlags`: bit values carried in a header's `flags` and in
/// `sendMailMessage`'s `RecipientFlags`.
pub mod flags {
    /// The mail is in the archive list, not the inbox.
    pub const MAIL_ARCHIVE: i32 = 1;
    /// Cash on delivery: the attached cash is a price, not a gift.
    pub const MAIL_COD: i32 = 2;
    pub const MAIL_TO_VAULT: i32 = 4;
    pub const MAIL_TO_TEAM: i32 = 8;
    pub const MAIL_TO_COMMAND: i32 = 16;
    pub const MAIL_TO_COMMAND_OFFICERS: i32 = 32;
    pub const MAIL_TO_COMMAND_RANK0: i32 = 64;
    pub const MAIL_TO_COMMAND_RANK1: i32 = 128;
    pub const MAIL_TO_COMMAND_RANK2: i32 = 256;
    pub const MAIL_TO_COMMAND_RANK3: i32 = 512;
    pub const MAIL_TO_COMMAND_RANK4: i32 = 1024;
    pub const MAIL_TO_COMMAND_RANK5: i32 = 2048;
    /// **Not a power of two** (audit A-12): the client's enum says 4092,
    /// where the pattern predicts 4096. 4092 overlaps bits 2 to 11, so it
    /// cannot be tested as a single bit. Kept exactly as the client ships it.
    pub const MAIL_TO_COMMAND_RANK6: i32 = 4092;
    /// **Not a power of two** (audit A-12): 8196, not 8192; it overlaps
    /// `MAIL_TO_VAULT`.
    pub const MAIL_TO_COMMAND_RANK7: i32 = 8196;

    /// The vault alias bit (the Bank campaign's, D-SS07).
    pub const VAULT_ALIASES: i32 = MAIL_TO_VAULT;

    /// Every organization alias bit (Team, Command, officers, ranks 0-7),
    /// anomalies included.
    pub const ORGANIZATION_ALIASES: i32 = MAIL_TO_TEAM
        | MAIL_TO_COMMAND
        | MAIL_TO_COMMAND_OFFICERS
        | MAIL_TO_COMMAND_RANK0
        | MAIL_TO_COMMAND_RANK1
        | MAIL_TO_COMMAND_RANK2
        | MAIL_TO_COMMAND_RANK3
        | MAIL_TO_COMMAND_RANK4
        | MAIL_TO_COMMAND_RANK5
        | MAIL_TO_COMMAND_RANK6
        | MAIL_TO_COMMAND_RANK7;
}

/// `EMailResultCodes`: `sendMailResult`'s `ResultCode` byte. The client's
/// handler (`0x00e13b90`, SS-E1 M-Q1) switches on 0-7 with one fixed string
/// each, and shows "Unknown mail error." for anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MailResult {
    /// "Gate-mail message sent." plus the failed names, if any; the client
    /// closes the compose window.
    Sent = 0,
    /// "No valid recipients specified. Gate-mail message was not sent."
    NoRecipients = 1,
    /// "Attachment item not available. Gate-mail message was not sent."
    ItemNotAvailable = 2,
    /// "You cannot send a gate-mail message with cash or item attachments
    /// to multiple recipients."
    AttachmentsAndMultipleRecipients = 3,
    /// "You do not have enough naquadah to send that gate-mail message."
    NotEnoughCash = 4,
    /// "You cannot send a message without an item to your vault."
    VaultButNoItem = 5,
    /// "You cannot send a message with cash attached to your vault."
    VaultPlusCash = 6,
    /// "Sending item to vault." The client closes the compose window.
    SentToVault = 7,
}

impl MailResult {
    /// Every value, in declaration order.
    pub const ALL: [MailResult; 8] = [
        MailResult::Sent,
        MailResult::NoRecipients,
        MailResult::ItemNotAvailable,
        MailResult::AttachmentsAndMultipleRecipients,
        MailResult::NotEnoughCash,
        MailResult::VaultButNoItem,
        MailResult::VaultPlusCash,
        MailResult::SentToVault,
    ];

    /// The wire byte.
    pub fn code(self) -> u8 {
        self as u8
    }

    /// The `enumerations.xml` token name, also the stable `result` log value.
    pub fn token(self) -> &'static str {
        match self {
            MailResult::Sent => "MAILRESULT_Sent",
            MailResult::NoRecipients => "MAILRESULT_NoRecipients",
            MailResult::ItemNotAvailable => "MAILRESULT_ItemNotAvailable",
            MailResult::AttachmentsAndMultipleRecipients => {
                "MAILRESULT_AttachmentsAndMultipleRecipients"
            }
            MailResult::NotEnoughCash => "MAILRESULT_NotEnoughCash",
            MailResult::VaultButNoItem => "MAILRESULT_VaultButNoItem",
            MailResult::VaultPlusCash => "MAILRESULT_VaultPlusCash",
            MailResult::SentToVault => "MAILRESULT_SentToVault",
        }
    }
}

impl TryFrom<u8> for MailResult {
    type Error = u8;

    fn try_from(v: u8) -> Result<Self, u8> {
        MailResult::ALL.get(usize::from(v)).copied().ok_or(v)
    }
}
