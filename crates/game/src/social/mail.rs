use serde::{Deserialize, Serialize};

/// A mail message between players.
///
/// Corresponds to the sgw_gate_mail table in the database schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub mail_id: i64,
    pub sender_name: String,
    pub recipient_name: String,
    pub subject: String,
    pub body: String,
    pub attached_money: i64,
    pub attached_item_id: Option<i64>,
    pub is_read: bool,
    pub is_collected: bool,
}

impl MailMessage {
    /// Create a new text-only mail message.
    pub fn new(mail_id: i64, sender_name: String, recipient_name: String, subject: String, body: String) -> Self {
        Self {
            mail_id,
            sender_name,
            recipient_name,
            subject,
            body,
            attached_money: 0,
            attached_item_id: None,
            is_read: false,
            is_collected: false,
        }
    }

    /// Attach money to this mail.
    pub fn with_money(mut self, amount: i64) -> Self {
        self.attached_money = amount;
        self
    }

    /// Attach an item to this mail.
    pub fn with_item(mut self, item_instance_id: i64) -> Self {
        self.attached_item_id = Some(item_instance_id);
        self
    }

    /// Returns `true` if this mail has any attachments.
    pub fn has_attachments(&self) -> bool {
        self.attached_money > 0 || self.attached_item_id.is_some()
    }

    /// Send this mail, persisting it to the database.
    pub fn send(&self) {
        todo!("MailMessage::send - write to sgw_gate_mail")
    }

    /// Collect attachments from this mail.
    pub fn collect_attachments(&mut self, _player_entity_id: i32) {
        todo!("MailMessage::collect_attachments - transfer money/items to player")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_mail_no_attachments() {
        let m = MailMessage::new(1, "Daniel".to_string(), "Jack".to_string(), "Artifacts".to_string(), "Found something.".to_string());
        assert!(!m.has_attachments());
        assert!(!m.is_read);
    }

    #[test]
    fn mail_with_money() {
        let m = MailMessage::new(2, "Sam".to_string(), "Teal'c".to_string(), "Funds".to_string(), "For supplies.".to_string())
            .with_money(500);
        assert!(m.has_attachments());
        assert_eq!(m.attached_money, 500);
    }

    #[test]
    fn mail_with_item() {
        let m = MailMessage::new(3, "Vala".to_string(), "Daniel".to_string(), "Gift".to_string(), "You'll like this.".to_string())
            .with_item(42);
        assert!(m.has_attachments());
        assert_eq!(m.attached_item_id, Some(42));
    }
}
