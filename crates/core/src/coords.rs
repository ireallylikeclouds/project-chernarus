//! Reference coordinate conventions.
//!
//! The simulation stores world positions in the same frame that ARMA 2 scripts
//! observe through `getPosASL`, so captured reference traces compare directly
//! with simulation output. Status of this convention: ESTIMATED (see
//! `docs/reference/conventions/coordinates.md`); the capture harness records
//! position and `getDir` together, which is how it will be verified.
//!
//! * `x` — metres east
//! * `y` — metres north
//! * `z` — metres up (above sea level for `getPosASL`)
//! * heading — degrees clockwise from north, `[0, 360)`, as `getDir`

/// Unit vector (east, north) for a heading in degrees clockwise from north.
pub fn heading_forward(heading_deg: f64) -> [f64; 2] {
    let r = heading_deg.to_radians();
    [r.sin(), r.cos()]
}

/// Unit vector (east, north) pointing to the right of a heading.
pub fn heading_right(heading_deg: f64) -> [f64; 2] {
    let r = heading_deg.to_radians();
    [r.cos(), -r.sin()]
}

/// Normalise a heading into `[0, 360)`.
pub fn normalize_heading(heading_deg: f64) -> f64 {
    let h = heading_deg.rem_euclid(360.0);
    // rem_euclid can return 360.0 for tiny negative inputs due to rounding.
    if h >= 360.0 { 0.0 } else { h }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 2], b: [f64; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12
    }

    #[test]
    fn north_is_plus_y_and_east_is_plus_x() {
        assert!(close(heading_forward(0.0), [0.0, 1.0]));
        assert!(close(heading_forward(90.0), [1.0, 0.0]));
        assert!(close(heading_forward(180.0), [0.0, -1.0]));
    }

    #[test]
    fn right_is_clockwise_from_forward() {
        assert!(close(heading_right(0.0), [1.0, 0.0]));
        assert!(close(heading_right(90.0), [0.0, -1.0]));
    }

    #[test]
    fn heading_normalisation() {
        assert_eq!(normalize_heading(-90.0), 270.0);
        assert_eq!(normalize_heading(720.0), 0.0);
        assert!(normalize_heading(-1e-18) < 360.0);
    }
}
