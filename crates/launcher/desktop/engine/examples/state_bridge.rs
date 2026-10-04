//! Headless fixture for frontend logic UAT. No GUI, downloads or game mutation.
use cimmeria_launcher_engine::{DesktopState, NativeCommand};
use std::io::{BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("state directory required")?;
    let mut state = DesktopState::open(std::path::Path::new(&root))
        .map_err(|error| format!("state open failed: {error:?}"))?;
    for line in std::io::stdin().lock().lines() {
        let command: NativeCommand = serde_json::from_str(&line?)?;
        let reply = match state.dispatch(command) {
            Ok(snapshot) => serde_json::json!({"ok": snapshot}),
            Err(error) => serde_json::json!({"error": error}),
        };
        println!("{reply}");
        std::io::stdout().flush()?;
    }
    Ok(())
}
