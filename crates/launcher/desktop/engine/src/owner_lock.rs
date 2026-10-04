//! Release an exclusive file lock when its logical owner ends, even when an OS
//! handle was duplicated (including a transient child-process inheritance).
use std::{
    fs::File,
    ops::{Deref, DerefMut},
};

pub(crate) struct OwnerLock(File);

impl OwnerLock {
    pub(crate) fn acquire(file: File) -> Result<Self, std::fs::TryLockError> {
        file.try_lock()?;
        Ok(Self(file))
    }
}
impl Deref for OwnerLock {
    type Target = File;
    fn deref(&self) -> &File {
        &self.0
    }
}
impl DerefMut for OwnerLock {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.0
    }
}
impl Drop for OwnerLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_owner_releases_lock_even_while_a_duplicate_handle_survives() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("owner");
        let open = || {
            File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .unwrap()
        };
        let guard = OwnerLock::acquire(open()).unwrap();
        let duplicate = guard.try_clone().unwrap();
        assert!(OwnerLock::acquire(open()).is_err());
        drop(guard);
        let next = OwnerLock::acquire(open()).unwrap();
        assert!(OwnerLock::acquire(open()).is_err());
        drop(duplicate);
        drop(next);
        assert!(OwnerLock::acquire(open()).is_ok());
    }
}
