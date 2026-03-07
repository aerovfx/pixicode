//! Desktop commands that use pixicode-core and pixicode-types (unified workspace).

use pixicode_core::config::paths::ConfigPaths;
use pixicode_types::VersionInfo;
use serde::Serialize;

/// Returns the core data directory (same logic as CLI). Uses shared ConfigPaths from pixicode-core.
#[tauri::command]
#[specta::specta]
pub fn get_core_data_dir() -> Result<String, String> {
    let paths = ConfigPaths::new().map_err(|e| e.to_string())?;
    Ok(paths.data_dir().to_string_lossy().to_string())
}

/// Returns version info; shape matches pixicode_types::VersionInfo for shared frontend typing.
#[tauri::command]
#[specta::specta]
pub fn get_core_version_info() -> CoreVersionInfo {
    CoreVersionInfo::from(VersionInfo {
        version: pixicode_core_version(),
        build_date: None,
        git_hash: None,
        target: Some(std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())),
    })
}

fn pixicode_core_version() -> String {
    pixicode_core::VERSION.to_string()
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct CoreVersionInfo {
    pub version: String,
    pub build_date: Option<String>,
    pub git_hash: Option<String>,
    pub target: Option<String>,
}

impl From<VersionInfo> for CoreVersionInfo {
    fn from(v: VersionInfo) -> Self {
        Self {
            version: v.version,
            build_date: v.build_date,
            git_hash: v.git_hash,
            target: v.target,
        }
    }
}
