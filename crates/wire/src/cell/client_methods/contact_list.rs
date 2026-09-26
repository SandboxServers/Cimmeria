//! ContactListManager interface ClientMethods (indices 85–89).

/// Contact list created/updated (friends, ignore, etc).
pub const ON_CONTACT_LIST_UPDATE: u16 = 85;
/// Contact list deleted.
pub const ON_CONTACT_LIST_DELETE: u16 = 86;
/// Members added to a contact list.
pub const ON_CONTACT_LIST_ADD_MEMBERS: u16 = 87;
/// Members removed from a contact list.
pub const ON_CONTACT_LIST_REMOVE_MEMBERS: u16 = 88;
/// Contact list event (online/offline/zone change).
pub const ON_CONTACT_LIST_EVENT: u16 = 89;
