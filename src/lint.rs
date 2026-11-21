//! Lint rules for PKGBUILDs — port of sandcrawler's rule set.

use crate::pkgbuild::PkgbuildDoc;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Style/nit.
    Info,
    /// Likely mistake.
    Warning,
    /// Must fix before merge.
    Error,
}

/// A lint rule identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Missing required field (pkgname/pkgver/pkgrel/arch/source).
    MissingField(&'static str),
    /// `pkgver()` used where static `pkgver=` is expected, or vice versa.
    PkgverStyle,
    /// `pkgrel` should be an integer ≥ 1.
    BadPkgrel,
    /// `arch` missing `x86_64`/`any` coverage.
    BadArch,
    /// `license` missing or contains non-free licenses.
    BadLicense,
    /// `source` missing or empty.
    MissingSource,
    /// `md5sums`/`sha*` integrity missing for a source.
    MissingIntegrity,
    /// `url` missing.
    MissingUrl,
    /// `maintainer`/`packager` deprecation.
    DeprecatedField(&'static str),
    /// `prepare()`/`build()` empty or missing for source builds.
    EmptyFunction(&'static str),
    /// Use of `makepkg`/`sudo` inside PKGBUILD.
    ForbiddenCommand(&'static str),
}

/// A single diagnostic.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Rule that fired.
    pub rule: Rule,
    /// Severity.
    pub severity: Severity,
    /// Message.
    pub message: String,
    /// Line number if known.
    pub line: Option<usize>,
}

/// Lint report.
#[derive(Debug, Default)]
pub struct LintReport {
    /// All diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

impl LintReport {
    /// Errors only.
    pub fn errors(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }
    /// Whether the report has any Error-severity diagnostics.
    pub fn has_errors(&self) -> bool {
        self.errors() > 0
    }
    /// Diagnostics for a given rule.
    pub fn for_rule(&self, rule: Rule) -> Vec<&Diagnostic> {
        self.diagnostics.iter().filter(|d| d.rule == rule).collect()
    }
}

/// Run all lint rules over a parsed PKGBUILD.
pub fn lint_pkgbuild(doc: &PkgbuildDoc) -> LintReport {
    let mut r = LintReport::default();
    macro_rules! err {
        ($rule:expr, $msg:expr, $line:expr) => {
            r.diagnostics.push(Diagnostic { rule: $rule, severity: Severity::Error, message: $msg.to_string(), line: $line })
        };
    }
    macro_rules! warn {
        ($rule:expr, $msg:expr, $line:expr) => {
            r.diagnostics.push(Diagnostic { rule: $rule, severity: Severity::Warning, message: $msg.to_string(), line: $line })
        };
    }

    // Required scalar fields.
    for field in ["pkgname", "pkgver", "pkgrel", "arch"] {
        if !doc.has(field) {
            err!(Rule::MissingField(field), &format!("missing required field `{field}`"), None);
        }
    }
    if !doc.has("source") {
        err!(Rule::MissingSource, "missing `source` array", None);
    }
    if !doc.has("url") {
        warn!(Rule::MissingUrl, "missing `url`", None);
    }
    if !doc.has("license") {
        warn!(Rule::BadLicense, "missing `license`", None);
    }

    // pkgrel must be integer.
    if let Some(rel) = doc.get("pkgrel") {
        let cleaned = rel.trim_matches(|c| c == '"' || c == '\'');
        if cleaned.parse::<u32>().is_err() || cleaned.parse::<u32>().unwrap_or(0) < 1 {
            let line = doc.assignments.iter().find(|a| a.name == "pkgrel").map(|a| a.line);
            err!(Rule::BadPkgrel, "`pkgrel` must be an integer >= 1", line);
        }
    }

    // arch must contain x86_64 or any.
    if let Some(arch) = doc.get("arch") {
        let ok = arch.contains("x86_64") || arch.contains("any");
        if !ok {
            let line = doc.assignments.iter().find(|a| a.name == "arch").map(|a| a.line);
            warn!(Rule::BadArch, "`arch` should include x86_64 or any", line);
        }
    }

    // pkgver() function vs pkgver= variable consistency.
    if doc.has_function("pkgver") && doc.has("pkgver") {
        warn!(Rule::PkgverStyle, "both `pkgver` variable and `pkgver()` defined", None);
    }

    // Non-free licenses.
    if let Some(lic) = doc.get("license") {
        for bad in ["nonfree", "custom:proprietary", "unknown"] {
            if lic.contains(bad) {
                let line = doc.assignments.iter().find(|a| a.name == "license").map(|a| a.line);
                err!(Rule::BadLicense, &format!("non-free license `{bad}`"), line);
            }
        }
    }

    // Integrity: if source has http(s) entries, some checksum array should exist.
    if let Some(src) = doc.get("source") {
        let has_remote = src.contains("http://") || src.contains("https://") || src.contains("git+");
        if has_remote
            && !doc.has("md5sums")
            && !doc.has("sha1sums")
            && !doc.has("sha256sums")
            && !doc.has("sha512sums")
            && !doc.has("b2sums")
        {
            warn!(Rule::MissingIntegrity, "remote sources without integrity sums", None);
        }
    }

    // Deprecated fields.
    for dep in ["packager", "groups"] {
        if doc.has(dep) {
            let line = doc.assignments.iter().find(|a| a.name == dep).map(|a| a.line);
            warn!(Rule::DeprecatedField(dep), &format!("`{dep}` is deprecated in PKGBUILD"), line);
        }
    }

    // Forbidden commands inside functions — check raw text not available here;
    // this rule is exercised via lint_pkgbuild_text below.
    r
}

/// Lint a raw PKGBUILD text (adds function-body checks).
pub fn lint_pkgbuild_text(text: &str) -> Result<LintReport, crate::LintError> {
    let doc = crate::pkgbuild::parse_pkgbuild(text)?;
    let mut report = lint_pkgbuild(&doc);
    // Scan function bodies for forbidden commands.
    for forbidden in ["sudo", "makepkg", "pacman -S"] {
        if text.contains(forbidden) {
            report.diagnostics.push(Diagnostic {
                rule: Rule::ForbiddenCommand(match forbidden {
                    "sudo" => "sudo",
                    "makepkg" => "makepkg",
                    _ => "pacman -S",
                }),
                severity: Severity::Error,
                message: format!("forbidden command `{forbidden}` in PKGBUILD"),
                line: None,
            });
        }
    }
    Ok(report)
}
