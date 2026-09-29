//! Fixed simulation timestep.
//!
//! The simulation always advances in whole ticks of a fixed duration. The
//! reference engine runs a variable-rate simulation tied to frame rate; the
//! consequences of that difference are tracked in the parity matrix, not
//! hidden here.

use serde::{Deserialize, Serialize};

/// Fixed tick rate of a simulation instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickRate {
    hz: u32,
}

impl TickRate {
    /// Default rate used by tests and parity scenarios.
    pub const DEFAULT: TickRate = TickRate { hz: 60 };

    /// Returns `None` for a zero rate.
    pub fn new(hz: u32) -> Option<Self> {
        (hz > 0).then_some(Self { hz })
    }

    pub fn hz(self) -> u32 {
        self.hz
    }

    /// Duration of one tick in seconds.
    pub fn dt(self) -> f64 {
        1.0 / f64::from(self.hz)
    }

    /// Simulation time at the start of `tick`. Computed from the tick count
    /// rather than accumulated, so it does not drift.
    pub fn time_at(self, tick: u64) -> f64 {
        tick as f64 / f64::from(self.hz)
    }

    /// Number of whole ticks needed to cover `seconds` (rounded to nearest).
    pub fn ticks_for(self, seconds: f64) -> u64 {
        (seconds * f64::from(self.hz)).round().max(0.0) as u64
    }
}

impl Default for TickRate {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_rate_is_rejected() {
        assert!(TickRate::new(0).is_none());
    }

    #[test]
    fn time_does_not_drift() {
        let r = TickRate::new(60).expect("valid");
        assert_eq!(r.time_at(60 * 3600), 3600.0);
        assert_eq!(r.ticks_for(2.0), 120);
    }
}
