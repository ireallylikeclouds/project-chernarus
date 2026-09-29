//! Simulation container: fixed-step clock plus the units it advances.
//!
//! Framework-free by design (ADR 0002): state lives in plain typed
//! collections and systems are called in an explicit order each tick.

use crate::ledger::ParamLedger;
use crate::movement::{MoveIntent, MovementParams, PlayerMotion};
use chernarus_core::time::TickRate;
use std::sync::Arc;

/// Index of a player in a [`Simulation`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u32);

/// A deterministic, headless simulation instance.
#[derive(Debug, Clone)]
pub struct Simulation {
    rate: TickRate,
    tick: u64,
    movement: Arc<MovementParams>,
    players: Vec<PlayerMotion>,
    ledger: ParamLedger,
}

impl Simulation {
    pub fn new(rate: TickRate, movement: Arc<MovementParams>) -> Self {
        Self { rate, tick: 0, movement, players: Vec::new(), ledger: ParamLedger::default() }
    }

    pub fn spawn_player(&mut self, motion: PlayerMotion) -> PlayerId {
        self.players.push(motion);
        PlayerId(self.players.len() as u32 - 1)
    }

    pub fn player(&self, id: PlayerId) -> Option<&PlayerMotion> {
        self.players.get(id.0 as usize)
    }

    pub fn rate(&self) -> TickRate {
        self.rate
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Simulation time at the current tick, seconds.
    pub fn time(&self) -> f64 {
        self.rate.time_at(self.tick)
    }

    /// Parameters used so far.
    pub fn ledger(&self) -> &ParamLedger {
        &self.ledger
    }

    /// Advance one tick. `intents[i]` drives player `i`; missing intents
    /// leave a player idle in place with its current stance and heading.
    pub fn step(&mut self, intents: &[MoveIntent]) {
        let dt = self.rate.dt();
        for (i, player) in self.players.iter_mut().enumerate() {
            let intent = intents.get(i).copied().unwrap_or_else(|| MoveIntent::idle(player.stance, player.heading_deg));
            player.step(&self.movement, &intent, dt, &mut self.ledger);
        }
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::tests::test_params;
    use crate::movement::{Pace, Stance};

    fn scripted_run() -> Simulation {
        let mut sim = Simulation::new(TickRate::DEFAULT, Arc::new(test_params()));
        let a = sim.spawn_player(PlayerMotion::at([10.0, 20.0, 0.0], 45.0, Stance::Stand));
        sim.spawn_player(PlayerMotion::at([0.0; 3], 0.0, Stance::Crouch));
        for t in 0..600u32 {
            let intent = MoveIntent {
                forward: if t < 300 { 1 } else { 0 },
                right: if t % 97 < 40 { 1 } else { 0 },
                pace: if t < 150 { Pace::Run } else { Pace::Sprint },
                stance: Stance::Stand,
                heading_deg: 45.0 + f64::from(t) * 0.3,
            };
            sim.step(&[intent]);
        }
        assert!(sim.player(a).is_some());
        sim
    }

    #[test]
    fn identical_inputs_give_bit_identical_state() {
        let (a, b) = (scripted_run(), scripted_run());
        for i in 0..2 {
            let (pa, pb) = (a.player(PlayerId(i)), b.player(PlayerId(i)));
            assert_eq!(pa.map(|p| p.position.map(f64::to_bits)), pb.map(|p| p.position.map(f64::to_bits)));
        }
        assert_eq!(a.tick(), 600);
        assert_eq!(a.time(), 10.0);
    }

    #[test]
    fn players_without_intent_stay_idle() {
        let sim = scripted_run();
        let second = sim.player(PlayerId(1)).expect("spawned");
        assert_eq!(second.position, [0.0; 3]);
        assert_eq!(second.stance, Stance::Crouch);
    }
}
