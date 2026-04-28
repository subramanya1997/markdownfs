use std::fmt;

#[derive(Debug)]
pub enum VfsError {
    InvalidExtension { name: String },
    InvalidHandle { handle: u64 },
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
    NotSupported { message: String },
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::InvalidExtension { name } => {
                write!(f, "markdownfs: only .md files are supported: '{name}'")
            }
            VfsError::InvalidHandle { handle } => write!(f, "markdownfs: invalid handle: {handle}"),
            VfsError::NotFound { path } => write!(f, "markdownfs: no such file or directory: '{path}'"),
            VfsError::IsDirectory { path } => write!(f, "markdownfs: is a directory: '{path}'"),
            VfsError::NotDirectory { path } => write!(f, "markdownfs: not a directory: '{path}'"),
            VfsError::AlreadyExists { path } => write!(f, "markdownfs: already exists: '{path}'"),
            VfsError::NotEmpty { path } => write!(f, "markdownfs: directory not empty: '{path}'"),
            VfsError::InvalidPath { path } => write!(f, "markdownfs: invalid path: '{path}'"),
            VfsError::IoError(e) => write!(f, "markdownfs: I/O error: {e}"),
            VfsError::UnknownCommand { name } => write!(f, "markdownfs: unknown command: '{name}'"),
            VfsError::InvalidArgs { message } => write!(f, "markdownfs: {message}"),
            VfsError::SymlinkLoop { path } => write!(f, "markdownfs: symlink loop: '{path}'"),
            VfsError::ObjectNotFound { id } => write!(f, "markdownfs: object not found: {id}"),
            VfsError::CorruptStore { message } => write!(f, "markdownfs: corrupt store: {message}"),
            VfsError::NoCommits => write!(f, "markdownfs: no commits yet"),
            VfsError::DirtyWorkingTree => {
                write!(f, "markdownfs: working tree has uncommitted changes")
            }
            VfsError::PermissionDenied { path } => {
                write!(f, "markdownfs: permission denied: '{path}'")
            }
            VfsError::AuthError { message } => write!(f, "markdownfs: {message}"),
            VfsError::NotSupported { message } => write!(f, "markdownfs: operation not supported: {message}"),
        }
    }
}

impl std::error::Error for VfsError {}

impl From<std::io::Error> for VfsError {
    fn from(e: std::io::Error) -> Self {
        VfsError::IoError(e)
    }
}
