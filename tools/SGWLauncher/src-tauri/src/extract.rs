use serde::Serialize;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("7za exited with code {code}: {stderr}")]
    ExitCode { code: i32, stderr: String },
    #[error("Failed to spawn 7za: {0}")]
    Spawn(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractProgress {
    pub phase: String,
    pub current: u32,
    pub total: u32,
    pub filename: String,
}

pub fn count_cab_files(dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut cabs: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("cab") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    cabs.sort();
    Ok(cabs)
}

pub fn build_extract_args(archive: &str, output_dir: &str) -> Vec<String> {
    vec![
        "e".to_string(),
        archive.to_string(),
        format!("-o{}", output_dir),
        "-y".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_extract_args() {
        let args = build_extract_args("C:\\temp\\archive.rar", "C:\\Games\\SGW");
        assert_eq!(args, vec!["e", "C:\\temp\\archive.rar", "-oC:\\Games\\SGW", "-y"]);
    }

    #[test]
    fn test_count_cab_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data1.cab"), "").unwrap();
        std::fs::write(dir.path().join("data2.cab"), "").unwrap();
        std::fs::write(dir.path().join("readme.txt"), "").unwrap();

        let cabs = count_cab_files(dir.path()).unwrap();
        assert_eq!(cabs.len(), 2);
        assert!(cabs[0].file_name().unwrap().to_str().unwrap().ends_with(".cab"));
    }

    #[test]
    fn test_count_cab_files_empty() {
        let dir = tempfile::tempdir().unwrap();
        let cabs = count_cab_files(dir.path()).unwrap();
        assert_eq!(cabs.len(), 0);
    }
}
