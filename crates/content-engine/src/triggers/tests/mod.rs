use super::*;

mod matching_cover;
mod matching_entity;
mod matching_misc;
mod matching_mission;
mod trigger_type;

fn make_event(trigger_type: TriggerType, params: Vec<(&str, serde_json::Value)>) -> TriggerEvent {
    TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    }
}
