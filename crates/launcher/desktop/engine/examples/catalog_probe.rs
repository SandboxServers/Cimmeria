//! Read-only catalog smoke check; uses the same fixed URL and build-time key as the shell.
#[tokio::main(flavor = "current_thread")]
async fn main() {
    match cimmeria_launcher_engine::catalog::fetch_patch_notes().await {
        Ok(notes) => println!(
            "{}",
            serde_json::to_string(&notes).expect("notes serialize")
        ),
        Err(error) => {
            eprintln!("{error:?}");
            std::process::exit(1);
        }
    }
}
