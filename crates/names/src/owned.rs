//! Owned name lookups for log fields whose ID may be absent.
//!
//! `book().item(id)` borrows from the book's guard, so it works inline in a
//! `tracing` field but not behind an `Option` ID: `type_id.and_then(|t|
//! book().item(t))` would return a borrow of a guard dropped inside the
//! closure. These copy the name out instead. A `tracing` field expression
//! only runs when the event is enabled, so the copy is paid only by a line
//! that is written.

use crate::handle::book;

/// `items.name` of an item type, `None` for an absent or unnamed ID.
pub fn item(item_type_id: impl Into<Option<i32>>) -> Option<String> {
    let id = item_type_id.into()?;
    book().item(id).map(str::to_owned)
}

/// `entity_templates.template_name`, `None` for an absent or unnamed ID.
pub fn template(template_id: impl Into<Option<i32>>) -> Option<String> {
    let id = template_id.into()?;
    book().template(id).map(str::to_owned)
}

/// `applied_science.name`, `None` for an absent or unnamed ID.
pub fn applied_science(applied_science_id: impl Into<Option<i32>>) -> Option<String> {
    let id = applied_science_id.into()?;
    book().applied_science(id).map(str::to_owned)
}

/// `containers.name` of an inventory container (`MAIN`, `BANK`, ...),
/// `None` for an absent or unnamed ID.
pub fn container(container_id: impl Into<Option<i32>>) -> Option<String> {
    let id = container_id.into()?;
    book().container(id).map(str::to_owned)
}
