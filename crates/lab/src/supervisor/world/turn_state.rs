//! The turn keys of the walk: which keys turn left and right, and how a turn
//! command is put on the keyboard.

use super::io::WorldIo;
use super::steer::{TurnCmd, TurnKey};

/// The names of the turn keys this walk may press, for pressing and for
/// the release on every exit.
#[derive(Debug, Clone)]
pub(super) struct TurnKeys {
    pub(super) left: String,
    pub(super) right: String,
}

impl Default for TurnKeys {
    fn default() -> Self {
        Self {
            left: "Q".into(),
            right: "E".into(),
        }
    }
}

impl TurnKeys {
    fn name(&self, k: TurnKey) -> &str {
        match k {
            TurnKey::Left => &self.left,
            TurnKey::Right => &self.right,
        }
    }
}

/// Put the turn keys in the state `cmd` wants. `held` is the key currently
/// down. Returns whether a turn key was used.
pub(super) async fn apply_turn<W: WorldIo>(
    io: &mut W,
    keys: &TurnKeys,
    held: &mut Option<TurnKey>,
    cmd: TurnCmd,
) -> Result<bool, String> {
    let want = match cmd {
        TurnCmd::Off => None,
        TurnCmd::Hold(k) | TurnCmd::Pulse(k, _) => Some(k),
    };
    if *held != want {
        if let Some(old) = held.take() {
            io.key(keys.name(old), false).await?;
        }
        if let Some(k) = want {
            io.key(keys.name(k), true).await?;
            *held = Some(k);
        }
    }
    if let TurnCmd::Pulse(_, ms) = cmd {
        io.sleep(ms).await;
        if let Some(k) = held.take() {
            io.key(keys.name(k), false).await?;
        }
    }
    Ok(want.is_some())
}
