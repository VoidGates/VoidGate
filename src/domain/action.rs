//! Closed set of decisions shared by chat, proxy and event records.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Action {
    Allow,
    Mask,
    Block,
}

impl Action {
    pub(crate) fn severity(self) -> u8 {
        match self {
            Self::Allow => 0,
            Self::Mask => 1,
            Self::Block => 2,
        }
    }
}
