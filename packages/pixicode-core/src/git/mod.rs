//! Git Integration — Git worktree and repository management

pub mod worktree;
pub mod repository;
pub mod snapshot;

pub use worktree::WorktreeManager;
pub use repository::Repository;
pub use snapshot::SnapshotManager;
