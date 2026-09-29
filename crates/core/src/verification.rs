//! Knowledge status of a fact, value or behaviour relative to the reference game.
//!
//! See `docs/reference/README.md` for the full definitions. The ordering of the
//! variants is meaningful: it runs from least to most trustworthy, so the
//! confidence of a derived result is the minimum over its inputs.

use serde::{Deserialize, Serialize};
use std::fmt;

/// How well a piece of knowledge has been checked against the reference game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verification {
    /// Nothing is known. Any value attached is a placeholder chosen only so
    /// code can run; it carries no claim about the reference.
    Unknown,
    /// Taken from secondary sources (community documentation, memory,
    /// reasoning about related systems) but not yet checked against the
    /// reference installation.
    Estimated,
    /// Checked against the reference, but only under limited conditions or
    /// with limited precision. The source must say what was and was not checked.
    #[serde(alias = "PARTIALLY VERIFIED")]
    PartiallyVerified,
    /// Reproducibly measured or extracted from the reference with a documented
    /// method, and covered by a comparison test.
    Verified,
}

impl Verification {
    /// All statuses, least trustworthy first.
    pub const ALL: [Verification; 4] =
        [Verification::Unknown, Verification::Estimated, Verification::PartiallyVerified, Verification::Verified];

    /// The status as written in documentation (`PARTIALLY VERIFIED` with a space).
    pub fn label(self) -> &'static str {
        match self {
            Verification::Unknown => "UNKNOWN",
            Verification::Estimated => "ESTIMATED",
            Verification::PartiallyVerified => "PARTIALLY VERIFIED",
            Verification::Verified => "VERIFIED",
        }
    }

    /// Confidence of a result derived from several inputs: never better than
    /// the weakest input. An empty input set is `Unknown`.
    pub fn weakest<I: IntoIterator<Item = Verification>>(inputs: I) -> Verification {
        inputs.into_iter().min().unwrap_or(Verification::Unknown)
    }
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_runs_from_least_to_most_trustworthy() {
        assert!(Verification::Unknown < Verification::Estimated);
        assert!(Verification::Estimated < Verification::PartiallyVerified);
        assert!(Verification::PartiallyVerified < Verification::Verified);
    }

    #[test]
    fn weakest_input_bounds_confidence() {
        use Verification::*;
        assert_eq!(Verification::weakest([Verified, Estimated, PartiallyVerified]), Estimated);
        assert_eq!(Verification::weakest([]), Unknown);
    }

    #[test]
    fn serde_accepts_documentation_spelling() {
        #[derive(Deserialize)]
        struct Doc {
            a: Verification,
            b: Verification,
        }
        let doc: Doc =
            toml::from_str("a = \"PARTIALLY VERIFIED\"\nb = \"PARTIALLY_VERIFIED\"").expect("both spellings parse");
        assert_eq!(doc.a, Verification::PartiallyVerified);
        assert_eq!(doc.b, Verification::PartiallyVerified);
    }
}
