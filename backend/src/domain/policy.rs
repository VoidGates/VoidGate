use std::collections::HashSet;

pub(crate) struct ExactPathPolicy {
    decoy_paths: HashSet<String>,
}

pub(crate) enum Decision {
    Allow,
    Decoy,
}

impl ExactPathPolicy {
    pub(crate) fn new(decoy_paths: Vec<String>) -> Result<Self, String> {
        let mut paths = HashSet::new();
        for path in decoy_paths {
            if path == "/" || !valid_path(&path) {
                return Err(format!("invalid decoy path: {path}"));
            }
            paths.insert(path);
        }
        Ok(Self { decoy_paths: paths })
    }

    pub(crate) fn evaluate(&self, path: &str) -> Decision {
        if self.decoy_paths.contains(path) {
            Decision::Decoy
        } else {
            Decision::Allow
        }
    }
}

pub(crate) fn valid_path(path: &str) -> bool {
    path.starts_with('/')
        && path.len() <= 2048
        && !path.contains("//")
        && !path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        && path.bytes().all(|byte| {
            (0x21..=0x7e).contains(&byte) && !matches!(byte, b'?' | b'#' | b'\\' | b'%')
        })
}

#[cfg(test)]
mod tests {
    use super::{valid_path, ExactPathPolicy};

    #[test]
    fn accepts_plain_absolute_paths_up_to_2048_bytes() {
        for path in ["/", "/a", "/internal/backup", "/a-b_c.d~e/f;g=h"] {
            assert!(valid_path(path), "{path}");
        }
        assert!(valid_path(&format!("/{}", "a".repeat(2047))));
        assert!(!valid_path(&format!("/{}", "a".repeat(2048))));
    }

    #[test]
    fn rejects_ambiguous_or_encoded_paths() {
        for path in [
            "",
            "relative",
            "/a?b",
            "/a#b",
            "/a%2fb",
            "/a\\b",
            "/a//b",
            "/a/./b",
            "/a/../b",
            "/a/.",
            "/a/..",
            "/a b",
            "/a\tb",
            "/a\u{0000}b",
            "/caf\u{00E9}",
            "/a\u{007F}",
        ] {
            assert!(!valid_path(path), "{}", path.escape_default());
        }
    }

    #[test]
    fn decoy_list_rejects_the_root_and_invalid_paths() {
        assert!(ExactPathPolicy::new(vec!["/internal/backup".into()]).is_ok());
        for path in ["/", "relative", "/a//b", "/a%2fb"] {
            assert!(ExactPathPolicy::new(vec![path.into()]).is_err(), "{path}");
        }
    }
}
