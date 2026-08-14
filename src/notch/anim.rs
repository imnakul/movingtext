//! Motion primitives for the notch.
//!
//! Everything visible moves through a spring rather than a fixed-duration
//! tween: springs stay continuous when the target flips mid-flight, which is
//! exactly what happens when the cursor darts in and out of the notch.

/// A critically-ish damped spring integrated semi-implicitly.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    pub fn new(value: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            stiffness,
            damping,
        }
    }

    /// Advance towards `target`. `dt` is clamped and sub-stepped so a stalled
    /// frame (debugger, heavy app launch) can never explode the integrator.
    pub fn step(&mut self, target: f32, dt: f32) -> f32 {
        let dt = dt.clamp(0.0, 0.1);
        let steps = ((dt / 0.008).ceil() as u32).clamp(1, 16);
        let h = dt / steps as f32;

        for _ in 0..steps {
            let accel = self.stiffness * (target - self.value) - self.damping * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
        }

        // Snap once the motion is imperceptible, so we stop repainting.
        if (target - self.value).abs() < 0.0005 && self.velocity.abs() < 0.005 {
            self.value = target;
            self.velocity = 0.0;
        }

        self.value
    }

    pub fn set(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Hermite smoothstep, used to re-time one spring's output for a second
/// property so content never arrives before the shape has room for it.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
