fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let icon = "icons/icon.ico";
        if std::path::Path::new(icon).exists() {
            let mut res = winres::WindowsResource::new();
            res.set_icon(icon);
            if let Err(e) = res.compile() {
                println!("cargo:warning=winres compile failed (skipping icon embed): {e}");
            }
        }
    }
}
