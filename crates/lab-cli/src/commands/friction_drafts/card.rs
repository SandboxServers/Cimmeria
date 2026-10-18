//! Renders one friction cluster as a markdown bug card with a commented draft row.

use crate::human::friction::FrictionCluster;

/// The sample keys a card prints, in the order it prints them. Nothing else from a sample leaves the card.
const SAMPLE_KEYS: &[&str] = &[
    "x",
    "z",
    "template_id",
    "template_name",
    "mission_name",
    "step_name",
    "item_id",
    "interaction_flags_names",
];

/// Renders a cluster as a bug card. Pure: the same cluster always gives the same text.
///
/// Every line of the draft row is a comment, because the row is a proposal a person confirms.
pub fn render_card(c: &FrictionCluster) -> String {
    let signal = one_line(&c.signal);
    let subject = one_line(&c.subject);
    let world = one_line(&c.world);
    let mut lines = vec![
        format!("# Friction: {signal} at {subject} ({world})"),
        String::new(),
        format!("- signal: {signal}"),
        format!("- reason: {}", one_line(&c.reason)),
        format!("- world: {world}"),
        format!("- subject: {subject}"),
        format!("- count: {} rows", c.count),
        format!("- players: {} distinct", c.players),
        format!(
            "- first seen: {}",
            one_line(c.first_seen.as_deref().unwrap_or("-"))
        ),
        format!(
            "- last seen: {}",
            one_line(c.last_seen.as_deref().unwrap_or("-"))
        ),
    ];
    for key in SAMPLE_KEYS {
        if let Some(value) = sample_text(c, key) {
            lines.push(format!("- {key}: {}", one_line(&value)));
        }
    }
    let title = toml_str(&format!("{} at {}", c.signal, c.subject));
    lines.extend([
        String::new(),
        "## Expected".to_string(),
        String::new(),
        "TODO: what should have happened".to_string(),
        String::new(),
        "## Draft row (UNCONFIRMED)".to_string(),
        String::new(),
        "```toml".to_string(),
        "# [[row]]".to_string(),
        "# id = \"FR-<nn>\"".to_string(),
        format!("# title = \"{title}\""),
        "# expected = \"TODO: what should have happened\"".to_string(),
        format!(
            "# setup = [{{ tool = \"@stand\", args = {{}}, label = \"stand\" }}]   # adjust: target {subject}, see fs_p3_draft.toml"
        ),
        "# step = []".to_string(),
        "# [[row.expect]]".to_string(),
        "# TODO".to_string(),
        "```".to_string(),
    ]);
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

/// `text` on one line: every control character, newlines included, becomes a space.
///
/// Exported values are untrusted text. A newline in one would end a card line early,
/// leaving an uncommented line in the draft row or closing its fence.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `text` as the inside of a TOML basic string, on one line.
fn toml_str(text: &str) -> String {
    one_line(text).replace('\\', "\\\\").replace('"', "\\\"")
}

/// The text of one sample key: a string or number as is, an array joined with `, `.
fn sample_text(c: &FrictionCluster, key: &str) -> Option<String> {
    match c.sample.0.get(key)? {
        serde_json::Value::Array(items) => Some(
            items
                .iter()
                .filter_map(|item| match item {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
        _ => c.sample.text(key),
    }
}
