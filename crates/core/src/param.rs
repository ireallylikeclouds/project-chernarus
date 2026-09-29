//! Data-driven parameters that carry their own knowledge status.
//!
//! Every tunable number that is meant to reproduce reference behaviour is a
//! [`Param`]. The simulation reads `value`; tooling and parity reports read
//! `status` and `source` so a result computed from placeholders can never be
//! reported as more trustworthy than those placeholders.

use crate::Verification;
use serde::{Deserialize, Serialize};

/// A value together with how we know it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param<T> {
    /// The value used by the implementation. For `UNKNOWN` this is a placeholder.
    pub value: T,
    /// How well the value has been checked against the reference.
    pub status: Verification,
    /// Where the value came from: a measurement id, a reference file and
    /// field, a document, or an explicit "placeholder" note.
    pub source: String,
}

impl<T> Param<T> {
    pub fn new(value: T, status: Verification, source: impl Into<String>) -> Self {
        Self { value, status, source: source.into() }
    }

    /// A placeholder: a value with no claim about the reference.
    pub fn placeholder(value: T, why: impl Into<String>) -> Self {
        Self::new(value, Verification::Unknown, why)
    }
}

/// A named parameter as seen by reporting code, independent of its value type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParamUse {
    pub name: String,
    pub status: Verification,
    pub source: String,
}

impl ParamUse {
    pub fn of<T>(name: impl Into<String>, param: &Param<T>) -> Self {
        Self { name: name.into(), status: param.status, source: param.source.clone() }
    }
}
