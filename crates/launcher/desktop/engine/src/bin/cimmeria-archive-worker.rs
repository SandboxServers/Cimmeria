//! Built natively on Windows; run directly there or under a managed Wine prefix.
#[cfg(not(windows))]
fn main() {
    eprintln!("The archive worker executable must be built and run as a Windows binary.");
    std::process::exit(2);
}

#[cfg(windows)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    std::process::exit(cimmeria_launcher_engine::archive_worker::serve_stdio().await);
}
