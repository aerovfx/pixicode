//! Git Worktree Manager — list and create worktrees via git CLI

use std::process::Command;

/// Worktree manager for git worktree operations (uses `git` CLI).
pub struct WorktreeManager {
    repo: std::path::PathBuf,
}

impl WorktreeManager {
    pub fn new(repo: std::path::PathBuf) -> Self {
        Self { repo }
    }

    /// Create at current directory (uses env current_dir as repo).
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

    /// List worktree paths (porcelain format: first line of each block is "worktree <path>").
    pub fn list(&self) -> Result<Vec<String>, String> {
        let out = self.git(&["worktree", "list", "--porcelain"])?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        let paths: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("worktree ") {
                    Some(line.strip_prefix("worktree ").unwrap_or("").to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(paths)
    }

    /// Create a new worktree at `path` for branch `branch`.
    pub fn create(&self, path: &str, branch: &str) -> Result<(), String> {
        self.git(&["worktree", "add", path, branch])?;
        Ok(())
    }

    /// Remove a worktree (does not delete the working tree files, use `git worktree remove` for that).
    pub fn remove(&self, path: &str) -> Result<(), String> {
        self.git(&["worktree", "remove", path])?;
        Ok(())
    }
}

impl Default for WorktreeManager {
    fn default() -> Self {
        Self::current_dir().unwrap_or_else(|_| Self::new(std::path::PathBuf::from(".")))
    }
}
