//! The key presses of an unstick: a jump or a strafe that breaks a snag.

use super::io::WorldIo;
use super::steer::Unstick;

/// How long a strafe unstick holds its key.
const STRAFE_MS: u64 = 600;

/// Presses the unstick's key: a jump is a short tap, a strafe holds for `STRAFE_MS`.
pub async fn run_unstick<W: WorldIo>(io: &mut W, u: Unstick) -> Result<(), String> {
    match u {
        Unstick::Jump => {
            io.key(u.key(), true).await?;
            io.sleep(80).await;
            io.key(u.key(), false).await?;
        }
        Unstick::StrafeRight | Unstick::StrafeLeft => {
            io.key(u.key(), true).await?;
            io.sleep(STRAFE_MS).await;
            io.key(u.key(), false).await?;
        }
    }
    Ok(())
}
