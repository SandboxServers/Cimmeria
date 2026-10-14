//! TOML section parsing and validation after capability resolution.

use super::{validate, SectionSpec};

/// Parse and validate one section file.
pub fn parse(text: &str) -> Result<SectionSpec, String> {
    let mut spec: SectionSpec = toml::from_str(text).map_err(|e| format!("spec TOML: {e}"))?;
    // `@capability` names become tool names first.
    crate::uat::tools::resolve_section(&mut spec)?;
    validate(&spec)?;
    for row in &spec.rows {
        for action in row.setup.iter().chain(&row.steps).chain(&row.teardown) {
            action
                .route_args()
                .map_err(|e| format!("row {}: {e}", row.id))?;
            for fallback in &action.fallback {
                fallback
                    .route_args()
                    .map_err(|e| format!("row {} fallback: {e}", row.id))?;
            }
        }
    }
    Ok(spec)
}
