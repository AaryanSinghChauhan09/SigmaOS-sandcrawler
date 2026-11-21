//! SigmaOS sandcrawler component.
//!
//! Rust implementation of `archlinux/sandcrawler`: a PKGBUILD linter that
//! flags common packaging mistakes before they reach the repos.
//!
//! Upstream lives on gitlab.archlinux.org (GitHub mirror is docs-only);
//! this port implements the linter's rule engine: parse PKGBUILD text into
//! assignments and apply lint rules producing severity-graded diagnostics.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod lint;
pub mod pkgbuild;

pub use error::LintError;
pub use lint::{LintReport, Rule, Severity, lint_pkgbuild, lint_pkgbuild_text};
pub use pkgbuild::{Assignment, PkgbuildDoc, parse_pkgbuild};
