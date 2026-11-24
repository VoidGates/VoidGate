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
    /// At least one finding asks to hide its span and none asks to stop the
    /// text: send [`crate::redact`]ed text instead. Built in: personal data.
    Mask,
    /// At least one finding asks to stop the text: do not send it. Built in:
    /// manipulation and hidden characters.
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
