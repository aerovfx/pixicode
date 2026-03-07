//! Git Snapshot Manager — create and revert snapshots via git stash

use std::process::Command;

/// Snapshot manager (uses `git stash` for create, revert).
pub struct SnapshotManager {
    repo: std::path::PathBuf,
}

impl SnapshotManager {
    pub fn new(repo: std::path::PathBuf) -> Self {
        Self { repo }
    }

    pub fn current_dir() -> Result<Self, String> {
        let repo = std::env::current_dir().map_err(|e| e.to_string())?;
        Ok(Self::new(repo))
    }

    fn git(&self, args: &[&str]) -> Result<std::process::Output, String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(stderr.to_string());
        }
        Ok(out)
    }

    /// Create a snapshot (git stash push with message). Returns stash ref (e.g. "stash@{0}").
    pub fn create(&self, message: &str) -> Result<String, String> {
        self.git(&["stash", "push", "-m", message])?;
        let out = self.git(&["stash", "list", "--format=%gd"])?;
        let first = String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .map(str::trim)
            .unwrap_or("stash@{0}")
            .to_string();
        Ok(first)
    }

    /// Revert to a snapshot (git stash apply by ref). Does not drop the stash.
    pub fn revert(&self, snapshot_id: &str) -> Result<(), String> {
        let ref_str = if snapshot_id.starts_with("stash@{") {
            snapshot_id
        } else {
            return Err("snapshot_id should be like stash@{0}".to_string());
        };
        self.git(&["stash", "apply", ref_str])?;
        Ok(())
    }

    /// List snapshots (stash list).
    pub fn list(&self) -> Result<Vec<String>, String> {
        let out = self.git(&["stash", "list", "--format=%gd %s"])?;
        let list: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        Ok(list)
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::current_dir().unwrap_or_else(|_| Self::new(std::path::PathBuf::from(".")))
    }
}
