#[cfg(not(all(windows, target_arch = "x86")))]
fn main() {
    eprintln!("runtime-probe requires a native Windows x86 build");
    std::process::exit(2);
}

#[cfg(all(windows, target_arch = "x86"))]
fn main() {
    use cimmeria_runtime_probe::{decode, windows, MAX_REQUEST};
    use std::io::Read;
    if std::env::args().nth(1).as_deref() == Some("--self-test") {
        std::process::exit(if windows::self_test() { 0 } else { 1 });
    }
    let mut bytes = Vec::new();
    let result = std::io::stdin()
        .take((MAX_REQUEST + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "input_unavailable")
        .and_then(|_| decode(&bytes))
        .and_then(windows::probe);
    match result {
        Ok(report) => println!(
            "{}",
            serde_json::to_string(&report).expect("fixed report serializes")
        ),
        Err(code) => {
            eprintln!("{code}");
            std::process::exit(2);
        }
    }
}
