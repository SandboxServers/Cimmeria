use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("SGW.exe not found at {0}")]
    NotFound(String),
    #[error("Failed to launch game: {0}")]
    SpawnFailed(#[from] std::io::Error),
}

pub fn verify_installation(install_path: &str) -> Result<std::path::PathBuf, LaunchError> {
    let exe_path = Path::new(install_path).join("SGW.exe");
    if !exe_path.exists() {
        return Err(LaunchError::NotFound(exe_path.display().to_string()));
    }
    Ok(exe_path)
}

pub fn launch_game(install_path: &str) -> Result<u32, LaunchError> {
    let exe_path = verify_installation(install_path)?;
    let child = Command::new(&exe_path)
        .current_dir(install_path)
        .spawn()?;
    Ok(child.id())
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum InstallState {
    NotInstalled,
    Installed,
}

pub fn check_installation(install_path: &str) -> InstallState {
    if install_path.is_empty() {
        return InstallState::NotInstalled;
    }
    match verify_installation(install_path) {
        Ok(_) => InstallState::Installed,
        Err(_) => InstallState::NotInstalled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_installation_missing() {
        let result = verify_installation("/nonexistent/path");
        assert!(matches!(result, Err(LaunchError::NotFound(_))));
    }

    #[test]
    fn test_verify_installation_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "fake exe").unwrap();
        let result = verify_installation(dir.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_installation_empty_path() {
        let state = check_installation("");
        assert!(matches!(state, InstallState::NotInstalled));
    }

    #[test]
    fn test_check_installation_with_exe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "fake exe").unwrap();
        let state = check_installation(dir.path().to_str().unwrap());
        assert!(matches!(state, InstallState::Installed));
    }
}
