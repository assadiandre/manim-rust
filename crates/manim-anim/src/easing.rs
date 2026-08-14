//! Easing functions. `eval` maps linear progress t∈[0,1] to eased alpha.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Easing {
    Linear,
    /// Manim's default: smoothstep.
    #[default]
    Smooth,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
}

impl Easing {
    pub fn eval(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::Smooth => t * t * (3.0 - 2.0 * t),
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed() {
        for e in [
            Easing::Linear,
            Easing::Smooth,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
        ] {
            assert!((e.eval(0.0)).abs() < 1e-12);
            assert!((e.eval(1.0) - 1.0).abs() < 1e-12);
        }
    }
}
