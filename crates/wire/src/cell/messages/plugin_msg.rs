//! The feature-message envelope, `CellToBaseMsg::Plugin`
//! (`docs/architecture/plugin-architecture.md` §3.4).
//!
//! A migrated feature sends its cell-to-base messages as a [`PluginMsg`]
//! instead of adding a variant to `CellToBaseMsg`, which every crate on both
//! tracks names. The payload is a feature-owned type; the base plugin that
//! consumes it registers for that type (`cimmeria-base-session`'s
//! `base::plugin`) and downcasts it.
//!
//! The envelope travels on the same channel as every other
//! `CellToBaseMsg`, so it keeps its FIFO position relative to the sends
//! around it (ADR §2, C6). Messages never leave the process: the payload is
//! boxed, not serialized.

use std::any::{Any, TypeId};

/// One feature message on the cell-to-base channel.
pub struct PluginMsg {
    type_id: TypeId,
    type_name: &'static str,
    payload: Box<dyn Any + Send>,
}

impl PluginMsg {
    /// Wrap `payload`. Its type is the routing key.
    pub fn new<T: Any + Send>(payload: T) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            payload: Box::new(payload),
        }
    }

    /// The payload's type, which the base's consumer registry keys on.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// The payload's type name, for logs.
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// Whether the payload is a `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    /// The payload as a `T`, or the envelope back when it is another type.
    pub fn downcast<T: Any>(self) -> Result<T, Self> {
        if !self.is::<T>() {
            return Err(self);
        }
        match self.payload.downcast::<T>() {
            Ok(payload) => Ok(*payload),
            // Unreachable: the type id matched. Kept total rather than
            // panicking.
            Err(payload) => Err(Self {
                type_id: self.type_id,
                type_name: self.type_name,
                payload,
            }),
        }
    }
}

impl std::fmt::Debug for PluginMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginMsg")
            .field("type_name", &self.type_name)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Ping(u32);
    struct Pong;

    #[test]
    fn a_payload_downcasts_to_its_own_type_only() {
        let msg = PluginMsg::new(Ping(7));
        assert!(msg.is::<Ping>());
        assert!(!msg.is::<Pong>());
        assert_eq!(msg.type_id(), TypeId::of::<Ping>());
        assert!(msg.type_name().ends_with("Ping"));
        let msg = match msg.downcast::<Pong>() {
            Ok(_) => panic!("a Ping is not a Pong"),
            Err(msg) => msg,
        };
        assert_eq!(msg.downcast::<Ping>().unwrap(), Ping(7));
    }

    #[test]
    fn debug_names_the_payload_type_not_its_value() {
        let text = format!("{:?}", PluginMsg::new(Ping(7)));
        assert!(text.contains("Ping"), "{text}");
        assert!(!text.contains('7'), "{text}");
    }
}
