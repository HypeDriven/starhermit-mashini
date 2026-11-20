//! The tabletop starts a new shot every fifteen seconds of uninterrupted idle time.
pub const SHOT_SECONDS: f64 = 15.;
#[derive(Default)]
pub struct IdleCamera {
    idle: f64,
}
impl IdleCamera {
    pub fn reset(&mut self) {
        self.idle = 0.;
    }
    pub fn advance(&mut self, dt: f64, eligible: bool) {
        if eligible {
            self.idle += dt.max(0.);
        } else {
            self.reset();
        }
    }
    pub fn subject(&self, first: usize, cars: usize) -> Option<(usize, f32)> {
        let elapsed = self.idle - SHOT_SECONDS;
        if elapsed < 0. || cars == 0 {
            return None;
        }
        Some((
            (first + (elapsed / SHOT_SECONDS) as usize) % cars,
            elapsed as f32,
        ))
    }
}

/// A separate spring for the lens position and focus lets the lens drift through a
/// corner while the subject stays in frame. All timing is in simulation seconds.
#[derive(Default)]
pub struct FollowCamera {
    motion: [f32; 3],
    heading: f32,
    shot_number: Option<u64>,
    shot: Shot,
    previous: Option<Shot>,
    eye_velocity: [f32; 3],
    focus_velocity: [f32; 3],
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Shot {
    #[default]
    Orbit,
    Alongside,
    Chase,
    Crane,
    Lead,
}

impl FollowCamera {
    fn start_shot(&mut self, number: u64, velocity: [f32; 3], wet: bool) {
        use Shot::*;
        let cycle = [Orbit, Alongside, Chase, Crane, Lead];
        let speed = velocity[0].hypot(velocity[2]);
        let preferred = if wet {
            Some(Alongside)
        } else if velocity[1] < -3. {
            Some(Crane)
        } else if speed > 10. {
            Some(Lead)
        } else {
            None
        };
        let rotated = (0..cycle.len()).map(|i| cycle[(number as usize + i) % cycle.len()]);
        let next = preferred
            .into_iter()
            .chain(rotated)
            .find(|shot| {
                self.shot_number.is_none() || (*shot != self.shot && Some(*shot) != self.previous)
            })
            .unwrap_or(Orbit);
        self.previous = self.shot_number.map(|_| self.shot);
        self.shot = next;
        self.shot_number = Some(number);
        self.motion = velocity;
        if speed > 0.65 {
            self.heading = velocity[0].atan2(velocity[2]);
        }
    }

    pub fn update(
        &mut self,
        eye: &mut [f32; 3],
        focus: &mut [f32; 3],
        pos: [f32; 3],
        velocity: [f32; 3],
        t: f32,
        wet: bool,
        aspect: f32,
        dt: f32,
    ) {
        let dt = dt.clamp(0., 0.1);
        let number = (t / SHOT_SECONDS as f32) as u64;
        if self.shot_number != Some(number) {
            self.start_shot(number, velocity, wet);
        }
        let blend = 1. - (-dt * 3.).exp();
        for (smoothed, v) in self.motion.iter_mut().zip(velocity) {
            *smoothed += (v.clamp(-32., 32.) - *smoothed) * blend;
        }
        let [vx, vy, vz] = self.motion;
        let speed = vx.hypot(vz);
        let flight = vy.abs().min(14.);
        let radius = (5.2 + 0.48 * speed.min(20.) + 0.44 * flight + 1.1 * (t * 0.31).sin())
            / aspect.clamp(0.25, 1.).sqrt();
        let orbit = 0.7 + t * 0.145 + 0.38 * (t * 0.27).sin();
        // Hold the last travel heading at rest and turn by the shortest arc through corners.
        if speed > 0.65 {
            let turn = vx.atan2(vz) - self.heading;
            self.heading += turn.sin().atan2(turn.cos()) * (1. - (-dt * 2.2).exp());
        }
        let phase = t % SHOT_SECONDS as f32;
        let side = if number % 2 == 0 { 1. } else { -1. };
        let height = 2.7 + speed.min(20.) * 0.12 + flight * 0.43;
        // Azimuth, horizontal distance, height, look-ahead time and trailing offset.
        let (angle, range, up, ahead, trail) = match self.shot {
            Shot::Orbit => (orbit, radius, height + 0.7 * (t * 0.34).cos(), 0.23, 0.24),
            Shot::Alongside => (
                self.heading + side * (1.35 + 0.2 * (phase * 0.3).sin()),
                radius * 1.10,
                2.0 + flight * 0.22,
                0.16,
                0.,
            ),
            Shot::Chase => (
                self.heading + std::f32::consts::PI + side * 0.35,
                radius * 0.92,
                height,
                0.36,
                0.08,
            ),
            Shot::Crane => (
                orbit * 0.65,
                radius * 0.85,
                height + radius * 0.94,
                0.18,
                0.,
            ),
            Shot::Lead => (
                self.heading + side * (0.50 + 0.18 * (phase * 0.25).sin()),
                radius * 1.15,
                height * 0.80,
                0.12,
                0.,
            ),
        };
        // A low tracking shot keeps the roof below the horizon, with room for the sky.
        let focus_height = if self.shot == Shot::Alongside && !wet {
            1.45
        } else {
            0.55
        };
        let gaze = [
            pos[0] + vx * ahead,
            (pos[1] + focus_height + vy * 0.15).max(0.35),
            pos[2] + vz * ahead,
        ];
        let lens = [
            pos[0] + range * angle.sin() - vx * trail,
            (pos[1] + up).max(1.),
            pos[2] + range * angle.cos() - vz * trail,
        ];
        spring(eye, &mut self.eye_velocity, lens, 3.8, dt);
        spring(focus, &mut self.focus_velocity, gaze, 8., dt);
    }
}

fn spring(value: &mut [f32; 3], velocity: &mut [f32; 3], goal: [f32; 3], frequency: f32, dt: f32) {
    let decay = (-frequency * dt).exp();
    for i in 0..3 {
        let offset = value[i] - goal[i];
        let slope = velocity[i] + frequency * offset;
        value[i] = goal[i] + (offset + slope * dt) * decay;
        velocity[i] = (velocity[i] - frequency * slope * dt) * decay;
    }
}

#[cfg(test)]
mod shots {
    use super::*;
    #[test]
    fn handoffs_favor_the_action_and_avoid_the_last_two_angles() {
        let mut rig = FollowCamera::default();
        rig.start_shot(0, [12., 0., 0.], false);
        assert!(rig.shot == Shot::Lead);
        rig.start_shot(1, [4., -7., 0.], false);
        assert!(rig.shot == Shot::Crane);
        rig.start_shot(2, [0., -1., 0.], true);
        assert!(rig.shot == Shot::Alongside);
        for n in 3..15 {
            let recent = [Some(rig.shot), rig.previous];
            rig.start_shot(n, [0.; 3], true);
            assert!(!recent.contains(&Some(rig.shot)));
        }
    }
}
