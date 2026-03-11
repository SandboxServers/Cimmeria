/// Access levels for GM/admin commands, ordered from least to most privileged.
///
/// The ordering is used for permission checks: a command requiring `GameMaster`
/// access can be executed by anyone with `GameMaster`, `Admin`, or `Developer`
/// level, but not by `Player` or `Moderator`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AccessLevel {
    /// Regular player. Can execute basic informational commands only.
    Player = 0,
    /// Moderator. Can mute, kick, and view player info.
    Moderator = 1,
    /// Game Master. Can spawn entities, teleport, and modify world state.
    GameMaster = 2,
    /// Server administrator. Full access to configuration and management commands.
    Admin = 3,
    /// Developer. Unrestricted access including debug and diagnostic commands.
    Developer = 4,
}

impl AccessLevel {
    /// Check whether this access level is sufficient to execute a command
    /// that requires the given `required` level.
    ///
    /// Returns `true` if `self >= required`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cimmeria_commands::permissions::AccessLevel;
    ///
    /// assert!(AccessLevel::Admin.can_execute(AccessLevel::GameMaster));
    /// assert!(AccessLevel::GameMaster.can_execute(AccessLevel::GameMaster));
    /// assert!(!AccessLevel::Moderator.can_execute(AccessLevel::GameMaster));
    /// ```
    pub fn can_execute(&self, required: AccessLevel) -> bool {
        *self >= required
    }
}

impl std::fmt::Display for AccessLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccessLevel::Player => write!(f, "Player"),
            AccessLevel::Moderator => write!(f, "Moderator"),
            AccessLevel::GameMaster => write!(f, "Game Master"),
            AccessLevel::Admin => write!(f, "Admin"),
            AccessLevel::Developer => write!(f, "Developer"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_level_ordering() {
        assert!(AccessLevel::Developer > AccessLevel::Admin);
        assert!(AccessLevel::Admin > AccessLevel::GameMaster);
        assert!(AccessLevel::GameMaster > AccessLevel::Moderator);
        assert!(AccessLevel::Moderator > AccessLevel::Player);
    }

    #[test]
    fn can_execute_same_level() {
        assert!(AccessLevel::GameMaster.can_execute(AccessLevel::GameMaster));
    }

    #[test]
    fn can_execute_higher_level() {
        assert!(AccessLevel::Developer.can_execute(AccessLevel::Player));
        assert!(AccessLevel::Admin.can_execute(AccessLevel::Moderator));
    }

    #[test]
    fn cannot_execute_insufficient_level() {
        assert!(!AccessLevel::Player.can_execute(AccessLevel::Moderator));
        assert!(!AccessLevel::Moderator.can_execute(AccessLevel::Admin));
    }
}
