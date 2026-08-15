//! Authoring-time MathTex part splits (`{{...}}` and multi-string sources).
//!
//! These constructors run once when a formula is built. The per-frame path
//! only sees already-named groups.

/// Split one string on `{{...}}` markers.
/// `"a {{b}} c"` → `["a ", "b", " c"]`
/// `"no braces"` → `["no braces"]`
/// `"{{only}}"` → `["only"]`
pub fn split_double_brace_parts(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = source;
    loop {
        match rest.find("{{") {
            None => {
                if parts.is_empty() || !rest.is_empty() {
                    parts.push(rest.to_string());
                }
                break;
            }
            Some(start) => {
                let before = &rest[..start];
                if !before.is_empty() {
                    parts.push(before.to_string());
                }
                let after_open = &rest[start + 2..];
                match after_open.find("}}") {
                    Some(end) => {
                        parts.push(after_open[..end].to_string());
                        rest = &after_open[end + 2..];
                    }
                    None => {
                        // Unmatched `{{`: keep the remainder as one literal.
                        parts.push(rest[start..].to_string());
                        break;
                    }
                }
            }
        }
    }
    parts
}

/// Flatten a list of sources, expanding `{{ }}` in each.
/// Empty input → `vec![""]`.
pub fn expand_tex_parts(sources: &[String]) -> Vec<String> {
    if sources.is_empty() {
        return vec![String::new()];
    }
    sources
        .iter()
        .flat_map(|s| split_double_brace_parts(s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_examples() {
        assert_eq!(split_double_brace_parts("a {{b}} c"), vec!["a ", "b", " c"]);
        assert_eq!(split_double_brace_parts("no braces"), vec!["no braces"]);
        assert_eq!(split_double_brace_parts("{{only}}"), vec!["only"]);
    }

    #[test]
    fn unmatched_brace_is_literal() {
        assert_eq!(split_double_brace_parts("foo {{bar"), vec!["foo ", "{{bar"]);
    }

    #[test]
    fn expand_empty_is_one_blank() {
        assert_eq!(expand_tex_parts(&[]), vec![""]);
    }
}
