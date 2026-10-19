//! Beta Site E1 cross-race guard (UB-45a, part 1). Each chain packet proves
//! only its own chains (P8), so these tests load the whole seed and fire
//! every trigger key at every state a player can be in. `walk` derives the
//! states from the seed, `dialog_exclusive` proves one dialog per click and
//! one offer per choice, and `race_negatives` proves no state moves another
//! race's player.

mod dialog_exclusive;
mod race_negatives;
mod walk;
