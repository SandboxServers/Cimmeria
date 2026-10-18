//! Offline readers for telemetry exported from ClickHouse as JSONL (player-capture-bots, D-PC15).

pub mod friction;
pub mod rows;

#[cfg(test)]
mod tests;
