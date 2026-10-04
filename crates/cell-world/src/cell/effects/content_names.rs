//! Content names for log fields whose ID may be an `Option` (Rule 6, NT-20).
//!
//! `cimmeria_names::book().ability(id)` borrows its name from the book guard,
//! so it can't sit inside an `Option::and_then` closure. These resolve the
//! name and intern it, the way `EntityNames::of` interns template names, so
//! `pending_ability_name = ability_name(pending_ability_id)` reads as one
//! field. `None` in, or an unnamed ID, gives `None`, and the field is left
//! off the line. Call them inside the macro that logs, never ahead of it.

use cimmeria_entity::name_intern::intern_opt;

/// `abilities.name` for an optional ability ID.
pub fn ability_name(ability_id: impl Into<Option<i32>>) -> Option<&'static str> {
    ability_id
        .into()
        .and_then(|id| intern_opt(cimmeria_names::book().ability(id)))
}

/// `effects.name` for an optional effect ID.
pub fn effect_name(effect_id: impl Into<Option<i32>>) -> Option<&'static str> {
    effect_id
        .into()
        .and_then(|id| intern_opt(cimmeria_names::book().effect(id)))
}

/// `entity_templates.template_name` for an optional template ID.
pub fn template_name(template_id: impl Into<Option<i32>>) -> Option<&'static str> {
    template_id
        .into()
        .and_then(|id| intern_opt(cimmeria_names::book().template(id)))
}

/// `items.name` for an optional item *type* (design) ID. An inventory
/// instance ID is not a type: resolve it to its design ID first.
pub fn item_name(item_type_id: impl Into<Option<i32>>) -> Option<&'static str> {
    item_type_id
        .into()
        .and_then(|id| intern_opt(cimmeria_names::book().item(id)))
}
