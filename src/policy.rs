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
