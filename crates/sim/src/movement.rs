//! Infantry movement on flat ground.
//!
//! Parity level P1 (conceptually implemented). The reference engine derives
//! infantry motion from animations: each movement state in the `CfgMoves*`
//! configs plays an animation whose root displacement moves the unit, and
//! state changes blend between animations. That structure is ESTIMATED
//! (`docs/reference/systems/movement.md`) and is *not* reproduced yet. This
//! model is a kinematic stand-in with the same inputs and observable outputs
//! (position and heading over time) so the measurement and comparison chain
//! can run end to end:
//!
//! * a target speed per (stance, pace, direction class) from data;
//! * velocity approaches the target at a bounded rate (acceleration when
//!   speeding up, deceleration when slowing down);
//! * stance changes take a fixed time from data, during which the unit does
//!   not translate.
//!
//! Every number comes from `data/movement/infantry.toml`, where all values
//! are currently UNKNOWN placeholders.

use crate::ledger::ParamLedger;
use chernarus_core::Param;
use chernarus_core::coords::{heading_forward, heading_right, normalize_heading};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Body stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    Stand,
    Crouch,
    Prone,
}

impl Stance {
    pub fn key(self) -> &'static str {
        match self {
            Stance::Stand => "stand",
            Stance::Crouch => "crouch",
            Stance::Prone => "prone",
        }
    }
}

/// Movement pace selected by the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pace {
    Walk,
    /// The default pace when a movement key is held.
    Run,
    Sprint,
}

impl Pace {
    pub fn key(self) -> &'static str {
        match self {
            Pace::Walk => "walk",
            Pace::Run => "run",
            Pace::Sprint => "sprint",
        }
    }
}

/// Movement direction relative to the body, with left/right folded together
/// (assumed symmetric; UNKNOWN).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Forward,
    ForwardDiagonal,
    Sideways,
    BackwardDiagonal,
    Backward,
}

impl Direction {
    pub fn key(self) -> &'static str {
        match self {
            Direction::Forward => "forward",
            Direction::ForwardDiagonal => "forward_diagonal",
            Direction::Sideways => "sideways",
            Direction::BackwardDiagonal => "backward_diagonal",
            Direction::Backward => "backward",
        }
    }

    /// Classify a movement-key combination. `None` when no key is held.
    pub fn classify(forward: i8, right: i8) -> Option<Direction> {
        match (forward.signum(), right.signum()) {
            (0, 0) => None,
            (1, 0) => Some(Direction::Forward),
            (1, _) => Some(Direction::ForwardDiagonal),
            (0, _) => Some(Direction::Sideways),
            (-1, 0) => Some(Direction::Backward),
            _ => Some(Direction::BackwardDiagonal),
        }
    }
}

/// Target speeds (m/s) for one stance and pace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionSpeeds {
    pub forward: Param<f64>,
    pub forward_diagonal: Param<f64>,
    pub sideways: Param<f64>,
    pub backward_diagonal: Param<f64>,
    pub backward: Param<f64>,
}

impl DirectionSpeeds {
    fn get(&self, d: Direction) -> &Param<f64> {
        match d {
            Direction::Forward => &self.forward,
            Direction::ForwardDiagonal => &self.forward_diagonal,
            Direction::Sideways => &self.sideways,
            Direction::BackwardDiagonal => &self.backward_diagonal,
            Direction::Backward => &self.backward,
        }
    }

    fn all(&self) -> [(Direction, &Param<f64>); 5] {
        [
            Direction::Forward,
            Direction::ForwardDiagonal,
            Direction::Sideways,
            Direction::BackwardDiagonal,
            Direction::Backward,
        ]
        .map(|d| (d, self.get(d)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaceSpeeds {
    pub walk: DirectionSpeeds,
    pub run: DirectionSpeeds,
    pub sprint: DirectionSpeeds,
}

impl PaceSpeeds {
    fn get(&self, p: Pace) -> &DirectionSpeeds {
        match p {
            Pace::Walk => &self.walk,
            Pace::Run => &self.run,
            Pace::Sprint => &self.sprint,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StanceSpeeds {
    pub stand: PaceSpeeds,
    pub crouch: PaceSpeeds,
    pub prone: PaceSpeeds,
}

impl StanceSpeeds {
    fn get(&self, s: Stance) -> &PaceSpeeds {
        match s {
            Stance::Stand => &self.stand,
            Stance::Crouch => &self.crouch,
            Stance::Prone => &self.prone,
        }
    }
}

/// How quickly velocity follows the target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    /// Maximum rate of speed increase, m/s².
    pub acceleration_mps2: Param<f64>,
    /// Maximum rate of speed decrease, m/s².
    pub deceleration_mps2: Param<f64>,
}

/// Duration of each stance change, seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StanceTransitions {
    pub stand_to_crouch: Param<f64>,
    pub crouch_to_stand: Param<f64>,
    pub stand_to_prone: Param<f64>,
    pub prone_to_stand: Param<f64>,
    pub crouch_to_prone: Param<f64>,
    pub prone_to_crouch: Param<f64>,
}

impl StanceTransitions {
    fn get(&self, from: Stance, to: Stance) -> Option<(&'static str, &Param<f64>)> {
        use Stance::*;
        Some(match (from, to) {
            (Stand, Crouch) => ("stance_transition_s.stand_to_crouch", &self.stand_to_crouch),
            (Crouch, Stand) => ("stance_transition_s.crouch_to_stand", &self.crouch_to_stand),
            (Stand, Prone) => ("stance_transition_s.stand_to_prone", &self.stand_to_prone),
            (Prone, Stand) => ("stance_transition_s.prone_to_stand", &self.prone_to_stand),
            (Crouch, Prone) => ("stance_transition_s.crouch_to_prone", &self.crouch_to_prone),
            (Prone, Crouch) => ("stance_transition_s.prone_to_crouch", &self.prone_to_crouch),
            _ => return None,
        })
    }
}

/// Complete infantry movement definition (`data/movement/infantry.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementParams {
    pub schema: u32,
    pub response: Response,
    pub stance_transition_s: StanceTransitions,
    pub speed_mps: StanceSpeeds,
}

#[derive(Debug, Error)]
pub enum ParamsError {
    #[error("cannot read {0}: {1}")]
    Io(String, std::io::Error),
    #[error("cannot parse movement parameters: {0}")]
    Parse(#[from] Box<toml::de::Error>),
    #[error("unsupported schema {0} (expected 1)")]
    Schema(u32),
    #[error("invalid value for {key}: {detail}")]
    Invalid { key: String, detail: String },
}

impl MovementParams {
    pub fn load(path: &Path) -> Result<Self, ParamsError> {
        let text = std::fs::read_to_string(path).map_err(|e| ParamsError::Io(path.display().to_string(), e))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, ParamsError> {
        let params: Self = toml::from_str(text).map_err(Box::new)?;
        if params.schema != 1 {
            return Err(ParamsError::Schema(params.schema));
        }
        params.validate()?;
        Ok(params)
    }

    fn validate(&self) -> Result<(), ParamsError> {
        for (key, p) in self.all() {
            // Accelerations must be positive; durations and speeds may be zero.
            let strictly_positive = key.starts_with("response.");
            let v = p.value;
            let ok = v.is_finite() && if strictly_positive { v > 0.0 } else { v >= 0.0 };
            if !ok {
                let kind = if strictly_positive { "positive" } else { "non-negative" };
                return Err(ParamsError::Invalid { key, detail: format!("{v} is not a {kind} finite number") });
            }
        }
        Ok(())
    }

    /// Every parameter with its data path, in a stable order.
    pub fn all(&self) -> Vec<(String, &Param<f64>)> {
        let mut out = vec![
            ("response.acceleration_mps2".to_owned(), &self.response.acceleration_mps2),
            ("response.deceleration_mps2".to_owned(), &self.response.deceleration_mps2),
        ];
        let stances = [Stance::Stand, Stance::Crouch, Stance::Prone];
        for from in stances {
            for to in stances {
                if let Some((key, p)) = self.stance_transition_s.get(from, to) {
                    out.push((key.to_owned(), p));
                }
            }
        }
        for stance in stances {
            for pace in [Pace::Walk, Pace::Run, Pace::Sprint] {
                for (dir, p) in self.speed_mps.get(stance).get(pace).all() {
                    out.push((Self::speed_key(stance, pace, dir), p));
                }
            }
        }
        out
    }

    /// Data path of a speed parameter, as used in ledgers and reports.
    pub fn speed_key(stance: Stance, pace: Pace, dir: Direction) -> String {
        format!("speed_mps.{}.{}.{}", stance.key(), pace.key(), dir.key())
    }

    pub fn speed(&self, stance: Stance, pace: Pace, dir: Direction) -> &Param<f64> {
        self.speed_mps.get(stance).get(pace).get(dir)
    }
}

/// What the player asks for this tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveIntent {
    /// +1 forward key, -1 backward key, 0 neither.
    pub forward: i8,
    /// +1 right key, -1 left key, 0 neither.
    pub right: i8,
    pub pace: Pace,
    /// Desired stance; a change starts a transition.
    pub stance: Stance,
    /// Body heading, degrees clockwise from north.
    pub heading_deg: f64,
}

impl MoveIntent {
    pub fn idle(stance: Stance, heading_deg: f64) -> Self {
        Self { forward: 0, right: 0, pace: Pace::Run, stance, heading_deg }
    }
}

/// An in-progress stance change.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StanceTransition {
    pub to: Stance,
    pub remaining_s: f64,
}

/// Kinematic state of one infantry unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMotion {
    /// World position (x east, y north, z up), metres.
    pub position: [f64; 3],
    /// Degrees clockwise from north.
    pub heading_deg: f64,
    /// Horizontal velocity (east, north), m/s.
    pub velocity: [f64; 2],
    pub stance: Stance,
    pub transition: Option<StanceTransition>,
}

impl PlayerMotion {
    pub fn at(position: [f64; 3], heading_deg: f64, stance: Stance) -> Self {
        Self { position, heading_deg: normalize_heading(heading_deg), velocity: [0.0, 0.0], stance, transition: None }
    }

    pub fn speed(&self) -> f64 {
        self.velocity[0].hypot(self.velocity[1])
    }

    /// Advance by one fixed step of `dt` seconds.
    pub fn step(&mut self, params: &MovementParams, intent: &MoveIntent, dt: f64, ledger: &mut ParamLedger) {
        self.heading_deg = normalize_heading(intent.heading_deg);

        // Stance change: starts only when none is in progress (UNKNOWN rule).
        if self.transition.is_none()
            && intent.stance != self.stance
            && let Some((key, duration)) = params.stance_transition_s.get(self.stance, intent.stance)
        {
            ledger.record(key, duration);
            self.transition = Some(StanceTransition { to: intent.stance, remaining_s: duration.value });
        }

        let target = match (self.transition, Direction::classify(intent.forward, intent.right)) {
            (Some(_), _) | (None, None) => [0.0, 0.0],
            (None, Some(dir)) => {
                let key = MovementParams::speed_key(self.stance, intent.pace, dir);
                let speed = params.speed(self.stance, intent.pace, dir);
                ledger.record(&key, speed);
                let f = heading_forward(self.heading_deg);
                let r = heading_right(self.heading_deg);
                let (kf, kr) = (f64::from(intent.forward.signum()), f64::from(intent.right.signum()));
                let norm = kf.hypot(kr);
                [speed.value * (f[0] * kf + r[0] * kr) / norm, speed.value * (f[1] * kf + r[1] * kr) / norm]
            }
        };

        let dv = [target[0] - self.velocity[0], target[1] - self.velocity[1]];
        let dv_len = dv[0].hypot(dv[1]);
        if dv_len > 0.0 {
            let speeding_up = target[0].hypot(target[1]) > self.speed();
            let (key, rate) = if speeding_up {
                ("response.acceleration_mps2", &params.response.acceleration_mps2)
            } else {
                ("response.deceleration_mps2", &params.response.deceleration_mps2)
            };
            ledger.record(key, rate);
            let max_step = rate.value * dt;
            if dv_len <= max_step {
                self.velocity = target;
            } else {
                self.velocity[0] += dv[0] / dv_len * max_step;
                self.velocity[1] += dv[1] / dv_len * max_step;
            }
        }

        // Semi-implicit Euler; flat ground at z = const.
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;

        if let Some(t) = &mut self.transition {
            t.remaining_s -= dt;
            if t.remaining_s <= 1e-9 {
                self.stance = t.to;
                self.transition = None;
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chernarus_core::Verification;

    /// Round-number test parameters, independent of the data file.
    pub(crate) fn test_params() -> MovementParams {
        let p = |v: f64| Param::new(v, Verification::Unknown, "unit-test value");
        let dirs = |base: f64| DirectionSpeeds {
            forward: p(base),
            forward_diagonal: p(base * 0.9),
            sideways: p(base * 0.8),
            backward_diagonal: p(base * 0.7),
            backward: p(base * 0.6),
        };
        let paces =
            |scale: f64| PaceSpeeds { walk: dirs(1.0 * scale), run: dirs(4.0 * scale), sprint: dirs(6.0 * scale) };
        MovementParams {
            schema: 1,
            response: Response { acceleration_mps2: p(8.0), deceleration_mps2: p(16.0) },
            stance_transition_s: StanceTransitions {
                stand_to_crouch: p(0.5),
                crouch_to_stand: p(0.5),
                stand_to_prone: p(1.0),
                prone_to_stand: p(1.5),
                crouch_to_prone: p(0.75),
                prone_to_crouch: p(1.0),
            },
            speed_mps: StanceSpeeds { stand: paces(1.0), crouch: paces(0.5), prone: paces(0.25) },
        }
    }

    fn run(motion: &mut PlayerMotion, intent: MoveIntent, seconds: f64, ledger: &mut ParamLedger) {
        let params = test_params();
        let dt = 1.0 / 60.0;
        for _ in 0..(seconds * 60.0).round() as usize {
            motion.step(&params, &intent, dt, ledger);
        }
    }

    fn forward(pace: Pace, heading: f64) -> MoveIntent {
        MoveIntent { forward: 1, right: 0, pace, stance: Stance::Stand, heading_deg: heading }
    }

    #[test]
    fn direction_classification() {
        assert_eq!(Direction::classify(0, 0), None);
        assert_eq!(Direction::classify(1, 0), Some(Direction::Forward));
        assert_eq!(Direction::classify(1, -1), Some(Direction::ForwardDiagonal));
        assert_eq!(Direction::classify(0, 1), Some(Direction::Sideways));
        assert_eq!(Direction::classify(-1, 1), Some(Direction::BackwardDiagonal));
        assert_eq!(Direction::classify(-1, 0), Some(Direction::Backward));
    }

    #[test]
    fn idle_unit_does_not_move() {
        let mut m = PlayerMotion::at([100.0, 200.0, 0.0], 0.0, Stance::Stand);
        let mut ledger = ParamLedger::default();
        run(&mut m, MoveIntent::idle(Stance::Stand, 0.0), 2.0, &mut ledger);
        assert_eq!(m.position, [100.0, 200.0, 0.0]);
        assert!(ledger.is_empty());
    }

    #[test]
    fn reaches_target_speed_after_acceleration_time() {
        let mut m = PlayerMotion::at([0.0; 3], 0.0, Stance::Stand);
        let mut ledger = ParamLedger::default();
        // Run speed 4 m/s at 8 m/s² → full speed after 0.5 s.
        run(&mut m, forward(Pace::Run, 0.0), 0.25, &mut ledger);
        assert!((m.speed() - 2.0).abs() < 1e-9, "{}", m.speed());
        run(&mut m, forward(Pace::Run, 0.0), 1.0, &mut ledger);
        assert!((m.speed() - 4.0).abs() < 1e-12);
        let keys: Vec<_> = ledger.uses().map(|u| u.name.as_str()).collect();
        assert_eq!(keys, ["response.acceleration_mps2", "speed_mps.stand.run.forward"]);
        assert_eq!(ledger.weakest(), Verification::Unknown);
    }

    #[test]
    fn heading_follows_reference_convention() {
        let mut ledger = ParamLedger::default();
        let mut east = PlayerMotion::at([0.0; 3], 90.0, Stance::Stand);
        run(&mut east, forward(Pace::Walk, 90.0), 2.0, &mut ledger);
        assert!(east.position[0] > 1.0 && east.position[1].abs() < 1e-9);

        let mut strafe = PlayerMotion::at([0.0; 3], 0.0, Stance::Stand);
        let right = MoveIntent { forward: 0, right: 1, ..forward(Pace::Walk, 0.0) };
        run(&mut strafe, right, 2.0, &mut ledger);
        assert!(strafe.position[0] > 1.0 && strafe.position[1].abs() < 1e-9);
    }

    #[test]
    fn stopping_uses_deceleration() {
        let mut m = PlayerMotion::at([0.0; 3], 0.0, Stance::Stand);
        let mut ledger = ParamLedger::default();
        run(&mut m, forward(Pace::Run, 0.0), 2.0, &mut ledger);
        let before = m.position[1];
        run(&mut m, MoveIntent::idle(Stance::Stand, 0.0), 1.0, &mut ledger);
        // 4 m/s at 16 m/s² stops in 0.25 s covering ~0.5 m.
        let slide = m.position[1] - before;
        assert!((slide - 0.5).abs() < 0.05, "slide {slide}");
        assert_eq!(m.speed(), 0.0);
    }

    #[test]
    fn stance_change_blocks_translation_then_applies() {
        let mut m = PlayerMotion::at([0.0; 3], 0.0, Stance::Stand);
        let mut ledger = ParamLedger::default();
        let go_prone = MoveIntent { stance: Stance::Prone, ..forward(Pace::Run, 0.0) };
        run(&mut m, go_prone, 0.5, &mut ledger);
        assert_eq!(m.position, [0.0; 3]);
        assert_eq!(m.stance, Stance::Stand);
        run(&mut m, go_prone, 0.6, &mut ledger);
        assert_eq!(m.stance, Stance::Prone);
        run(&mut m, go_prone, 2.0, &mut ledger);
        assert!(m.position[1] > 0.5);
        assert!(ledger.uses().any(|u| u.name == "stance_transition_s.stand_to_prone"));
        assert!(ledger.uses().any(|u| u.name == "speed_mps.prone.run.forward"));
    }

    #[test]
    fn invalid_values_are_rejected() {
        let mut params = test_params();
        params.response.acceleration_mps2.value = 0.0;
        assert!(params.validate().is_err());
        let mut params = test_params();
        params.speed_mps.crouch.sprint.backward.value = f64::NAN;
        assert!(params.validate().is_err());
    }
}

#[cfg(test)]
mod data_file_tests {
    use super::*;
    use chernarus_core::Verification;

    #[test]
    fn shipped_data_file_loads_and_is_honest() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/movement/infantry.toml");
        let params = MovementParams::load(&path).expect("data/movement/infantry.toml is valid");
        let all = params.all();
        assert_eq!(all.len(), 2 + 6 + 3 * 3 * 5, "every parameter is present");
        // A raised status must come with a source that is not a placeholder note.
        for (key, p) in all {
            if p.status > Verification::Unknown {
                assert!(!p.source.starts_with("placeholder"), "{key}: status raised without a real source");
            }
        }
    }
}
