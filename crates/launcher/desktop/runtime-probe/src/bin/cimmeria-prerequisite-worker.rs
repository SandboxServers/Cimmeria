#[cfg(not(all(windows, target_arch = "x86")))]
fn main() {
    eprintln!("prerequisite-worker requires a native Windows x86 build");
    std::process::exit(2);
}
#[cfg(all(windows, target_arch = "x86"))]
fn main() {
    use cimmeria_runtime_probe::{prerequisite, MAX_REQUEST};
    use std::io::Read;
    let mut bytes = Vec::new();
    let request = std::io::stdin()
        .take((MAX_REQUEST + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "input_unavailable")
        .and_then(|_| prerequisite::decode_request(&bytes));
    match request {
        Ok(request) => {
            let result = prerequisite::windows::prepare(request);
            println!(
                "{}",
                serde_json::to_string(&result).expect("fixed result serializes")
            );
        }
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(2);
        }
    }
}
