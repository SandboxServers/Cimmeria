use serde::Serialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("7za exited with code {code}: {stderr}")]
    ExitCode { code: i32, stderr: String },
    #[error("Failed to spawn 7za: {0}")]
    Spawn(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Invalid zip entry (path traversal attempt): {0}")]
    InvalidEntry(String),
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

pub fn extract_zip_to_dir(zip_path: &Path, output_dir: &Path) -> Result<(), ExtractError> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_path = entry
            .enclosed_name()
            .ok_or_else(|| ExtractError::InvalidEntry(entry.name().to_string()))?
            .to_path_buf();
        let out_path = output_dir.join(&entry_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out_file = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }
    Ok(())
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

    #[test]
    fn test_extract_zip_valid() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        let output_dir = dir.path().join("extracted");

        // Create a simple zip in memory
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("test.txt", Default::default()).unwrap();
            std::io::Write::write_all(&mut zip, b"hello").unwrap();
            zip.finish().unwrap();
        }

        // Extract it
        extract_zip_to_dir(&zip_path, &output_dir).unwrap();

        // Verify
        let extracted_file = output_dir.join("test.txt");
        assert!(extracted_file.exists());
        let content = std::fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "hello");
    }

    #[test]
    fn test_extract_zip_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        let output_dir = dir.path().join("extracted");

        // Create a zip with path traversal attempt
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("../evil.txt", Default::default()).unwrap();
            std::io::Write::write_all(&mut zip, b"evil").unwrap();
            zip.finish().unwrap();
        }

        // Extract should reject it
        let result = extract_zip_to_dir(&zip_path, &output_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_zip_nested_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("test.zip");
        let output_dir = dir.path().join("extracted");

        // Create a zip with nested structure
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("subdir/file.txt", Default::default()).unwrap();
            std::io::Write::write_all(&mut zip, b"nested").unwrap();
            zip.finish().unwrap();
        }

        // Extract it
        extract_zip_to_dir(&zip_path, &output_dir).unwrap();

        // Verify nested structure
        let extracted_file = output_dir.join("subdir").join("file.txt");
        assert!(extracted_file.exists());
        let content = std::fs::read_to_string(&extracted_file).unwrap();
        assert_eq!(content, "nested");
    }
}
