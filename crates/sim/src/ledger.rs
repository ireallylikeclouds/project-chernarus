//! Record of which data parameters influenced a simulation run.

use chernarus_core::param::ParamUse;
use chernarus_core::{Param, Verification};
use std::collections::BTreeMap;

/// Parameters read during a run, keyed by their data path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParamLedger {
    used: BTreeMap<String, ParamUse>,
}

impl ParamLedger {
    /// Record a use. Allocates only the first time a key is seen.
    pub fn record<T>(&mut self, key: &str, param: &Param<T>) {
        if !self.used.contains_key(key) {
            self.used.insert(key.to_owned(), ParamUse::of(key, param));
        }
    }

    pub fn uses(&self) -> impl Iterator<Item = &ParamUse> {
        self.used.values()
    }

    /// The weakest status among the parameters used (`Unknown` if none).
    pub fn weakest(&self) -> Verification {
        Verification::weakest(self.used.values().map(|u| u.status))
    }

    pub fn is_empty(&self) -> bool {
        self.used.is_empty()
    }
}
