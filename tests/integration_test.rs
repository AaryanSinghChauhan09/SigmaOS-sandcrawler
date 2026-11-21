//! Integration tests for sigmaos-sandcrawler.

use sandcrawler::{Severity, lint_pkgbuild_text, parse_pkgbuild, lint_pkgbuild};

#[test]
fn test_clean_pkgbuild_passes() {
    let text = "pkgname=foo\npkgver=1.0\npkgrel=1\narch=('x86_64')\nsource=('https://x.tar.gz')\nsha256sums=('abc')\nlicense=('MIT')\nurl='https://x'\n";
    let doc = parse_pkgbuild(text).unwrap();
    let report = lint_pkgbuild(&doc);
    assert!(!report.has_errors());
}

#[test]
fn test_missing_required_fields() {
    let doc = parse_pkgbuild("pkgname=foo\n").unwrap();
    let report = lint_pkgbuild(&doc);
    assert!(report.has_errors());
    assert!(report.for_rule(sandcrawler::Rule::MissingField("pkgver")).len() > 0);
}

#[test]
fn test_bad_pkgrel() {
    let doc = parse_pkgbuild("pkgname=f\npkgver=1\npkgrel=0\narch=('x86_64')\nsource=()\nlicense=('MIT')\n").unwrap();
    let report = lint_pkgbuild(&doc);
    assert!(report.for_rule(sandcrawler::Rule::BadPkgrel).len() > 0);
}

#[test]
fn test_nonfree_license() {
    let doc = parse_pkgbuild("pkgname=f\npkgver=1\npkgrel=1\narch=('x86_64')\nsource=()\nlicense=('nonfree')\n").unwrap();
    let report = lint_pkgbuild(&doc);
    assert!(report.for_rule(sandcrawler::Rule::BadLicense).len() > 0);
}

#[test]
fn test_remote_source_without_sums() {
    let doc = parse_pkgbuild("pkgname=f\npkgver=1\npkgrel=1\narch=('x86_64')\nsource=('https://x')\nlicense=('MIT')\nurl='u'\n").unwrap();
    let report = lint_pkgbuild(&doc);
    assert!(report.for_rule(sandcrawler::Rule::MissingIntegrity).len() > 0);
}

#[test]
fn test_forbidden_sudo_in_function() {
    let text = "pkgname=f\npkgver=1\npkgrel=1\narch=('x86_64')\nsource=()\nlicense=('MIT')\nbuild() {\n  sudo make install\n}\n";
    let report = lint_pkgbuild_text(text).unwrap();
    assert!(report.for_rule(sandcrawler::Rule::ForbiddenCommand("sudo")).len() > 0);
}

#[test]
fn test_severity_grading() {
    let doc = parse_pkgbuild("pkgname=f\n").unwrap();
    let report = lint_pkgbuild(&doc);
    let has_error = report.diagnostics.iter().any(|d| d.severity == Severity::Error);
    assert!(has_error);
}
