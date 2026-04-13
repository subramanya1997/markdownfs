use std::fmt;

#[derive(Debug)]
pub enum VfsError {
    InvalidExtension { name: String },
    NotFound { path: String },
    IsDirectory { path: String },
    NotDirectory { path: String },
    AlreadyExists { path: String },
    NotEmpty { path: String },
    InvalidPath { path: String },
    IoError(std::io::Error),
    UnknownCommand { name: String },
    InvalidArgs { message: String },
    SymlinkLoop { path: String },
    ObjectNotFound { id: String },
    CorruptStore { message: String },
    NoCommits,
    DirtyWorkingTree,
    PermissionDenied { path: String },
    AuthError { message: String },
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::InvalidExtension { name } => {
                write!(f, "mdvfs: only .md files are supported: '{name}'")
            }
            VfsError::NotFound { path } => write!(f, "mdvfs: no such file or directory: '{path}'"),
            VfsError::IsDirectory { path } => write!(f, "mdvfs: is a directory: '{path}'"),
            VfsError::NotDirectory { path } => write!(f, "mdvfs: not a directory: '{path}'"),
            VfsError::AlreadyExists { path } => write!(f, "mdvfs: already exists: '{path}'"),
            VfsError::NotEmpty { path } => write!(f, "mdvfs: directory not empty: '{path}'"),
            VfsError::InvalidPath { path } => write!(f, "mdvfs: invalid path: '{path}'"),
            VfsError::IoError(e) => write!(f, "mdvfs: I/O error: {e}"),
            VfsError::UnknownCommand { name } => write!(f, "mdvfs: unknown command: '{name}'"),
            VfsError::InvalidArgs { message } => write!(f, "mdvfs: {message}"),
            VfsError::SymlinkLoop { path } => write!(f, "mdvfs: symlink loop: '{path}'"),
            VfsError::ObjectNotFound { id } => write!(f, "mdvfs: object not found: {id}"),
            VfsError::CorruptStore { message } => write!(f, "mdvfs: corrupt store: {message}"),
            VfsError::NoCommits => write!(f, "mdvfs: no commits yet"),
            VfsError::DirtyWorkingTree => {
                write!(f, "mdvfs: working tree has uncommitted changes")
            }
            VfsError::PermissionDenied { path } => {
                write!(f, "mdvfs: permission denied: '{path}'")
            }
            VfsError::AuthError { message } => write!(f, "mdvfs: {message}")
        }
    }
}

impl std::error::Error for VfsError {}

impl From<std::io::Error> for VfsError {
    fn from(e: std::io::Error) -> Self {
        VfsError::IoError(e)
    }
}
