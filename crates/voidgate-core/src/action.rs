//! Closed set of decisions. The order is the severity: `Allow < Mask < Block`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "lowercase")
)]
pub enum Action {
    /// Nothing found: the text can go on as is.
    Allow,
    /// Only personal data found: send [`crate::redact`]ed text instead.
    Mask,
    /// Manipulation or hidden characters found: do not send the text.
    Block,
}

impl Action {
    /// `"allow"`, `"mask"` or `"block"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Mask => "mask",
            Self::Block => "block",
        }
    }
}
