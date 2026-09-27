//! # cimmeria-wire
//!
//! The wire contract the Base and Cell services share, and the server's
//! Mercury message layer: the Base↔Cell message enums (`cell::messages`),
//! the services-side packet builders (`mercury`, which builds on the
//! `cimmeria-mercury` transport crate and is not part of it), the per-packet
//! log firehoses (`firehose`), the client- and cell-method index tables, the
//! `stateField` bits, and the payload serializers both halves of the server
//! send. It is the bottom of the `cimmeria-services` split
//! (`docs/architecture/services-crate-split.md`): every service crate may
//! depend on it, and it depends on none of them.
//!
//! Code that moved here from `cimmeria-services` keeps its module path
//! (`cell::kismet`, `base::contact_list::wire`, ...) under the same
//! `cell::` / `base::` skeleton, so its own `crate::` paths compile
//! unchanged, and `cimmeria-services` re-exports each module or item at
//! its old path.

#![warn(unreachable_pub)]

/// The ability-tree payloads both the base and the cell send.
pub mod ability_tree {
    pub mod points_property;
    pub mod respec;

    pub use points_property::{training_points_property_args, GENERICPROPERTY_TRAINING_POINTS};
    pub use respec::{
        respec_error_code_args, RespecOutcome, RESPEC_COST_NAQUADAH,
        RESPEC_FEEDBACK_NOTHING_TRAINED, RESPEC_FEEDBACK_NOT_AT_TRAINER,
        RESPEC_FEEDBACK_NOT_ENOUGH_NAQUADAH,
    };

    #[cfg(test)]
    mod tests;
}

/// Base-side wire serializers.
pub mod base {
    /// Contact-list client methods (CM 85-89).
    pub mod contact_list {
        pub mod wire;
    }
}

/// Cell-side method indices, payload serializers and the Base↔Cell messages.
pub mod cell {
    pub mod cell_methods;
    pub mod chat;
    pub mod client_methods;
    /// The client->server cell-method indices under their `CM_*` /
    /// `CLIENT_MG_*` names, and their names for logging. The router that
    /// dispatches on them is `cimmeria-cell`'s.
    pub mod dispatch {
        pub mod constants;
        pub mod names;

        pub use constants::*;
        pub use names::cell_method_name;
    }
    pub mod kismet;
    pub mod mail;
    pub mod messages;
    pub mod player_journal;
    pub mod spawn_record;
}

pub mod containers;
pub mod firehose;
pub mod hex;
pub mod mercury;
pub mod state_field;

// Generic helpers come from `cimmeria-test-support` (a dev-dependency); tests
// import them from `crate::test_support`, as in every service crate.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
