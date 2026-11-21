//! Error types for the sandcrawler linter.

/// Errors that can occur while linting a PKGBUILD.
#[derive(Debug, thiserror::Error)]
pub enum LintError {
    /// The PKGBUILD file could not be read.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The file is not valid PKGBUILD text.
    #[error("invalid PKGBUILD: {0}")]
    Invalid(String),
}
