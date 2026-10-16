//! Parse the project-maintained patch 012 slash table without inferring effects.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Client parser types, in the order declared by patch 012.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SlashParameterType {
    Boolean,
    Integer,
    Float,
    String,
}

/// One mandatory or optional client parser parameter.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SlashParameter {
    pub name: String,
    pub parameter_type: SlashParameterType,
    pub required: bool,
}

/// A slash word and its client event, independent of any server method claim.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SlashEntry {
    pub name: String,
    pub event: String,
    pub access: String,
    pub parameters: Vec<SlashParameter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchTable {
    access: String,
    command: Vec<PatchCommand>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PatchCommand {
    name: String,
    event: String,
    access: Option<String>,
    #[serde(default)]
    params: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

/// Parse patch 012's TOML and reject malformed or duplicated words/parameters.
pub fn parse_patch_table(source: &str) -> Result<Vec<SlashEntry>, String> {
    let table: PatchTable = toml::from_str(source).map_err(|e| format!("slash TOML: {e}"))?;
    valid_access(&table.access)?;
    if table.command.is_empty() {
        return Err("slash table is empty".into());
    }
    let mut words = HashSet::new();
    let mut entries = Vec::with_capacity(table.command.len());
    for command in table.command {
        if !valid_word(&command.name) || !words.insert(command.name.clone()) {
            return Err(format!(
                "invalid or duplicated slash word: {}",
                command.name
            ));
        }
        if !command.event.starts_with("Event_SlashCmd_")
            || !command.event["Event_SlashCmd_".len()..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric())
            || command.event.len() == "Event_SlashCmd_".len()
        {
            return Err(format!("invalid slash event: {}", command.event));
        }
        let access = command.access.unwrap_or_else(|| table.access.clone());
        valid_access(&access)?;
        let mut names = HashSet::new();
        let mut parameters = Vec::with_capacity(command.params.len() + command.optional.len());
        for (required, declarations) in [(true, command.params), (false, command.optional)] {
            for declaration in declarations {
                let parameter = parse_parameter(&declaration, required)?;
                if !names.insert(parameter.name.clone()) {
                    return Err(format!(
                        "duplicate parameter in {}: {}",
                        command.name, parameter.name
                    ));
                }
                parameters.push(parameter);
            }
        }
        entries.push(SlashEntry {
            name: command.name,
            event: command.event,
            access,
            parameters,
        });
    }
    Ok(entries)
}

fn valid_word(word: &str) -> bool {
    word.len() > 1
        && word.starts_with('/')
        && word.as_bytes()[1].is_ascii_lowercase()
        && word[2..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

fn valid_access(access: &str) -> Result<(), String> {
    if access.is_empty() || !access.bytes().all(|b| b"gcCqQadDpm".contains(&b)) {
        Err(format!("invalid slash access: {access}"))
    } else {
        Ok(())
    }
}

fn parse_parameter(declaration: &str, required: bool) -> Result<SlashParameter, String> {
    let (kind, name) = declaration
        .split_once(' ')
        .ok_or_else(|| format!("parameter must be '<Type> <Name>': {declaration}"))?;
    let parameter_type = match kind {
        "Boolean" => SlashParameterType::Boolean,
        "Integer" => SlashParameterType::Integer,
        "Float" => SlashParameterType::Float,
        "String" => SlashParameterType::String,
        _ => return Err(format!("unknown slash parameter type: {kind}")),
    };
    if name.is_empty()
        || !name.as_bytes()[0].is_ascii_alphabetic()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-".contains(&b))
    {
        return Err(format!("invalid slash parameter name: {name}"));
    }
    Ok(SlashParameter {
        name: name.into(),
        parameter_type,
        required,
    })
}
