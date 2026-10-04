//! Windows x86 lifecycle worker; compile natively on Windows.
fn main() {
    #[cfg(windows)]
    if cimmeria_runtime_probe::game_launch::windows::run().is_err() {
        std::process::exit(1);
    }
    #[cfg(not(windows))]
    std::process::exit(2);
}
