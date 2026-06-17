use cimmeria_common::math::Vector3;
use cimmeria_common::types::EntityId;

/// Parse a command string into the command name and its arguments.
///
/// The first whitespace-delimited token is the command name (with any leading
/// `/` stripped). Remaining tokens are returned as arguments.
///
/// Returns `None` if the input is empty or contains only whitespace.
///
/// # Examples
///
/// ```
/// use cimmeria_commands::parser::parse_command;
///
/// let (name, args) = parse_command("/teleport player1 100 200 300").unwrap();
/// assert_eq!(name, "teleport");
/// assert_eq!(args, vec!["player1", "100", "200", "300"]);
///
/// let (name, args) = parse_command("help").unwrap();
/// assert_eq!(name, "help");
/// assert!(args.is_empty());
/// ```
pub fn parse_command(input: &str) -> Option<(&str, Vec<&str>)> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let raw_command = parts.next()?;

    // Strip leading '/' if present (e.g., "/teleport" -> "teleport").
    let command = raw_command.strip_prefix('/').unwrap_or(raw_command);
    if command.is_empty() {
        return None;
    }

    let args: Vec<&str> = parts.collect();
    Some((command, args))
}

/// Parse a string into an `EntityId`.
///
/// Accepts both plain numeric IDs (`"123"`) and hash-prefixed IDs (`"#123"`).
///
/// # Examples
///
/// ```
/// use cimmeria_commands::parser::parse_entity_id;
///
/// let id = parse_entity_id("42").unwrap();
/// assert_eq!(id.0, 42);
///
/// let id = parse_entity_id("#999").unwrap();
/// assert_eq!(id.0, 999);
///
/// assert!(parse_entity_id("abc").is_none());
/// ```
pub fn parse_entity_id(s: &str) -> Option<EntityId> {
    let cleaned = s.strip_prefix('#').unwrap_or(s);
    cleaned.parse::<i32>().ok().map(EntityId)
}

/// Parse three consecutive string arguments into a `Vector3`.
///
/// Expects exactly three parseable `f32` values in x, y, z order. Non-finite
/// values (`nan`, `inf`) are rejected — Rust's float parser accepts those
/// spellings, but they are never valid coordinates.
///
/// # Examples
///
/// ```
/// use cimmeria_commands::parser::parse_vector3;
///
/// let v = parse_vector3(&["1.0", "2.5", "-3.0"]).unwrap();
/// assert_eq!(v.x, 1.0);
/// assert_eq!(v.y, 2.5);
/// assert_eq!(v.z, -3.0);
///
/// assert!(parse_vector3(&["1.0", "2.0"]).is_none());
/// ```
pub fn parse_vector3(args: &[&str]) -> Option<Vector3> {
    if args.len() < 3 {
        return None;
    }

    // Reject non-finite values: Rust's `f32` parser accepts "nan", "inf",
    // "-inf", "infinity" (case-insensitive), but a non-finite coordinate is
    // never valid input — it poisons AoI distance math (NaN fails every
    // ordering comparison, dropping the entity out of witness calc) and ships
    // a garbage forced-position snap to the client.
    let x = args[0].parse::<f32>().ok().filter(|v| v.is_finite())?;
    let y = args[1].parse::<f32>().ok().filter(|v| v.is_finite())?;
    let z = args[2].parse::<f32>().ok().filter(|v| v.is_finite())?;

    Some(Vector3::new(x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_with_slash() {
        let (name, args) = parse_command("/kick player1").unwrap();
        assert_eq!(name, "kick");
        assert_eq!(args, vec!["player1"]);
    }

    #[test]
    fn parse_command_without_slash() {
        let (name, args) = parse_command("help commands").unwrap();
        assert_eq!(name, "help");
        assert_eq!(args, vec!["commands"]);
    }

    #[test]
    fn parse_command_no_args() {
        let (name, args) = parse_command("/status").unwrap();
        assert_eq!(name, "status");
        assert!(args.is_empty());
    }

    #[test]
    fn parse_command_empty_input() {
        assert!(parse_command("").is_none());
        assert!(parse_command("   ").is_none());
    }

    #[test]
    fn parse_command_only_slash() {
        assert!(parse_command("/").is_none());
    }

    #[test]
    fn parse_command_extra_whitespace() {
        let (name, args) = parse_command("  /spawn   npc1  100  ").unwrap();
        assert_eq!(name, "spawn");
        assert_eq!(args, vec!["npc1", "100"]);
    }

    #[test]
    fn parse_entity_id_plain() {
        let id = parse_entity_id("42").unwrap();
        assert_eq!(id, EntityId(42));
    }

    #[test]
    fn parse_entity_id_hash_prefix() {
        let id = parse_entity_id("#1000").unwrap();
        assert_eq!(id, EntityId(1000));
    }

    #[test]
    fn parse_entity_id_negative() {
        // EntityId uses i32, so negative values are valid.
        let id = parse_entity_id("-5").unwrap();
        assert_eq!(id, EntityId(-5));
    }

    #[test]
    fn parse_entity_id_invalid() {
        assert!(parse_entity_id("abc").is_none());
        assert!(parse_entity_id("#").is_none());
        assert!(parse_entity_id("").is_none());
    }

    #[test]
    fn parse_vector3_valid() {
        let v = parse_vector3(&["10.0", "20.5", "-30.0"]).unwrap();
        assert_eq!(v.x, 10.0);
        assert_eq!(v.y, 20.5);
        assert_eq!(v.z, -30.0);
    }

    #[test]
    fn parse_vector3_integers() {
        let v = parse_vector3(&["1", "2", "3"]).unwrap();
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }

    #[test]
    fn parse_vector3_too_few_args() {
        assert!(parse_vector3(&["1.0", "2.0"]).is_none());
        assert!(parse_vector3(&["1.0"]).is_none());
        assert!(parse_vector3(&[]).is_none());
    }

    #[test]
    fn parse_vector3_non_numeric() {
        assert!(parse_vector3(&["1.0", "abc", "3.0"]).is_none());
    }

    /// **Regression guard:** `nan`/`inf` (in any spelling Rust's float parser
    /// accepts) must be rejected — a non-finite coordinate poisons AoI
    /// distance math and the client forced-position snap. Dropping the
    /// `is_finite` filter lets these through as a valid `Vector3`.
    #[test]
    fn parse_vector3_rejects_non_finite() {
        assert!(parse_vector3(&["nan", "0", "0"]).is_none());
        assert!(parse_vector3(&["NaN", "0", "0"]).is_none());
        assert!(parse_vector3(&["inf", "0", "0"]).is_none());
        assert!(parse_vector3(&["0", "-inf", "0"]).is_none());
        assert!(parse_vector3(&["0", "0", "infinity"]).is_none());
    }

    #[test]
    fn parse_vector3_extra_args_ignored() {
        // Only first three are consumed; extras are silently ignored.
        let v = parse_vector3(&["1.0", "2.0", "3.0", "4.0"]).unwrap();
        assert_eq!(v.x, 1.0);
        assert_eq!(v.y, 2.0);
        assert_eq!(v.z, 3.0);
    }
}
