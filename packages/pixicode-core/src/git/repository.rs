//! Git Repository

/// Git repository wrapper.
pub struct Repository;

impl Repository {
    pub fn open(_path: &str) -> Result<Self, String> {
        Err("Not implemented".to_string())
    }

    pub fn status(&self) -> Result<String, String> {
        Err("Not implemented".to_string())
    }
}
