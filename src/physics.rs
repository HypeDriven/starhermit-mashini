use std::f32::consts::FRAC_PI_2;
pub const BOOST_CODES: [u8; 7] = [6, 7, 5, 8, 9, 10, 11];
pub const BOOSTS: [f32; 7] = [8., 2., 4., 12., 18., 28., 40.];
pub const DRUM_CODES: [u8; 8] = [16, 17, 14, 18, 19, 20, 21, 22];
const DRUM_SPEEDS: [f32; 8] = [2., 4., 6.5, 8., 12., 18., 28., 40.];
pub const LOOP: u8 = 12;
pub const LAUNCH: u8 = 13;
pub const FORK_RIGHT: u8 = 27;
pub const FORK_LEFT: u8 = 28;
const HILL_DESCENT: u8 = 29;
const HILL_LIFT: u8 = 30;
const SHORT_DESCENT: u8 = 31;
const SHORT_LIFT: u8 = 32;
const DOWNHILL: u8 = 33;
const DOWN_LEFT: u8 = 34;
const DOWN_RIGHT: u8 = 35;
pub const CREST: u8 = 36;
pub fn switch_code(current: u8, left: bool) -> u8 {
    if matches!(current, FORK_RIGHT | FORK_LEFT) {
        if left { FORK_LEFT } else { FORK_RIGHT }
    } else if left {
        LAUNCH
    } else {
        LOOP
    }
}
pub const START: [u8; 8] = [14, 15, 14, LOOP, 14, 15, 14, 15];
pub const SOAK_TIME: f32 = 3.0;
pub const STARTERS: [(&str, &[u8]); 9] = [
    ("Pocket Oval", &[18, 15, 15, 18, 15, 15]),
    ("Splash Jump", &START),
    (
        "Switchback Run",
        &[
            14, 23, 14, 15, 14, 15, 14, 15, 14, 23, 14, 15, 14, 15, 14, 15,
        ],
    ),
    (
        "Ribbon Leap",
        &[
            14,
            SHORT_DESCENT,
            15,
            14,
            15,
            17,
            SHORT_LIFT,
            14,
            15,
            14,
            14,
            15,
            14,
            24,
            14,
            19,
            25,
        ],
    ),
    (
        "Waterfall Bend",
        &[
            14,
            24,
            14,
            15,
            14,
            15,
            14,
            HILL_LIFT,
            14,
            15,
            14,
            24,
            14,
            15,
            14,
            HILL_DESCENT,
            14,
            14,
            19,
            26,
        ],
    ),
    (
        "Canyon Hop",
        &[
            14,
            14,
            15,
            14,
            15,
            14,
            24,
            14,
            HILL_LIFT,
            14,
            HILL_LIFT,
            14,
            15,
            14,
            HILL_DESCENT,
            14,
            15,
            14,
            HILL_DESCENT,
            14,
            14,
            19,
            25,
        ],
    ),
    (
        "High Dive",
        &[
            14,
            23,
            14,
            23,
            14,
            HILL_DESCENT,
            14,
            HILL_DESCENT,
            14,
            14,
            23,
            14,
            15,
            14,
            15,
            14,
            HILL_LIFT,
            18,
            14,
            15,
            14,
            HILL_LIFT,
            22,
            19,
            20,
            20,
            26,
        ],
    ),
    (
        "Comet Run",
        &[
            14,
            15,
            14,
            HILL_DESCENT,
            17,
            14,
            15,
            14,
            HILL_DESCENT,
            14,
            15,
            14,
            23,
            14,
            LOOP,
            18,
            HILL_LIFT,
            14,
            LOOP,
            14,
            24,
            18,
            HILL_LIFT,
            14,
            24,
            14,
            14,
            19,
            25,
        ],
    ),
    (
        "Cloud Cascade",
        &[
            DOWNHILL, DOWNHILL, DOWN_RIGHT, DOWNHILL, DOWN_RIGHT, DOWNHILL, DOWN_LEFT, DOWNHILL,
            DOWNHILL, DOWN_RIGHT, DOWNHILL, DOWNHILL, DOWN_RIGHT, DOWNHILL, DOWNHILL, DOWNHILL,
            DOWNHILL, DOWNHILL, DOWNHILL, DOWN_RIGHT, DOWNHILL, DOWNHILL, DOWNHILL, DOWNHILL,
            DOWNHILL, DOWNHILL, DOWNHILL, DOWNHILL, CREST, DOWNHILL, DOWNHILL, 26,
        ],
    ),
];
pub fn starter_anchors(index: usize) -> Vec<Option<Pose>> {
    let mut anchors = vec![None; STARTERS[index].1.len()];
    if index >= 3 {
        anchors[0] = Some(Pose {
            x: -6.,
            z: -4.,
            y: if index == 8 {
                15. - build(STARTERS[index].1).iter().map(|s| s.rise).sum::<f32>()
            } else {
                [4., 6., 8., 10., 12.][index - 3]
            },
            a: 0.,
        });
    }
    // Comet Run also descends a terrace. A short gap after a powered
    // straight carries the car onto a lower deck before its final launch.
    let drops: &[usize] = match index {
        7 => &[24],
        _ => &[],
    };
    if !drops.is_empty() {
        let step = 0.55;
        anchors[0].as_mut().unwrap().y += step * drops.len() as f32;
        for &i in drops {
            let p = build_at(STARTERS[index].1, &anchors)[i].p;
            anchors[i] = Some(Pose {
                x: p.x + 0.6 * p.a.cos(),
                z: p.z + 0.6 * p.a.sin(),
                y: p.y - step,
                ..p
            });
        }
    }
    anchors
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub a: f32,
}
#[derive(Clone, Copy)]
pub struct Segment {
    pub p: Pose,
    pub len: f32,
    pub turn: f32,
    pub rise: f32,
    pub kind: u8,
    pub start: f32,
    pub boost: f32,
    pub high_rails: bool,
    pub launch: bool,
    pub owner: usize,
}
impl Segment {
    pub fn profile(&self) -> [(f32, f32); 8] {
        let mut profile = PROFILE;
        if self.high_rails || self.kind == 4 {
            for (side, height) in &mut profile {
                if *height > 0. {
                    *height = 0.95;
                    if side.abs() < 1.0 {
                        *side = side.signum() * 0.904;
                    }
                }
            }
        }
        profile
    }
    /// Clip the two inside lips at their common centreline to form a single open throat.
    pub fn vertex(&self, t: f32, j: usize) -> [f32; 3] {
        let (mut side, mut h) = self.profile()[j];
        let p = self.at(t);
        if matches!(self.kind, 6 | 8) {
            let theta = t * FRAC_PI_2;
            let sign = self.turn.signum();
            let centre = -4. * (1. - theta.cos()) / theta.cos().max(0.001);
            if side * sign < centre && centre > -1.04 {
                side = centre * sign;
                h = h.min(0.);
            }
        }
        [p.x - p.a.sin() * side, p.y + h, p.z + p.a.cos() * side]
    }
    fn local(&self, p: Pose) -> (f32, f32) {
        let dx = p.x - self.p.x;
        let dz = p.z - self.p.z;
        (
            dx * self.p.a.cos() + dz * self.p.a.sin(),
            dz * self.p.a.cos() - dx * self.p.a.sin(),
        )
    }
    pub fn lever(&self, angle: f32, offset: f32) -> Pose {
        let a = self.p.a + angle;
        Pose {
            x: self.p.x + 2.86 * self.p.a.cos() + offset * a.cos(),
            z: self.p.z + 2.86 * self.p.a.sin() + offset * a.sin(),
            y: self.p.y + 0.32,
            a,
        }
    }
    pub fn at(&self, t: f32) -> Pose {
        let p = self.p;
        let a = p.a + self.turn * t;
        let (dx, dz) = if self.turn == 0. {
            (self.len * t * p.a.cos(), self.len * t * p.a.sin())
        } else {
            let r = self.len / self.turn;
            (r * (a.sin() - p.a.sin()), r * (p.a.cos() - a.cos()))
        };
        Pose {
            x: p.x + dx,
            y: p.y + self.rise * t,
            z: p.z + dz,
            a,
        }
    }
}
pub fn build(kinds: &[u8]) -> Vec<Segment> {
    build_at(kinds, &[])
}
pub fn build_at(kinds: &[u8], anchors: &[Option<Pose>]) -> Vec<Segment> {
    let mut p = Pose {
        x: -6.,
        y: 3.6,
        z: -4.,
        a: 0.,
    };
    if kinds.first().is_some_and(|k| DRUM_CODES.contains(k)) {
        p.y = 1.35;
    }
    let mut branches = vec![];
    let mut start = 0.;
    let mut road: Vec<_> = kinds
        .iter()
        .enumerate()
        .map(|(owner, &code)| {
            if let Some(Some(anchor)) = anchors.get(owner) {
                p = *anchor;
            }
            let kind = match code {
                LOOP | LAUNCH | FORK_RIGHT | FORK_LEFT => 6,
                14 | 16..=22 => 5,
                15 => 2,
                23 => 1,
                24 => 0,
                25 | 26 => 7,
                HILL_DESCENT | SHORT_DESCENT => 3,
                HILL_LIFT | SHORT_LIFT => 4,
                DOWNHILL | CREST => 3,
                DOWN_LEFT => 1,
                DOWN_RIGHT => 2,
                _ => code.min(5),
            };
            let boost = if let Some(i) = DRUM_CODES.iter().position(|c| *c == code) {
                DRUM_SPEEDS[i]
            } else {
                BOOSTS[code.saturating_sub(5).min(6) as usize]
            };
            let turn = match kind {
                1 => -FRAC_PI_2,
                2 | 6 => FRAC_PI_2,
                _ => 0.,
            };
            let len = if turn == 0. { 6. } else { 4. * FRAC_PI_2 };
            let rise = match kind {
                _ if matches!(code, DOWN_LEFT | DOWN_RIGHT) => -1.,
                3 if code == DOWNHILL => -0.7,
                3 if code == CREST => 1.,
                3 if code == HILL_DESCENT => -2.4,
                3 if code == SHORT_DESCENT => -1.25,
                4 if code == HILL_LIFT => 2.4,
                4 if code == SHORT_LIFT => 1.25,
                7 if code == 25 => 2.,
                3 => -(p.y - 0.6).min(3.),
                4 => (7. - p.y).min(3.),
                _ => 0.,
            };
            let r = Segment {
                p,
                len,
                turn,
                rise,
                kind,
                boost,
                high_rails: code >= LOOP,
                launch: matches!(code, LAUNCH | FORK_LEFT),
                owner,
                start,
            };
            p = r.at(1.);
            start += len;
            if kind == 6 {
                let diverge = Segment {
                    turn: -FRAC_PI_2,
                    kind: 8,
                    ..r
                };
                if matches!(code, FORK_RIGHT | FORK_LEFT) {
                    branches.push(diverge);
                    return r;
                }
                let booster = Segment {
                    p: diverge.at(1.),
                    kind: 5,
                    len: 3.8,
                    turn: 0.,
                    boost: 8.,
                    high_rails: false,
                    ..r
                };
                let ramp = Segment {
                    p: booster.at(1.),
                    kind: 7,
                    len: 3.2,
                    rise: 1.15,
                    ..booster
                };
                branches.extend([diverge, booster, ramp]);
            }
            r
        })
        .collect();
    road.extend(branches);
    road
}
/// Place the midpoint on the tap, or mate a nearby, aligned connector at its exact height.
pub fn place_track(code: u8, heading: f32, x: f32, z: f32, road: &[Segment]) -> Pose {
    let y = if code == 3 { 3.6 } else { 0.6 };
    let pieces = build_at(
        &[code],
        &[Some(Pose {
            x: 0.,
            z: 0.,
            y,
            a: heading,
        })],
    );
    let mut piece = pieces[0];
    let mid = if pieces.len() == 2 {
        let mut p = piece.at(0.5);
        let other = pieces[1].at(0.5);
        p.x = (p.x + other.x) * 0.5;
        p.z = (p.z + other.z) * 0.5;
        p
    } else {
        piece.at(0.5)
    };
    piece.p.x = x - mid.x;
    piece.p.z = z - mid.z;
    let mut anchor = piece.p;
    let mut best = 1.25;
    for (mut port, end) in
        std::iter::once((pieces[0].p, 0.)).chain(pieces.iter().map(|r| (r.at(1.), 1.)))
    {
        port.x += piece.p.x;
        port.z += piece.p.z;
        for existing in road {
            for other_end in [0., 1.] {
                let other = existing.at(other_end);
                let opposite = if end == other_end { -1. } else { 1. };
                if (port.a - other.a).cos() * opposite < 0.995 {
                    continue;
                }
                let gap = (port.x - other.x).hypot(port.z - other.z);
                let tap = (x - other.x).hypot(z - other.z);
                // Tapping the socket directly is also a way to extend a track.
                let distance = gap.min(tap + 0.35);
                let y = other.y - piece.rise * end;
                if distance < best && y >= 0.59 && y + piece.rise >= 0.59 {
                    best = distance;
                    anchor = Pose {
                        x: piece.p.x + other.x - port.x,
                        z: piece.p.z + other.z - port.z,
                        y,
                        a: heading,
                    };
                }
            }
        }
    }
    anchor
}
use rapier3d::na::{Point3, UnitQuaternion, Vector3};
use rapier3d::prelude::*;
use std::f32::consts::{PI, TAU};
pub const AXLES: [[f32; 3]; 4] = [
    [-0.81, 0.27, 0.51],
    [-0.81, 0.27, -0.51],
    [0.81, 0.27, 0.51],
    [0.81, 0.27, -0.51],
];
pub const BUCKET_RADIUS: f32 = 1.85;
pub const STUNT_POOL_RADIUS: f32 = 8.5;
pub const SPLASH_RADIUS: f32 = 3.85;
const DRY_WHEEL_DAMPING: f32 = 0.065;
const WHEEL_RADIUS: f32 = 0.237 + 0.032;
pub const WATER: f32 = 1.12;
pub const RIM: f32 = 1.45;
pub const CLEATS: usize = 5;
pub const PROFILE: [(f32, f32); 8] = [
    (-1.04, 0.32),
    (-0.94, 0.32),
    (-0.9, 0.),
    (0.9, 0.),
    (0.94, 0.32),
    (1.04, 0.32),
    (1.04, -0.14),
    (-1.04, -0.14),
];
pub const WAVE_DURATION: f32 = 2.4;
#[derive(Clone, Copy, Debug)]
pub struct Impact {
    pub id: u32,
    pub age: f32,
    pub x: f32,
    pub z: f32,
    pub angle: f32,
    pub strength: f32,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BucketDepth {
    #[default]
    Shallow,
    Deep,
    ExtraDeep,
}
impl BucketDepth {
    pub fn from_index(index: u8) -> Self {
        match index {
            1 => Self::Deep,
            2 => Self::ExtraDeep,
            _ => Self::Shallow,
        }
    }
    fn height(self) -> f32 {
        match self {
            Self::Shallow => WATER,
            Self::Deep => 2.65,
            Self::ExtraDeep => 4.15,
        }
    }
}
#[derive(Clone)]
pub struct Bubble {
    pub position: [f32; 3],
    pub radius: f32,
    phase: f32,
}
#[derive(Clone, Default)]
pub struct Bucket {
    pub radius: f32,
    pub x: f32,
    pub z: f32,
    pub depth: BucketDepth,
    pub bubbles: Vec<Bubble>,
    pub impacts: Vec<Impact>,
    pub serial: u32,
}
impl Bucket {
    pub fn surface(&self) -> f32 {
        self.depth.height()
    }
    pub fn rim(&self) -> f32 {
        self.surface() + (RIM - WATER)
    }
    fn bubble(&mut self, position: [f32; 3], pulse: u32) {
        if self.bubbles.len() >= 64 || position[1] > self.surface() - 0.12 {
            return;
        }
        let phase = (pulse as f32 * 2.4 + position[0]) % std::f32::consts::TAU;
        self.bubbles.push(Bubble {
            position,
            phase,
            radius: 0.045 + 0.038 * phase.sin().abs(),
        });
    }
    pub fn strike(&mut self, x: f32, z: f32, speed: f32) {
        let x = (x - self.x) / self.radius;
        let z = (z - self.z) / self.radius;
        self.serial = self.serial.wrapping_add(1);
        self.impacts.push(Impact {
            id: self.serial,
            age: 0.,
            x,
            z,
            angle: z.atan2(x),
            strength: (speed * 0.20).clamp(0.35, 2.),
        });
    }
    pub fn advance(&mut self, dt: f32) {
        let surface = self.surface();
        self.bubbles.retain_mut(|b| {
            b.phase += dt * 3.;
            b.position[0] += dt * 0.10 * b.phase.sin();
            b.position[2] += dt * 0.10 * b.phase.cos();
            b.position[1] += dt * (0.65 + 4. * b.radius);
            b.position[1] + b.radius < surface
        });
        for impact in &mut self.impacts {
            impact.age += dt
        }
        self.impacts.retain(|hit| hit.age < WAVE_DURATION);
    }
}
pub fn starter_bucket(road: &[Segment]) -> Bucket {
    starter_buckets(1, road).next().unwrap()
}
pub fn starter_buckets(course: usize, road: &[Segment]) -> impl Iterator<Item = Bucket> + '_ {
    road.iter()
        .enumerate()
        .filter(|(_, r)| r.kind == 7)
        .map(move |(i, ramp)| {
            let p = ramp.at(1.);
            let elevated = ramp.p.y > 3.;
            let fast = i > 0 && road[i - 1].kind == 5 && road[i - 1].boost >= 18.;
            let depth = if STARTERS[course].0 == "Waterfall Bend" || p.y >= 14. {
                BucketDepth::ExtraDeep
            } else {
                BucketDepth::Shallow
            };
            let reach = if elevated {
                (if fast { 16. } else { 10. }) * (2. * (p.y - depth.height()).max(0.) / 9.81).sqrt()
            } else {
                6.2
            };
            Bucket {
                depth,
                radius: if elevated {
                    STUNT_POOL_RADIUS
                } else {
                    SPLASH_RADIUS
                },
                x: p.x + reach * p.a.cos(),
                z: p.z + reach * p.a.sin(),

                ..Bucket::default()
            }
        })
}
/// Compress finger velocity while keeping direction and small gestures responsive.
pub fn nudge(delta: [f32; 3]) -> [f32; 3] {
    let v = Vector3::from(delta);
    let speed = v.norm();
    if speed < 0.0001 {
        return [0.; 3];
    }
    (v * ((0.95 * (speed * 0.55).ln_1p()).min(3.2) / speed)).into()
}
pub const LIFT_SPEED: f32 = 2.55;
pub const LIFT_PULLEY_RADIUS: f32 = 0.38;
pub const LIFT_INSET: f32 = 0.20;
pub const LIFT_FLIGHT_HEIGHT: f32 = 0.18;
/// Stadium-shaped belt in the inclined plane, including both sprocket arcs.
pub fn cleat(r: &Segment, index: usize, time: f32) -> Pose {
    let pitch = r.rise.atan2(r.len);
    let length = r.len.hypot(r.rise) - 2. * LIFT_INSET;
    let radius = LIFT_PULLEY_RADIUS;
    let loop_len = 2. * length + TAU * radius;
    let d = (index as f32 * loop_len / CLEATS as f32 + time * LIFT_SPEED).rem_euclid(loop_len);
    let (s, n) = if d < length {
        (d, radius)
    } else if d < length + PI * radius {
        let a = (d - length) / radius;
        (length + radius * a.sin(), radius * a.cos())
    } else if d < 2. * length + PI * radius {
        (2. * length + PI * radius - d, -radius)
    } else {
        let a = (d - 2. * length - PI * radius) / radius;
        (-radius * a.sin(), -radius * a.cos())
    };
    let u = (s + LIFT_INSET) * pitch.cos() - (n - radius) * pitch.sin();
    let mut p = r.at(u / r.len);
    p.y = r.p.y + (s + LIFT_INSET) * pitch.sin() + (n - radius) * pitch.cos() + LIFT_FLIGHT_HEIGHT;
    p
}
pub struct Car {
    pub model: u8,
    pub paint: u8,
    pub p: Pose,
    pub rotation: [f32; 4],
    pub spin: [f32; 4],
    pub rest: f32,
    stationary_seconds: f64,
    stationary_pose: Isometry<f32>,
    pub wet: bool,
    dry_time: f32,
    bump_wait: f32,
    recent_speed: f32,
    revving: bool,
    pub soaked: f32,
    pub portal: f32,
    pub departure: Pose,
    pub arrival: Pose,
    body: RigidBodyHandle,
    wheels: [RigidBodyHandle; 4],
}
#[derive(Default, Copy, Clone)]
pub struct Sounds {
    pub splash: f32,
    pub bump: f32,
    pub rev: f32,
}
pub struct World {
    pub sounds: Sounds,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad: BroadPhaseMultiSap,
    narrow: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    joints: ImpulseJointSet,
    multi: MultibodyJointSet,
    ccd: CCDSolver,
    lifts: Vec<(RigidBodyHandle, Segment, usize)>,
    boosters: Vec<(Segment, [RigidBodyHandle; 2], f32)>,
    gates: Vec<Gate>,
    pub time: f32,
}
fn quat(q: &UnitQuaternion<f32>) -> [f32; 4] {
    [q.i, q.j, q.k, q.w]
}
fn deck_shape(r: &Segment) -> SharedShape {
    let n = if r.turn == 0. { 1 } else { 24 };
    let mut vertices = vec![];
    let mut triangles = vec![];
    for i in 0..=n {
        for j in 0..8 {
            vertices.push(Point3::from(r.vertex(i as f32 / n as f32, j)));
            if i < n {
                let a = (i * 8 + j) as u32;
                let b = (i * 8 + (j + 1) % 8) as u32;
                triangles.extend([[a, b, a + 8], [b, b + 8, a + 8]])
            }
        }
    }
    SharedShape::trimesh_with_flags(vertices, triangles, TriMeshFlags::FIX_INTERNAL_EDGES)
}
pub fn switch_clear(r: &Segment, cars: &[Car]) -> bool {
    cars.iter().all(|c| {
        let (x, z) = r.local(c.p);
        (c.p.y - r.p.y).abs() > 1.5 || !(-1.55..4.4).contains(&x) || z.abs() > 2.25
    })
}
struct Gate {
    piece: Segment,
    blade: RigidBodyHandle,
    angle: f32,
    waiting: bool,
}
const THROW: f32 = 0.41;
impl World {
    pub fn new(road: &[Segment], buckets: &[Bucket]) -> Self {
        let mut w = Self {
            sounds: Sounds::default(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad: BroadPhaseMultiSap::new(),
            narrow: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            joints: ImpulseJointSet::new(),
            multi: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            gates: vec![],
            lifts: vec![],
            boosters: vec![],
            time: 0.,
        };
        w.colliders.insert(
            ColliderBuilder::halfspace(Vector3::y_axis())
                .friction(0.88)
                .restitution(0.15)
                .user_data(1)
                .build(),
        );
        for (index, r) in road.iter().enumerate() {
            w.colliders.insert(
                ColliderBuilder::new(deck_shape(r))
                    .user_data(100 + index as u128)
                    .friction(0.10)
                    .friction_combine_rule(CoefficientCombineRule::Min)
                    .build(),
            );
            let p = r.at(0.05);
            for side in [-0.75, 0.75] {
                w.colliders.insert(
                    ColliderBuilder::cylinder((p.y - 0.1) / 2., 0.23)
                        .translation(vector![
                            p.x - side * p.a.sin(),
                            p.y / 2. - 0.06,
                            p.z + side * p.a.cos()
                        ])
                        .build(),
                );
            }
            if r.kind == 6 {
                let angle = if r.launch { -THROW } else { THROW };
                let p = r.lever(angle, -1.38);
                let blade = w.bodies.insert(
                    RigidBodyBuilder::kinematic_position_based()
                        .translation(vector![p.x, p.y, p.z])
                        .rotation(vector![0., -p.a, 0.]),
                );
                w.attach(
                    blade,
                    ColliderBuilder::round_cuboid(1.34, 0.26, 0.032, 0.035).friction(0.04),
                );
                w.gates.push(Gate {
                    piece: *r,
                    blade,
                    angle,
                    waiting: false,
                });
            }
            if r.kind == 4 {
                for i in 0..CLEATS {
                    let p = cleat(r, i, 0.);
                    let h = w.bodies.insert(
                        RigidBodyBuilder::kinematic_position_based()
                            .translation(vector![p.x, p.y, p.z])
                            .rotation(vector![0., -p.a, 0.]),
                    );
                    w.attach(
                        h,
                        ColliderBuilder::round_cuboid(0.075, 0.185, 0.78, 0.035).friction(1.),
                    );
                    w.lifts.push((h, *r, i));
                }
            }
            if r.kind == 5 {
                let p = r.at(0.65);
                let pair = [-1., 1.].map(|side| {
                    let (x, y, z) = if r.high_rails {
                        (
                            p.x + side * p.a.cos() * 0.48,
                            p.y - 0.245,
                            p.z + side * p.a.sin() * 0.48,
                        )
                    } else {
                        (
                            p.x - side * p.a.sin() * 1.03,
                            p.y + 0.34,
                            p.z + side * p.a.cos() * 1.03,
                        )
                    };
                    let h = w.bodies.insert(
                        RigidBodyBuilder::kinematic_velocity_based().translation(vector![x, y, z]),
                    );
                    let shape = if r.high_rails {
                        ColliderBuilder::cylinder(0.86, 0.28).rotation(
                            UnitQuaternion::from_euler_angles(FRAC_PI_2, -p.a, 0.).scaled_axis(),
                        )
                    } else {
                        ColliderBuilder::cylinder(0.18, 0.46)
                    };
                    w.attach(h, shape.friction(1.4));
                    h
                });
                w.boosters.push((*r, pair, r.boost));
            }
        }
        for b in buckets {
            let n = 32;
            for i in 0..n {
                let a = TAU * i as f32 / n as f32;
                w.colliders.insert(
                    ColliderBuilder::cuboid(
                        b.radius * (PI / n as f32).tan() + 0.015,
                        b.rim() / 2.,
                        0.055,
                    )
                    .translation(vector![
                        b.x + b.radius * a.sin(),
                        b.rim() / 2.,
                        b.z + b.radius * a.cos()
                    ])
                    .rotation(vector![0., a, 0.])
                    .user_data(1)
                    .build(),
                );
            }
        }
        w
    }
    pub fn align_switch(&mut self, r: &Segment) {
        if let Some(g) = self.gates.iter_mut().find(|g| g.piece.owner == r.owner) {
            g.piece.launch = r.launch;
            g.waiting = false;
        }
    }
    pub fn gate_state(&self, owner: usize) -> Option<(bool, f32)> {
        self.gates
            .iter()
            .find(|g| g.piece.owner == owner)
            .map(|g| (g.piece.launch, g.angle))
    }
    fn attach(&mut self, h: RigidBodyHandle, c: ColliderBuilder) {
        self.colliders
            .insert_with_parent(c.build(), h, &mut self.bodies);
    }
    pub fn spawn(&mut self, model: u8, paint: u8, road: &[Segment]) -> Car {
        // When a fleet loads together, give each car a clear spot near the start.
        let clear = |p: Pose| {
            self.bodies.iter().all(|(_, b)| {
                let v = b.translation();
                b.user_data != 2 || (v.y - p.y).abs() > 1.3 || (v.x - p.x).hypot(v.z - p.z) > 2.7
            })
        };
        let (segment, mut p) = road
            .iter()
            .flat_map(|r| {
                let slots = if r.kind == 5 && !r.high_rails {
                    [0.52, 0.98]
                } else if r.kind == 5 {
                    [0.435, 0.95]
                } else {
                    [0.19, 0.68]
                };
                slots.map(|t| (r, r.at(t)))
            })
            .find(|(_, p)| clear(*p))
            .unwrap_or((&road[0], road[0].at(0.19)));
        while !clear(p) {
            p.y += 1.4;
        }
        let pitch = segment.rise.atan2(segment.len);
        let q = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), -p.a)
            * UnitQuaternion::from_axis_angle(&Vector3::z_axis(), pitch);
        let origin = vector![p.x, p.y + 0.075, p.z];

        let base = RigidBodyBuilder::dynamic()
            .angular_damping(0.07)
            .linear_damping(0.018)
            .ccd_enabled(true);
        let h = self.bodies.insert(
            base.clone()
                .user_data(2)
                .position(Isometry::from_parts(origin.into(), q)),
        );
        self.attach(
            h,
            ColliderBuilder::round_cuboid(1.19, 0.13, 0.455, 0.055)
                .translation(vector![0., 0.43, 0.])
                .mass([1.18, 1.49, 1.34][model as usize])
                .friction(0.65)
                .restitution(0.18),
        );
        self.attach(
            h,
            ColliderBuilder::round_cuboid(0.58, 0.16, 0.35, 0.055)
                .translation(vector![-0.12, 0.76, 0.])
                .mass(0.16),
        );
        // The tuned cabin block stops short of the modelled roof; this massless cap adds the
        // rest so a car landing on its roof or tumbling meets the ground with its roof.
        self.attach(h, roof_collider(model).density(0.));
        let wheels = AXLES.map(|a| {
            let offset = Vector3::from(a);
            let wh = self.bodies.insert(
                base.clone()
                    .position(Isometry::from_parts((origin + q * offset).into(), q))
                    .angular_damping(DRY_WHEEL_DAMPING),
            );
            self.attach(
                wh,
                ColliderBuilder::round_cylinder(0.067, 0.237, 0.032)
                    .rotation(vector![FRAC_PI_2, 0., 0.])
                    .mass(0.07)
                    .friction(0.8)
                    .restitution(0.16),
            );
            self.joints.insert(
                h,
                wh,
                RevoluteJointBuilder::new(Vector3::z_axis())
                    .local_anchor1(Point3::from(a))
                    .contacts_enabled(false),
                true,
            );
            wh
        });
        Car {
            model,
            paint,
            p,
            rotation: quat(&q),
            spin: [0.; 4],
            rest: 0.,
            stationary_seconds: 0.,
            stationary_pose: *self.bodies[h].position(),
            wet: false,
            dry_time: 1.,
            bump_wait: 0.,
            recent_speed: 0.,
            revving: false,
            soaked: 0.,
            portal: 0.,
            departure: p,
            arrival: p,
            body: h,
            wheels,
        }
    }
    pub fn reconfigure(&mut self, road: &[Segment], buckets: &[Bucket], cars: &mut [Car]) {
        let mut next = Self::new(road, buckets);
        for c in cars {
            let mut fresh = next.spawn(c.model, c.paint, road);
            for (old, new) in c
                .wheels
                .into_iter()
                .chain([c.body])
                .zip(fresh.wheels.into_iter().chain([fresh.body]))
            {
                let from = &self.bodies[old];
                let to = &mut next.bodies[new];
                to.set_position(*from.position(), true);
                to.set_linvel(*from.linvel(), true);
                to.set_angvel(*from.angvel(), true);
            }
            fresh.p = c.p;
            fresh.rotation = c.rotation;
            fresh.spin = c.spin;
            fresh.soaked = c.soaked;
            fresh.portal = c.portal;
            fresh.departure = c.departure;
            fresh.arrival = c.arrival;
            fresh.wet = c.wet;
            fresh.dry_time = c.dry_time;
            fresh.bump_wait = c.bump_wait;
            fresh.recent_speed = c.recent_speed;
            fresh.revving = c.revving;
            fresh.rest = c.rest;
            fresh.stationary_seconds = c.stationary_seconds;
            fresh.stationary_pose = c.stationary_pose;
            *c = fresh;
        }
        *self = next;
    }
    pub fn remove(&mut self, c: &Car) {
        for h in c.wheels.into_iter().chain([c.body]) {
            self.bodies.remove(
                h,
                &mut self.islands,
                &mut self.colliders,
                &mut self.joints,
                &mut self.multi,
                true,
            );
        }
    }
    pub fn flick(&mut self, c: &Car, delta: [f32; 3]) {
        let dv = Vector3::from(delta).cap_magnitude(40.);
        for h in c.wheels.into_iter().chain([c.body]) {
            let b = &mut self.bodies[h];
            b.apply_impulse(dv * b.mass(), true)
        }
    }
    pub fn step(&mut self, cars: &mut [Car], road: &[Segment], buckets: &mut [Bucket], dt: f32) {
        self.time += dt;
        for g in &mut self.gates {
            if g.waiting && switch_clear(&g.piece, cars) {
                g.piece.launch = !g.piece.launch;
                g.waiting = false;
            }
            let target = if g.piece.launch { -THROW } else { THROW };
            g.angle += (target - g.angle).clamp(-dt * 3., dt * 3.);
            let p = g.piece.lever(g.angle, -1.38);
            self.bodies[g.blade].set_next_kinematic_position(Isometry::new(
                vector![p.x, p.y, p.z],
                vector![0., -p.a, 0.],
            ));
        }
        for (h, r, i) in &self.lifts {
            let p = cleat(r, *i, self.time);
            self.bodies[*h].set_next_kinematic_translation(vector![p.x, p.y, p.z]);
        }
        for (r, hs, speed) in &mut self.boosters {
            let p = r.at(0.65);
            let forward = vector![p.a.cos(), 0., p.a.sin()];
            let mut engaged = false;
            for c in cars.iter() {
                let b = &self.bodies[c.body];
                let rel = b.translation() - vector![p.x, p.y, p.z];
                let along = rel.dot(&forward);
                if rel.y.abs() < 1.
                    && (-3.5..-1.4).contains(&along)
                    && (rel.x * forward.z - rel.z * forward.x).abs() < 0.8
                {
                    *speed = b.linvel().dot(&forward).max(0.) + r.boost;
                    engaged = true;
                    break;
                }
                if rel.norm() < 2.5 {
                    engaged = true
                }
            }
            if !engaged {
                *speed = r.boost
            }
            if r.high_rails {
                *speed = r.boost;
            }
            for (h, side) in hs.iter().zip([-1., 1.]) {
                let omega = if r.high_rails {
                    vector![p.a.sin(), 0., -p.a.cos()] * (*speed / 0.28)
                } else {
                    vector![0., -side * *speed / 0.46, 0.]
                };
                self.bodies[*h].set_angvel(omega, true)
            }
            // The rubber drive tyres grip only during actual solver contact.
            // The motor has a finite traction limit, with slip approaching zero at belt speed.
            for h in hs.iter() {
                let disk = self.bodies[*h].colliders()[0];
                for pair in self.narrow.contact_pairs_with(disk) {
                    if !pair.has_any_active_contact {
                        continue;
                    }
                    let other = if pair.collider1 == disk {
                        pair.collider2
                    } else {
                        pair.collider1
                    };
                    let Some(hit) = self.colliders.get(other).and_then(|c| c.parent()) else {
                        continue;
                    };
                    let Some(car) = cars
                        .iter()
                        .find(|c| c.body == hit || c.wheels.contains(&hit))
                    else {
                        continue;
                    };
                    let point = *self.bodies[hit].center_of_mass();
                    let rb = &mut self.bodies[car.body];
                    let slip = *speed - rb.velocity_at_point(&point).dot(&forward);
                    let impulse = slip.clamp(0., 7.) * rb.mass() * (dt * 28.).min(1.);
                    rb.apply_impulse_at_point(forward * impulse, point, true);
                }
            }
        }
        for b in buckets.iter_mut() {
            b.advance(dt);
        }
        for c in cars.iter_mut() {
            let rb = &mut self.bodies[c.body];
            let mut wet = false;
            for offset in [
                point![0.85, 0.30, 0.],
                point![-0.85, 0.30, 0.],
                point![0., 0.76, 0.],
            ] {
                let point = rb.position() * offset;
                for b in buckets.iter_mut() {
                    if (point.x - b.x).hypot(point.z - b.z) < b.radius - 0.06
                        && point.y < b.surface()
                    {
                        if !wet && c.dry_time >= 0.15 {
                            let speed = rb.velocity_at_point(&point).norm();
                            b.strike(point.x, point.z, speed);
                            self.sounds.splash =
                                self.sounds.splash.max((speed / 10.).clamp(0.2, 1.));
                        }
                        let pulse = ((c.soaked + dt) * 7.) as u32;
                        if !wet && pulse != (c.soaked * 7.) as u32 && rb.linvel().y < -0.06 {
                            b.bubble([point.x, point.y, point.z], pulse);
                        }
                        wet = true;
                        let depth = ((b.surface() - point.y) / 0.42).clamp(0., 1.);
                        let (buoyancy, drag) = if b.depth == BucketDepth::Shallow {
                            (8.3, 11.)
                        } else {
                            (5.2, 4.8)
                        };
                        let impulse = (vector![0., buoyancy, 0.]
                            - rb.velocity_at_point(&point) * drag)
                            * rb.mass()
                            * depth
                            * dt
                            / 3.;
                        rb.apply_impulse_at_point(impulse, point, true);
                        break;
                    }
                }
            }
            for handle in c.wheels {
                let wheel = &mut self.bodies[handle];
                let p = wheel.translation();
                let immersed = buckets
                    .iter()
                    .filter(|b| (p.x - b.x).hypot(p.z - b.z) < b.radius - 0.055)
                    .map(|b| {
                        ((b.surface() - p.y + WHEEL_RADIUS) / (2. * WHEEL_RADIUS)).clamp(0., 1.)
                    })
                    .fold(0., f32::max);
                wheel.set_angular_damping(DRY_WHEEL_DAMPING + 8. * immersed);
            }
            c.dry_time = if wet { 0. } else { c.dry_time + dt };
            c.wet = wet;
            c.soaked = if wet { c.soaked + dt } else { 0. };
            c.portal = (c.portal - dt).max(0.);
            // Rolling resistance opposes horizontal travel only while wheels touch the deck.
            let grounded = c.wheels.iter().any(|h| {
                let wheel = self.bodies[*h].colliders()[0];
                self.narrow.contact_pairs_with(wheel).any(|p| {
                    p.has_any_active_contact
                        && self
                            .colliders
                            .get(if p.collider1 == wheel {
                                p.collider2
                            } else {
                                p.collider1
                            })
                            .is_some_and(|c| c.user_data >= 100)
                })
            });
            if grounded {
                let rb = &mut self.bodies[c.body];
                let horizontal = vector![rb.linvel().x, 0., rb.linvel().z];
                rb.apply_impulse(-horizontal.cap_magnitude(dt * 0.43) * rb.mass(), false);
            }
        }
        let settings = IntegrationParameters {
            dt,
            max_ccd_substeps: 3,
            ..Default::default()
        };
        self.pipeline.step(
            &vector![0., -9.81, 0.],
            &settings,
            &mut self.islands,
            &mut self.broad,
            &mut self.narrow,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multi,
            &mut self.ccd,
            None,
            &(),
            &(),
        );
        for c in cars.iter_mut() {
            let rb = &self.bodies[c.body];
            let q = rb.rotation();
            let p = rb.translation();
            let prev = c.p;
            c.p.x = p.x;
            c.p.y = p.y;
            c.p.z = p.z;
            for g in &mut self.gates {
                let (x, z) = g.piece.local(c.p);
                if g.piece.local(prev).0 < 2.4
                    && x >= 2.4
                    && z.abs() < 2.2
                    && (c.p.y - g.piece.p.y).abs() < 1.0
                {
                    g.waiting = true;
                }
            }
            c.rotation = quat(q);
            for (i, h) in c.wheels.iter().enumerate() {
                let rot = q.inverse() * self.bodies[*h].rotation();
                c.spin[i] = 2. * rot.k.atan2(rot.w);
            }
            let offtrack = (p.y < RIM + 0.5)
                && c.wheels.iter().chain([&c.body]).any(|h| {
                    self.bodies[*h].colliders().iter().any(|ch| {
                        self.narrow.contact_pairs_with(*ch).any(|pair| {
                            pair.has_any_active_contact
                                && (self.colliders[pair.collider1].user_data == 1
                                    || self.colliders[pair.collider2].user_data == 1)
                        })
                    })
                });
            // Compare travel speed to its recent average so turning or falling straight down
            // does not rev the engine. Hysteresis keeps one continuous boost to one sound.
            let speed = rb.linvel().x.hypot(rb.linvel().z);
            let gain = speed - c.recent_speed;
            c.recent_speed += gain * (1. - (-dt / 0.20).exp());
            if c.wet || gain < 0.07 {
                c.revving = false;
            } else if !c.revving && gain > 0.24 {
                self.sounds.rev = self.sounds.rev.max((0.25 + gain * 0.38).min(1.));
                c.revving = true;
            }
            c.bump_wait = (c.bump_wait - dt).max(0.);
            if c.bump_wait == 0. {
                let mut impact = 0.0_f32;
                for h in c.wheels.iter().chain([&c.body]) {
                    for ch in self.bodies[*h].colliders() {
                        for pair in self
                            .narrow
                            .contact_pairs_with(*ch)
                            .filter(|p| p.has_any_active_contact)
                        {
                            impact = impact.max(pair.total_impulse_magnitude() / rb.mass());
                        }
                    }
                }
                if impact > 0.65 {
                    c.bump_wait = 0.14;
                    self.sounds.bump = self.sounds.bump.max((impact / 4.).clamp(0.18, 1.));
                }
            }
            let settled = rb.linvel().norm() < 0.12 && rb.angvel().norm() < 0.25;
            // Compare to a fixed point so slow rolling counts as movement, even if
            // each individual step is smaller than the contact jitter tolerance.
            if settled
                && (*p - c.stationary_pose.translation.vector).norm_squared() <= 0.01_f32.powi(2)
                && c.stationary_pose.rotation.angle_to(q) <= 0.02
            {
                c.stationary_seconds += f64::from(dt);
            } else {
                c.stationary_seconds = 0.;
                c.stationary_pose = *rb.position();
            }
            if offtrack {
                let horizontal = vector![rb.linvel().x, 0., rb.linvel().z];
                let drag = -horizontal.cap_magnitude(1.) * rb.mass() * dt * 1.65;
                self.bodies[c.body].apply_impulse(drag, false);
            }
            if !c.wet && offtrack && settled {
                c.rest += dt
            } else {
                c.rest = 0.
            }
        }
        // Finish reading this solver step's contact graph before retiring any bodies.
        for c in cars.iter_mut() {
            if c.rest > 1.2 || c.soaked >= SOAK_TIME || c.stationary_seconds >= 60. {
                let departure = c.p;
                self.remove(c);
                *c = self.spawn(c.model, c.paint, road);
                c.portal = 0.85;
                c.departure = departure;
                c.arrival = c.p;
            }
        }
    }
    pub fn velocity(&self, car: &Car) -> [f32; 3] {
        (*self.bodies[car.body].linvel()).into()
    }
    pub fn roller_speed(&self, index: usize) -> f32 {
        self.boosters[index].2
    }
}

#[cfg(test)]
mod immersion_tests {
    use super::*;

    #[test]
    fn spinning_wheels_slow_in_water_while_the_car_sinks_and_releases_air() {
        let road = build(&[0]);
        let mut tanks = [Bucket {
            depth: BucketDepth::ExtraDeep,
            radius: 3.,
            x: 15.,
            z: -4.,
            ..Default::default()
        }];
        let mut world = World::new(&road, &tanks);
        let mut cars = [world.spawn(0, 0, &road)];
        let offset = vector![15. - cars[0].p.x, 1.6, 0.];
        for h in cars[0].wheels.into_iter().chain([cars[0].body]) {
            let rb = &mut world.bodies[h];
            rb.set_translation(*rb.translation() + offset, true);
        }
        for h in cars[0].wheels {
            world.bodies[h].set_angvel(vector![0., 0., 30.], true);
        }
        let mut saw_air = false;
        let mut saw_bubbles = false;
        for _ in 0..240 {
            world.step(&mut cars, &road, &mut tanks, 1. / 120.);
            let wheel = &world.bodies[cars[0].wheels[0]];
            if wheel.translation().y > tanks[0].surface() + WHEEL_RADIUS {
                saw_air |= wheel.angvel().z > 28.;
            }
            saw_bubbles |= !tanks[0].bubbles.is_empty();
        }
        assert!(saw_air && saw_bubbles && cars[0].wet);
        assert!(cars[0].p.y < tanks[0].surface() - 1.5);
        for h in cars[0].wheels {
            assert!(world.bodies[h].angvel().norm() < 0.5);
        }
    }
}

/// The cabin's upper part as modelled in tools/cars.py: from the tuned cabin block's top up
/// to the roof (with its trim and rails), its glass sloping in at both ends.
fn roof_collider(model: u8) -> ColliderBuilder {
    let k = model.min(2) as usize;
    let top = [0.59f32, 0.68, 0.63][k];
    let (back, front) = [(-0.84f32, 0.51), (-1.16, 0.88), (-0.25, 0.80)][k];
    let roof = top + [0.42f32, 0.57, 0.51][k] + [0.09f32, 0.16, 0.0][k];
    let base = 0.93f32;
    // Where the sloping glass crosses the base height.
    let along = (base - top) / (roof - top);
    let (b0, f0) = (back + 0.2 * along, front - 0.29 * along);
    let w0 = 0.447 - (0.447 - 0.36) * along;
    let mut points = vec![];
    for z in [-w0, w0] {
        points.push(Point3::new(b0, base, z));
        points.push(Point3::new(f0, base, z));
    }
    for z in [-0.36f32, 0.36] {
        points.push(Point3::new(back + 0.2, roof, z));
        points.push(Point3::new(front - 0.29, roof, z));
    }
    ColliderBuilder::convex_hull(&points).expect("roof points span a volume")
}
