//! Linux Play through a Proton build the player already has (D-PP4).
//!
//! The launcher starts its workers through the Proton `proton` script (D-PP1,
//! D-PP3), sets up the prefix with `proton run wineboot.exe` (D-PP2), keeps one
//! private prefix per state root (D-PP5) and never signals a guest process
//! (D-PP9). `discovery` finds and ranks the installs; `vdf` reads the text files
//! Steam and Proton ship.

pub mod discovery;
pub mod environment;
pub mod invocation;
pub mod paths;
pub mod physx;
pub mod prefix;
pub mod setup;
mod vdf;
