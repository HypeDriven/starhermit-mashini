//! Mix independently timed Alembic wave samples onto a single bucket surface.
use crate::physics::Impact;
use starhermit_alembic::{Archive, GeometryState, Limits};

pub struct Waves {
    archive: Archive,
    state: GeometryState,
    node: usize,
    samples: Vec<Option<Vec<f32>>>,
    edge: usize,
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u16>,
}

impl Waves {
    pub fn open(bytes: Vec<u8>) -> Result<Self, String> {
        let archive = Archive::parse(bytes.into(), Limits::default()).map_err(|e| e.to_string())?;
        let (node, track) = archive
            .nodes()
            .iter()
            .enumerate()
            .find_map(|(i, n)| (n.name == "Water").then_some((i, n.geometry.as_ref()?)))
            .ok_or("Missing Water mesh in bucket cache")?;
        let count = track.num_samples;
        let mut state = GeometryState::default();
        let base = archive
            .read_geometry(node, 0, &mut state)
            .map_err(|e| e.to_string())?;
        let edge = base.positions.len().isqrt();
        if edge < 2 || edge * edge != base.positions.len() || count < 2 {
            return Err("Invalid bucket grid".into());
        }
        let indices = base
            .indices
            .into_iter()
            .map(u16::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            archive,
            state,
            node,
            samples: vec![None; count],
            edge,
            positions: base.positions,
            indices,
        })
    }
    fn frame(&self, age: f32) -> usize {
        ((age / crate::physics::WAVE_DURATION * (self.samples.len() - 1) as f32).round() as usize)
            .min(self.samples.len() - 1)
    }
    // Inverse of the square-to-disk projection used by the water bake.
    fn uv(x: f32, z: f32) -> [f32; 2] {
        let len = x.hypot(z).max(1.);
        let (x, z) = (x / len, z / len);
        let a = 2. + x * x - z * z;
        let b = 4. - a;
        let k = 2. * std::f32::consts::SQRT_2;
        [
            ((a + k * x).max(0.).sqrt() - (a - k * x).max(0.).sqrt()) * 0.25 + 0.5,
            ((b + k * z).max(0.).sqrt() - (b - k * z).max(0.).sqrt()) * 0.25 + 0.5,
        ]
    }
    pub fn compose(&mut self, impacts: &[Impact], positions: &mut Vec<f32>) -> Result<(), String> {
        positions.clear();
        positions.extend(self.positions.iter().flatten().copied());
        for hit in impacts {
            let f = self.frame(hit.age);
            if self.samples[f].is_none() {
                let g = self
                    .archive
                    .read_geometry(self.node, f, &mut self.state)
                    .map_err(|e| e.to_string())?;
                self.samples[f] = Some(g.positions.iter().map(|p| p[1]).collect());
            }
            let Some(sample) = &self.samples[f] else {
                continue;
            };
            let (s, c) = hit.angle.sin_cos();
            // The baked impulse lies at (0.39, 0). Sample relative to this car's entry point.
            for p in positions.chunks_exact_mut(3) {
                let x = p[0] - hit.x;
                let z = p[2] - hit.z;
                let [u, v] = Self::uv(x * c + z * s + 0.39, -x * s + z * c);
                let extent = (self.edge - 1) as f32;
                let (u, v) = (u.clamp(0., 1.) * extent, v.clamp(0., 1.) * extent);
                let (i, j) = (
                    (u as usize).min(self.edge - 2),
                    (v as usize).min(self.edge - 2),
                );
                let (a, b) = (u - i as f32, v - j as f32);
                let n = j * self.edge + i;
                let lerp = |x: f32, y: f32, t: f32| x + (y - x) * t;
                let height = lerp(
                    lerp(sample[n], sample[n + 1], a),
                    lerp(sample[n + self.edge], sample[n + self.edge + 1], a),
                    b,
                );
                p[1] += height * hit.strength;
            }
        }
        Ok(())
    }
    pub fn stamp(&self, hits: &[Impact]) -> Vec<(u32, usize)> {
        hits.iter().map(|h| (h.id, self.frame(h.age))).collect()
    }
}
