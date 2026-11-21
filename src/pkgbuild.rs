//! PKGBUILD text parser for linting.
//!
//! Extracts top-level `key=value` / `key=(...)` assignments and function
//! definitions (`name() { ... }`) without executing anything.

/// A single top-level assignment.
#[derive(Debug, Clone)]
pub struct Assignment {
    /// Variable/function name.
    pub name: String,
    /// Raw value text (trimmed).
    pub value: String,
    /// Line number (1-based).
    pub line: usize,
}

/// Parsed PKGBUILD document.
#[derive(Debug, Default)]
pub struct PkgbuildDoc {
    /// Top-level assignments in file order.
    pub assignments: Vec<Assignment>,
    /// Function names defined (`pkgver`, `prepare`, `build`, ...).
    pub functions: Vec<String>,
}

impl PkgbuildDoc {
    /// Get the last assignment value for `name`.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.assignments
            .iter()
            .rev()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Whether `name` was assigned at all.
    pub fn has(&self, name: &str) -> bool {
        self.assignments.iter().any(|a| a.name == name)
    }

    /// All values for `name` (in case of re-assignment).
    pub fn get_all(&self, name: &str) -> Vec<&str> {
        self.assignments
            .iter()
            .filter(|a| a.name == name)
            .map(|a| a.value.as_str())
            .collect()
    }

    /// Whether function `name` is defined.
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.iter().any(|f| f == name)
    }
}

/// Parse PKGBUILD text.
pub fn parse_pkgbuild(text: &str) -> Result<PkgbuildDoc, crate::LintError> {
    let mut doc = PkgbuildDoc::default();
    let mut in_func = false;
    let mut func_depth = 0i32;

    for (i, raw) in text.lines().enumerate() {
        let line_no = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if in_func {
            // Track brace depth to know when function ends.
            func_depth += line.matches('{').count() as i32;
            func_depth -= line.matches('}').count() as i32;
            if func_depth <= 0 {
                in_func = false;
            }
            continue;
        }
        // Function definition: name() { or name () {
        if let Some(pos) = line.find('(') {
            if line[..pos].chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !line[..pos].is_empty()
                && line[pos..].starts_with("()")
            {
                let name = line[..pos].to_string();
                if name != "if" && name != "while" && name != "for" && name != "case" {
                    doc.functions.push(name);
                    in_func = true;
                    func_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
                    if func_depth <= 0 {
                        in_func = false;
                    }
                    continue;
                }
            }
        }
        // Assignment: NAME=value (possibly array).
        if let Some(eq) = find_assignment_eq(line) {
            let name = line[..eq].trim().to_string();
            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !name.is_empty() {
                let value = strip_comment(&line[eq + 1..]).trim().to_string();
                doc.assignments.push(Assignment { name, value, line: line_no });
            }
        }
    }
    Ok(doc)
}

/// Read and parse a PKGBUILD file.
pub fn parse_pkgbuild_file(path: &std::path::Path) -> Result<PkgbuildDoc, crate::LintError> {
    let text = std::fs::read_to_string(path)?;
    parse_pkgbuild(&text)
}

fn find_assignment_eq(line: &str) -> Option<usize> {
    // First '=' not inside quotes and preceded by a valid name char.
    let mut in_squote = false;
    let mut in_dquote = false;
    for (i, c) in line.char_indices() {
        match c {
            '\'' if !in_dquote => in_squote = !in_squote,
            '"' if !in_squote => in_dquote = !in_dquote,
            '=' if !in_squote && !in_dquote => return Some(i),
            _ => {}
        }
    }
    None
}

fn strip_comment(s: &str) -> &str {
    let mut in_s = false;
    let mut in_d = false;
    for (i, c) in s.char_indices() {
        match c {
            '\'' if !in_d => in_s = !in_s,
            '"' if !in_s => in_d = !in_d,
            '#' if !in_s && !in_d && (i == 0 || s.as_bytes()[i - 1].is_ascii_whitespace()) => {
                return &s[..i]
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let text = r#"
pkgname=foo
pkgver=1.0
pkgrel=1
arch=('x86_64' 'any')
depend=("a" "b>=1") # comment
pkgver() { echo 1.0; }
build() {
  make
}
"#;
        let doc = parse_pkgbuild(text).unwrap();
        assert_eq!(doc.get("pkgname"), Some("foo"));
        assert_eq!(doc.get("arch"), Some("('x86_64' 'any')"));
        assert!(doc.has_function("pkgver"));
        assert!(doc.has_function("build"));
        assert!(!doc.has_function("prepare"));
    }

    #[test]
    fn test_assignment_in_function_ignored() {
        let text = "build() {\n  FOO=bar\n}\n";
        let doc = parse_pkgbuild(text).unwrap();
        assert!(!doc.has("FOO"));
    }
}
