use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Result;

/// A file-based deduplication lock.
///
/// Acquiring the lock writes the current Unix timestamp to a lock file.
/// Dropping the guard removes the lock file.
pub struct DedupLock {
    path: PathBuf,
}

impl DedupLock {
    /// Try to acquire the dedup lock at `path` with the given TTL in seconds.
    ///
    /// Returns:
    /// - `Ok(Some(lock))` if the lock was acquired (no live lock existed).
    /// - `Ok(None)` if a live lock already exists (within TTL).
    /// - `Err(_)` on I/O or other errors.
    pub fn try_acquire(path: &Path, ttl_seconds: u64) -> Result<Option<Self>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before UNIX epoch")
            .as_secs();

        if path.exists() {
            let content = fs::read_to_string(path)?;
            let ts: u64 = content.trim().parse().unwrap_or(0);
            let age = now.saturating_sub(ts);
            if age < ttl_seconds {
                // Lock is still fresh — held by someone else.
                return Ok(None);
            }
            // Stale lock — fall through and replace it.
        }

        fs::write(path, now.to_string())?;
        Ok(Some(Self {
            path: path.to_path_buf(),
        }))
    }
}

impl Drop for DedupLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Returns the path used for a dedup lock file for the given session id.
///
/// Format: `{temp_dir}/claude-notify-dedup-{session_id}.lock`
pub fn dedup_lock_path(session_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!("claude-notify-dedup-{session_id}.lock"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn acquire_lock_succeeds_first_time() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.lock");
        let guard = DedupLock::try_acquire(&path, 60).unwrap();
        assert!(guard.is_some(), "first acquire should succeed");
    }

    #[test]
    fn acquire_lock_fails_when_held() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.lock");

        let _guard = DedupLock::try_acquire(&path, 60).unwrap().unwrap();

        let second = DedupLock::try_acquire(&path, 60).unwrap();
        assert!(
            second.is_none(),
            "second acquire should fail while lock is held"
        );
    }

    #[test]
    fn stale_lock_is_replaced() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.lock");

        // Write a timestamp of 0 (ancient / stale).
        fs::write(&path, "0").unwrap();

        let guard = DedupLock::try_acquire(&path, 60).unwrap();
        assert!(
            guard.is_some(),
            "stale lock should be replaced and acquire succeed"
        );

        // The file should now contain a recent timestamp, not "0".
        let content = fs::read_to_string(&path).unwrap();
        let ts: u64 = content.trim().parse().unwrap();
        assert!(
            ts > 0,
            "lock file should contain a real timestamp after replacing stale lock"
        );
    }

    #[test]
    fn lock_path_for_session() {
        let session_id = "abc123";
        let path = dedup_lock_path(session_id);
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(
            filename.contains(session_id),
            "lock path should contain session id, got: {filename}"
        );
    }
}
