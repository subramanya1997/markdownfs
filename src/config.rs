use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityTarget {
    Markdown,
    Posix,
}

impl CompatibilityTarget {
    pub fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "markdown" | "markdown-only" | "markdown_only" => Some(Self::Markdown),
            "posix" | "mount" | "fuse" => Some(Self::Posix),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Posix => "posix",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: PathBuf,
    pub listen_addr: String,
    pub auto_save_interval_secs: u64,
    pub auto_save_write_threshold: u64,
    pub max_file_size: usize,
    pub max_inodes: usize,
    pub max_dir_depth: usize,
    pub compatibility_target: CompatibilityTarget,
}

impl Config {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("MARKDOWNFS_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let listen_addr = std::env::var("MARKDOWNFS_LISTEN")
            .unwrap_or_else(|_| "127.0.0.1:3000".to_string());

        let auto_save_interval_secs = std::env::var("MARKDOWNFS_AUTOSAVE_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        let auto_save_write_threshold = std::env::var("MARKDOWNFS_AUTOSAVE_WRITES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);

        let max_file_size = std::env::var("MARKDOWNFS_MAX_FILE_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10 * 1024 * 1024); // 10MB

        let max_inodes = std::env::var("MARKDOWNFS_MAX_INODES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1_000_000);

        let max_dir_depth = std::env::var("MARKDOWNFS_MAX_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(256);

        let compatibility_target = std::env::var("MARKDOWNFS_COMPAT_TARGET")
            .ok()
            .and_then(|value| CompatibilityTarget::from_env_value(&value))
            .unwrap_or(CompatibilityTarget::Markdown);

        Config {
            data_dir,
            listen_addr,
            auto_save_interval_secs,
            auto_save_write_threshold,
            max_file_size,
            max_inodes,
            max_dir_depth,
            compatibility_target,
        }
    }

    pub fn with_data_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.data_dir = dir.as_ref().to_path_buf();
        self
    }

    pub fn with_listen_addr(mut self, addr: impl Into<String>) -> Self {
        self.listen_addr = addr.into();
        self
    }

    pub fn with_compatibility_target(mut self, target: CompatibilityTarget) -> Self {
        self.compatibility_target = target;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
