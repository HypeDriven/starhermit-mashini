use crate::physics::{
    Bucket, CLEATS, Car, LIFT_FLIGHT_HEIGHT, LIFT_INSET, LIFT_PULLEY_RADIUS, LIFT_SPEED, Segment,
    World, cleat,
};
use starhermit_guest::{Handle as H, json::Json, new, read_file};
use std::collections::HashMap;
pub fn href(h: H) -> String {
    format!(r#"{{"$h":{}}}"#, h.0)
}
pub fn group(parent: H) -> H {
    let h = new("Group", "");
    parent.add(h);
    h
}
pub fn mat(color: u32, metal: f32, rough: f32) -> H {
    new(
        "MeshStandardMaterial",
        &format!(
            r#"[{{"color":{color},"roughness":{rough},"metalness":{metal},"side":"double"}}]"#
        ),
    )
}
pub fn mesh(parent: H, geo: H, material: H) -> H {
    let m = new("Mesh", &Json::args().handle(geo).handle(material).finish());
    parent.add(m);
    m.set("castShadow", "true");
    m.set("receiveShadow", "true");
    m
}
fn geom(pos: &[f32], ix: &[u16]) -> H {
    let g = new("BufferGeometry", "");
    g.set_attribute("position", 3, pos, false);
    g.set_index(ix);
    g.call("computeVertexNormals", "");
    g
}
const PAINT: [u32; 5] = [0xc43b26, 0x1e847f, 0xf3ba47, 0x668ac1, 0xaa5880];
pub const NAMES: [&str; 3] = ["Sprint GT", "Vista Wagon", "Orchard Pickup"];
pub struct Model {
    pub root: H,
    nodes: [H; 8],
    groups: Vec<H>,
    finish: H,
}
fn load_water(level: u32) -> Option<crate::water::Waves> {
    let tier = if level < 3 {
        0
    } else if level < 7 {
        1
    } else {
        2
    };
    let path = format!("assets/bucket-{tier}.wabc");
    let result = read_file(&path)
        .ok_or_else(|| format!("Missing {path}"))
        .and_then(crate::water::Waves::open);
    match result {
        Ok(w) => Some(w),
        Err(e) => {
            starhermit_guest::log(starhermit_guest::Level::Error, &e);
            None
        }
    }
}
// The water climbs slightly where it meets the bucket wall.
fn meniscus(positions: &mut [f32]) {
    for p in positions.chunks_exact_mut(3) {
        let rim = ((p[0] * p[0] + p[2] * p[2] - 0.80) / 0.20).clamp(0., 1.);
        p[1] += 0.055 * rim * rim;
    }
}
struct WaterSurface {
    geometry: H,
    positions: Vec<f32>,
    stamp: Vec<(u32, usize)>,
}
pub struct Studio {
    pub world: H,
    pub track: H,
    pub camera: H,
    sky: H,
    weather: H,
    clouds: H,
    cloud_inputs: String,
    pub car_assets: HashMap<(u8, u32), Vec<(usize, usize, H)>>,
    pub mats: Vec<H>,
    cube: H,
    cylinder: H,
    water: H,
    glass: H,
    rim_glass: H,
    bucket_optics: Vec<H>,
    water_side: H,
    open_cylinder: H,
    basin: H,
    bubble_finish: H,
    bubble_nodes: Vec<H>,
    visible_bubbles: usize,
    floor_ring: H,
    bead: H,
    bead_finish: H,
    spray: Vec<H>,
    visible_drops: usize,
    bucket_group: H,
    bucket_nodes: Vec<H>,
    water_source: Option<crate::water::Waves>,
    surfaces: Vec<WaterSurface>,
    wood: H,
    green: H,
    red: H,
    ivory: H,
    shadow: H,
    brass: H,
    rail: H,
    track_geo: Vec<H>,
    track_nodes: Vec<H>,
    pub level: u32,
    flights: Vec<(H, Segment, usize)>,
    pulleys: Vec<H>,

    flywheels: Vec<(H, f32, usize)>,
    drums: Vec<(H, usize)>,
    levers: Vec<(H, Segment)>,
    pub socket: H,
    /// Teleport sparkles: engine particles, depth-tested against the scene.
    pub portal_fx: H,
    sun: H,

    pub stage: H,

    pub center: [f32; 3],
    pub radius: f32,
}
impl Studio {
    pub fn new(level: u32) -> Self {
        let world = group(H::SCENE);
        let camera = new("PerspectiveCamera", "[37,1,0.05,400]");
        H::RENDERER.set_handle("camera", camera);
        H::SCENE.set_handle("background", new("Color", "[15720927]"));
        H::RENDERER.set(
            "shadowMap.enabled",
            if level >= 2 { "true" } else { "false" },
        );
        // Khronos PBR Neutral keeps the toys' paint and track colours true to their base
        // colours, where ACES Filmic shifted saturated reds and oranges and flattened them.
        H::RENDERER.set_f64("toneMapping", 7.);
        H::RENDERER.set_f64("toneMappingExposure", 0.92);
        H::SCENE.set_f64("environmentIntensity", 0.38);
        H::RENDERER.set("grade",r#"{"space":"display","contrastCurve":"s","contrast":0.12,"saturation":0.98,"vignette":{"amount":0.19,"inner":0.4,"outer":0.96,"scale":[1,1]}}"#);
        let sun = new("DirectionalLight", "[16770007,2.0]");
        sun.set_position(-13., 24., 12.);
        H::SCENE.add(sun);
        sun.set("castShadow", if level > 0 { "true" } else { "false" });
        for (key, v) in [
            ("left", -28.),
            ("right", 28.),
            ("top", 28.),
            ("bottom", -28.),
            ("far", 90.),
        ] {
            sun.set_f64(&format!("shadow.camera.{key}"), v)
        }
        sun.set_f64("shadow.bias", -0.0012);
        sun.set_f64("shadow.normalBias", 0.038);
        sun.call("shadow.camera.updateProjectionMatrix", "");
        let sz = if level < 4 {
            512
        } else if level < 8 {
            1024
        } else {
            2048
        };
        sun.call("shadow.mapSize.set", &format!("[{sz},{sz}]"));
        H::SCENE.add(new("HemisphereLight", "[16774372,11245434,0.81]"));
        let room = new("RoomEnvironment", &format!("[{}]", href(H::RENDERER)));
        let pm = new("PMREMGenerator", &format!("[{}]", href(H::RENDERER)));
        let target = pm.call_handle("fromScene", &format!("[{},0.04]", href(room)));
        H::SCENE.set_handle("environment", target.get_handle("texture"));
        let sky = starhermit_guest::module_new(
            "starhermit:sky",
            "SkyAtmosphere",
            r#"[{"day":172,"hours":10,"intensity":9,"environment":false,"moon":false,"milkyWay":false,"turbidity":0.78}]"#,
        );
        H::SCENE.add(sky);
        let weather = starhermit_guest::module_new(
            "starhermit:weather",
            "WeatherState",
            r#"[{"coverage":0.37,"cloudType":0.98,"base":650,"thickness":1800,"density":1.12,"cirrus":0.04,"windSpeed":5}]"#,
        );
        let clouds = starhermit_guest::module_new(
            "starhermit:weather",
            "VolumetricClouds",
            &format!(r#"[{},"full"]"#, href(sky)),
        );
        // The fair-weather parameters stay fixed while WeatherState advances their wind offset.
        let cloud_inputs = Json::args()
            .handle(weather)
            .handle(weather.get_handle("current"))
            .handle(camera)
            .v4(0., 1., 0., 0.)
            .finish();
        let mut mats = vec![];
        for (color, m, r) in [
            (0xcc503e, 0.65, 0.22),
            (0x292b2c, 0.04, 0.78),
            (0x58473e, 0., 0.7),
            (0xa5ced4, 0.12, 0.13),
            (0xcdcfc6, 0.95, 0.21),
            (0xb36f41, 0., 0.67),
            (0xe8d9ac, 0.12, 0.42),
            (0xffedbb, 0.35, 0.16),
            (0xad1815, 0.3, 0.18),
        ] {
            mats.push(mat(color, m, r))
        }
        mats[3].set("transparent", "true");
        mats[3].set_f64("opacity", 0.40);
        mats[3].set("depthWrite", "false");
        let wood = mat(0xf4dbba, 0., 0.49);
        let tex = new("TextureLoader", "")
            .call_handle("load", &format!(r#"["assets/maple-{}.jpg"]"#, level));
        tex.set("colorSpace", r#""srgb""#);
        tex.set("wrapS", r#""repeat""#);
        tex.set("wrapT", r#""repeat""#);
        tex.call("repeat.set", "[3,3]");
        wood.set_handle("map", tex);
        let s = Self {
            world,
            track: group(world),
            camera,
            sky,
            weather,
            clouds,
            cloud_inputs,
            car_assets: HashMap::new(),
            mats,
            cube: new("BoxGeometry", "[1,1,1]"),
            cylinder: new("CylinderGeometry", &format!("[1,1,1,{}]", 12 + level * 4)),
            water: new(
                "MeshPhysicalMaterial",
                r#"[{"color":15728124,"metalness":0,"roughness":0.055,"ior":1.333,"thickness":0.85,"attenuationColor":10346714,"attenuationDistance":3.8}]"#,
            ),
            bucket_optics: vec![],
            rim_glass: new(
                "ShaderMaterial",
                &Json::args()
                    .object()
                    .key("fragmentShader")
                    .str(include_str!("glass_rim.wgsl"))
                    .key("transparent")
                    .bool(true)
                    .key("depthWrite")
                    .bool(false)
                    .end()
                    .finish(),
            ),
            glass: new(
                "MeshPhysicalMaterial",
                r#"[{"color":13823985,"roughness":0.075,"metalness":0.04,"ior":1.52,"clearcoat":1,"transparent":true,"opacity":0.20,"side":"double","depthWrite":false}]"#,
            ),
            water_side: new(
                "MeshStandardMaterial",
                r#"[{"color":5297120,"roughness":0.17,"transparent":true,"opacity":0.14,"side":"double","depthWrite":false}]"#,
            ),
            open_cylinder: new("CylinderGeometry", "[1,1,1,48,1,true]"),
            basin: new(
                "MeshPhysicalMaterial",
                r#"[{"color":10870472,"roughness":0.1,"clearcoat":1,"transparent":true,"opacity":0.55,"depthWrite":false}]"#,
            ),
            bubble_finish: new(
                "MeshPhysicalMaterial",
                r#"[{"color":14482424,"roughness":0.045,"metalness":0.30,"clearcoat":1}]"#,
            ),
            bubble_nodes: vec![],
            visible_bubbles: 0,
            floor_ring: new("TorusGeometry", "[1,0.012,6,64]"),
            bead: new("SphereGeometry", "[1,8,6]"),
            bead_finish: new(
                "MeshPhysicalMaterial",
                r#"[{"color":12840429,"roughness":0.06,"ior":1.333,"transparent":true,"opacity":0.72,"depthWrite":false}]"#,
            ),
            spray: vec![],
            visible_drops: 0,
            bucket_group: group(world),
            bucket_nodes: vec![],
            water_source: load_water(level),
            surfaces: vec![],
            wood,
            green: mat(0x396a5a, 0.03, 0.58),
            red: mat(0xd84c38, 0.16, 0.36),
            ivory: mat(0xffeac0, 0.05, 0.42),
            shadow: mat(0x284841, 0.06, 0.57),
            brass: mat(0xd9a35f, 0.74, 0.29),
            rail: mat(0xf49641, 0.19, 0.33),
            level,
            track_geo: vec![],
            track_nodes: vec![],
            flights: vec![],
            pulleys: vec![],

            flywheels: vec![],
            drums: vec![],
            levers: vec![],
            socket: H::NONE,
            portal_fx: {
                let sparkle = new(
                    "PointsMaterial",
                    r#"[{"size":0.42,"sizeAttenuation":true,"transparent":true,"depthWrite":false,"blending":2}]"#,
                );
                let fx = new(
                    "ParticleEmitter",
                    &format!(
                        r#"[{{"material":{},"capacity":768,"rate":0,"space":"world","shape":"ring","extent":[1.5,0,1.5],"velocityMode":"radial","speed":[1.2,3.2],"gravity":[0,4.5,0],"drag":1.6,"life":[0.6,1.0],"size":[1,0.7,0],"opacity":[1,0.8,0],"color":16777215}}]"#,
                        href(sparkle)
                    ),
                );
                sparkle.release();
                H::SCENE.add(fx);
                fx
            },
            sun,

            stage: group(H::SCENE),

            center: [0.; 3],
            radius: 12.,
        };
        s.optics();
        s.scenery();
        s
    }
    fn shape(&self, p: H, g: H, m: H, xyz: [f32; 3], scale: [f32; 3]) -> H {
        let h = mesh(p, g, m);
        h.set_trs(xyz, [0., 0., 0., 1.], scale);
        h
    }
    fn scenery(&self) {
        self.shape(
            self.world,
            self.cube,
            self.wood,
            [0., -0.7, 0.],
            [500., 1.4, 500.],
        );
        self.shape(
            self.stage,
            self.cube,
            self.wood,
            [0., -0.55, 0.],
            [160., 0.18, 160.],
        );
        self.shape(
            self.stage,
            self.cylinder,
            self.shadow,
            [0., -0.26, 0.],
            [3.8, 0.40, 3.8],
        );
        self.shape(
            self.stage,
            self.cylinder,
            self.brass,
            [0., -0.062, 0.],
            [3.78, 0.025, 3.78],
        );
        self.shape(
            self.stage,
            self.cylinder,
            self.wood,
            [0., -0.038, 0.],
            [3.75, 0.048, 3.75],
        );
        self.stage.set_visible(false);
    }
    // The engine's water shading (starhermit:water WaterMaterial): refraction of the scene
    // behind through renderer.sceneColor/sceneDepth, Beer–Lambert absorption, Fresnel sky
    // reflection and the sun's glint. The baked bucket waves supply the surface itself.
    fn engine_water(&self) -> H {
        let material = starhermit_guest::module_new(
            "starhermit:water",
            "WaterMaterial",
            r#"[{"ripples":false,"roughness":0.03,"absorption":[0.10,0.040,0.021],"scattering":[0.010,0.030,0.032],"skyZenith":10466493,"skyHorizon":13220771}]"#,
        );
        let sun = new("Vector3", "[-13,24,12]");
        sun.call("normalize", "");
        material.call("setSun", &format!("[{}]", href(sun)));
        sun.release();
        material
    }
    pub fn buckets(&mut self, items: &[Bucket]) {
        self.world.remove(self.bucket_group);
        for h in self.bucket_nodes.drain(..) {
            h.release()
        }
        self.bucket_group.release();
        self.bucket_group = group(self.world);
        for surface in self.surfaces.drain(..) {
            surface.geometry.call("dispose", "");
            surface.geometry.release();
        }
        for material in self.bucket_optics.drain(..) {
            material.call("dispose", "");
            material.release();
        }
        let screen_effects = H::RENDERER.get("capabilities.screenEffects");
        for b in items {
            let (top_water, side_water) = if screen_effects.as_str() == Some("full") {
                let material = self.engine_water();
                self.bucket_optics.push(material);
                (material, material)
            } else {
                (self.water, self.water_side)
            };
            let count = 32;
            let mut vs = vec![];
            let mut ix = vec![];
            for i in 0..=count {
                let a = std::f32::consts::TAU * i as f32 / count as f32;
                for (j, (r, y)) in [
                    (b.radius + 0.055, 0.),
                    (b.radius + 0.055, b.rim()),
                    (b.radius - 0.055, b.rim()),
                    (b.radius - 0.055, 0.),
                ]
                .into_iter()
                .enumerate()
                {
                    vs.extend([b.x + r * a.sin(), y, b.z + r * a.cos()]);
                    if i < count {
                        let v = (4 * i + j) as u16;
                        let n = (4 * i + (j + 1) % 4) as u16;
                        ix.extend([v, n, v + 4, n, n + 4, v + 4]);
                    }
                }
            }
            let g = geom(&vs, &ix);
            let wall = mesh(self.bucket_group, g, self.glass);
            wall.set("castShadow", "false");
            wall.set_f64("renderOrder", 3.);
            self.bucket_nodes.push(wall);
            let side = self.shape(
                self.bucket_group,
                self.open_cylinder,
                side_water,
                [b.x, b.surface() * 0.5, b.z],
                [b.radius - 0.06, b.surface(), b.radius - 0.06],
            );
            side.set("castShadow", "false");
            side.set_f64("renderOrder", 2.);
            self.bucket_nodes.push(side);
            g.release();
            let floor = self.shape(
                self.bucket_group,
                self.cylinder,
                self.basin,
                [b.x, 0.04, b.z],
                [b.radius - 0.05, 0.07, b.radius - 0.05],
            );
            self.bucket_nodes.push(floor);
            // The moulded ribs on the basin floor make refraction visible through still water.
            for (fraction, y) in [(0.52, 0.081), (0.88, 0.081), (1., b.rim()), (1., 0.09)] {
                let r = b.radius * fraction;
                let ring = self.shape(
                    self.bucket_group,
                    self.floor_ring,
                    if fraction == 1. {
                        self.rim_glass
                    } else {
                        self.basin
                    },
                    [b.x, y, b.z],
                    [r, r, r],
                );
                ring.set("castShadow", "false");
                if fraction == 1. {
                    ring.set_f64("renderOrder", 4.);
                }
                ring.set_f64("rotation.x", std::f64::consts::FRAC_PI_2);
                self.bucket_nodes.push(ring);
            }
            if let Some(w) = &self.water_source {
                let mut positions: Vec<f32> = w.positions.iter().flatten().copied().collect();
                meniscus(&mut positions);
                let geometry = geom(&positions, &w.indices);
                let h = self.shape(
                    self.bucket_group,
                    geometry,
                    top_water,
                    [b.x, b.surface(), b.z],
                    [b.radius - 0.055, 1., b.radius - 0.055],
                );
                h.set("castShadow", "false");
                self.bucket_nodes.push(h);
                self.surfaces.push(WaterSurface {
                    geometry,
                    positions,
                    stamp: vec![],
                });
            } else {
                self.bucket_nodes.push(self.shape(
                    self.bucket_group,
                    self.cylinder,
                    top_water,
                    [b.x, b.surface(), b.z],
                    [b.radius - 0.055, 0.02, b.radius - 0.055],
                ));
            }
        }
        self.water_motion(items);
    }
    pub fn water_motion(&mut self, items: &[Bucket]) {
        self.splash_drops(items);
        let mut used = 0;
        for b in items
            .iter()
            .flat_map(|bucket| &bucket.bubbles)
            .take(24 + self.level as usize * 12)
        {
            if used == self.bubble_nodes.len() {
                let node = mesh(self.world, self.bead, self.bubble_finish);
                node.set("castShadow", "false");
                self.bubble_nodes.push(node);
            }
            let node = self.bubble_nodes[used];
            if used >= self.visible_bubbles {
                node.set_visible(true);
            }
            node.set_trs(b.position, [0., 0., 0., 1.], [b.radius; 3]);
            used += 1;
        }
        for node in self
            .bubble_nodes
            .iter()
            .take(self.visible_bubbles)
            .skip(used)
        {
            node.set_visible(false);
        }
        self.visible_bubbles = used;
        let Some(w) = &mut self.water_source else {
            return;
        };
        for (surface, b) in self.surfaces.iter_mut().zip(items) {
            let stamp = w.stamp(&b.impacts);
            if stamp == surface.stamp {
                continue;
            }
            surface.stamp = stamp;
            match w.compose(&b.impacts, &mut surface.positions) {
                Ok(()) => {
                    meniscus(&mut surface.positions);
                    surface
                        .geometry
                        .update_attribute("position", 3, &surface.positions, 0);
                    surface.geometry.call("computeVertexNormals", "");
                }
                Err(e) => starhermit_guest::log(starhermit_guest::Level::Error, &e),
            }
        }
    }
    fn splash_drops(&mut self, items: &[Bucket]) {
        let mut used = 0;
        'buckets: for b in items {
            for hit in b
                .impacts
                .iter()
                .filter(|h| h.age < 0.8 && h.strength > 0.48)
            {
                let count = 4 + self.level.min(8);
                for i in 0..count {
                    if used >= 128 {
                        break 'buckets;
                    }
                    // Each droplet has its own launch angle and speed, fixed for this impact.
                    let seed = hit
                        .id
                        .wrapping_mul(2654435761)
                        .wrapping_add(i.wrapping_mul(2246822519));
                    let jitter = (seed >> 16) as f32 / 65535.;
                    let t = hit.age;
                    let angle = hit.angle
                        + (i as f32 + 0.65 * jitter) * std::f32::consts::TAU / count as f32;
                    let up = (1.7 + 1.4 * jitter) * hit.strength.sqrt();
                    let y = t * (up - 4.905 * t);
                    if y <= 0.012 {
                        continue;
                    }
                    let radius = 0.13 + t * (0.65 + 0.95 * jitter) * hit.strength;
                    let x = hit.x * b.radius + radius * angle.cos();
                    let z = hit.z * b.radius + radius * angle.sin();
                    if x.hypot(z) > b.radius - 0.07 && b.surface() + y < b.rim() {
                        continue;
                    }
                    if used == self.spray.len() {
                        let drop = mesh(self.world, self.bead, self.bead_finish);
                        drop.set("castShadow", "false");
                        self.spray.push(drop);
                    }
                    let drop = self.spray[used];
                    if used >= self.visible_drops {
                        drop.set_visible(true);
                    }
                    let r = (0.027 + 0.025 * jitter) * hit.strength.sqrt();
                    drop.set_trs(
                        [b.x + x, b.surface() + y, b.z + z],
                        [0., 0., 0., 1.],
                        [r, r * (1. + 0.16 * (up - 9.81 * t).abs()), r],
                    );
                    used += 1;
                }
            }
        }
        for drop in self.spray.iter().take(self.visible_drops).skip(used) {
            drop.set_visible(false);
        }
        self.visible_drops = used;
    }
    fn lift(&mut self, r: &Segment) {
        let mount = group(self.track);
        self.track_nodes.push(mount);
        mount.set_position(r.p.x, r.p.y + LIFT_FLIGHT_HEIGHT, r.p.z);
        mount.set_f64("rotation.y", -f64::from(r.p.a));
        let frame = group(mount);
        self.track_nodes.push(frame);
        frame.set_f64("rotation.z", f64::from(r.rise.atan2(r.len)));
        let span = r.len.hypot(r.rise);
        let radius = LIFT_PULLEY_RADIUS;

        // Twin endless belts carry the ends of each free-pivoting pusher bar.
        // Both pulleys share a through-shaft, so the two sides run in step.
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let steps = 8 + self.level as usize;
        for side in [-1., 1.] {
            let z = side * 0.79;
            let first = vertices.len() / 3;
            for end in 0..2 {
                let hub = if end == 0 {
                    span - LIFT_INSET
                } else {
                    LIFT_INSET
                };
                for i in 0..=steps {
                    let angle = std::f32::consts::PI * (end as f32 + i as f32 / steps as f32);
                    for (offset, dz) in [
                        (-0.03, -0.056),
                        (0.03, -0.056),
                        (0.03, 0.056),
                        (-0.03, 0.056),
                    ] {
                        vertices.extend([
                            hub + (radius + offset) * angle.sin(),
                            (radius + offset) * angle.cos() - radius,
                            z + dz,
                        ]);
                    }
                }
            }
            let sections = 2 * (steps + 1);
            for i in 0..sections {
                for j in 0..4 {
                    let at = |i: usize, j: usize| (first + 4 * (i % sections) + j % 4) as u16;
                    indices.extend([
                        at(i, j),
                        at(i + 1, j),
                        at(i, j + 1),
                        at(i, j + 1),
                        at(i + 1, j),
                        at(i + 1, j + 1),
                    ]);
                }
            }
            let rail = self.shape(
                frame,
                self.cube,
                self.green,
                [span * 0.5, -radius, side * 0.98],
                [span, 0.18, 0.14],
            );
            self.track_nodes.push(rail);
        }
        let belt = geom(&vertices, &indices);
        self.track_geo.push(belt);
        self.track_nodes.push(mesh(frame, belt, self.shadow));

        for x in [LIFT_INSET, span - LIFT_INSET] {
            let hub = group(frame);
            hub.set_position(x, -radius, 0.);
            self.track_nodes.push(hub);
            self.pulleys.push(hub);
            let axle = self.shape(
                hub,
                self.cylinder,
                self.brass,
                [0.; 3],
                [0.085, 2.18, 0.085],
            );
            axle.set_f64("rotation.x", std::f64::consts::FRAC_PI_2);
            self.track_nodes.push(axle);
            for side in [-1., 1.] {
                let wheel = self.shape(
                    hub,
                    self.cylinder,
                    self.brass,
                    [0., 0., side * 0.79],
                    [radius - 0.025, 0.16, radius - 0.025],
                );
                wheel.set_f64("rotation.x", std::f64::consts::FRAC_PI_2);
                self.track_nodes.push(wheel);
                for j in 0..3 {
                    let spoke = self.shape(
                        hub,
                        self.cube,
                        self.shadow,
                        [0., 0., side * 0.879],
                        [0.56, 0.043, 0.02],
                    );
                    spoke.set_f64("rotation.z", f64::from(j) * std::f64::consts::PI / 3.);
                    self.track_nodes.push(spoke);
                }
            }
        }
        let drive = self.shape(
            frame,
            self.cylinder,
            self.green,
            [span - LIFT_INSET, -radius, 1.22],
            [0.25, 0.46, 0.25],
        );
        drive.set_f64("rotation.x", std::f64::consts::FRAC_PI_2);
        self.track_nodes.push(drive);

        for i in 0..CLEATS {
            let p = cleat(r, i, 0.);
            let flight = group(self.track);
            flight.set_position(p.x, p.y, p.z);
            flight.set_f64("rotation.y", -f64::from(p.a));
            self.track_nodes.push(flight);
            self.flights.push((flight, *r, i));
            let pin = self.shape(
                flight,
                self.cylinder,
                self.brass,
                [0.; 3],
                [0.075, 1.76, 0.075],
            );
            pin.set_f64("rotation.x", std::f64::consts::FRAC_PI_2);
            self.track_nodes.push(pin);
            let shoe = self.shape(flight, self.cube, self.shadow, [0.; 3], [0.22, 0.44, 1.48]);
            self.track_nodes.push(shoe);
            let edge = self.shape(
                flight,
                self.cube,
                self.brass,
                [0., -0.17, 0.],
                [0.225, 0.075, 1.48],
            );
            self.track_nodes.push(edge);
        }
    }
    pub fn road(&mut self, road: &[Segment], buckets: &[Bucket]) {
        self.world.remove(self.track);
        for h in self.track_nodes.drain(..) {
            h.release()
        }
        for g in self.track_geo.drain(..) {
            g.call("dispose", "");
            g.release()
        }
        self.track.release();
        self.track = group(self.world);
        self.flights.clear();
        self.pulleys.clear();
        self.flywheels.clear();
        self.drums.clear();
        self.levers.clear();
        let mut motor_index = 0;
        let mut highest = 0.0_f32;
        let (mut xmin, mut xmax, mut zmin, mut zmax) = (0f32, 0f32, 0f32, 0f32);
        for r in road {
            let n = 10 + self.level as usize * 2;
            let mut verts = vec![];
            let mut ix = vec![];

            for i in 0..=n {
                let p = r.at(i as f32 / n as f32);
                highest = highest.max(p.y);
                xmin = xmin.min(p.x);
                xmax = xmax.max(p.x);
                zmin = zmin.min(p.z);
                zmax = zmax.max(p.z);
                for j in 0..8 {
                    verts.extend(r.vertex(i as f32 / n as f32, j));
                    if i < n {
                        let a = (i * 8 + j) as u16;
                        let b = (i * 8 + (j + 1) % 8) as u16;
                        ix.extend([a, b, a + 8, b, b + 8, a + 8]);
                    }
                }
            }
            let g = geom(&verts, &ix);
            self.track_geo.push(g);
            self.track_nodes.push(mesh(
                self.track,
                g,
                match r.kind {
                    4 => self.green,
                    5 => self.red,
                    6 | 8 => self.green,
                    7 => self.brass,
                    _ => self.rail,
                },
            ));
            let p = r.at(0.05);
            for side in [-0.75, 0.75] {
                self.track_nodes.push(self.shape(
                    self.track,
                    self.cylinder,
                    self.wood,
                    [
                        p.x - p.a.sin() * side,
                        p.y / 2. - 0.06,
                        p.z + p.a.cos() * side,
                    ],
                    [0.23, p.y - 0.1, 0.23],
                ));
            }
            let joint = r.at(0.015);
            for side in [-0.99, 0.99] {
                let h = self.shape(
                    self.track,
                    self.cube,
                    self.brass,
                    [
                        joint.x - side * joint.a.sin(),
                        joint.y + 0.15,
                        joint.z + side * joint.a.cos(),
                    ],
                    [0.24, 0.046, 0.16],
                );
                h.set_f64("rotation.y", -joint.a as f64);
                self.track_nodes.push(h);
            }
            if r.kind == 6 {
                let hinge = r.lever(0., 0.);
                let arm = group(self.track);
                arm.set_position(hinge.x, hinge.y, hinge.z);
                arm.set_f64(
                    "rotation.y",
                    -(hinge.a + if r.launch { -0.41 } else { 0.41 }) as f64,
                );
                self.track_nodes.push(arm);
                self.levers.push((arm, *r));
                for (geo, material, xyz, size) in [
                    (self.cube, self.brass, [-1.38, 0., 0.], [2.75, 0.56, 0.12]),
                    (
                        self.cube,
                        self.ivory,
                        [-1.38, 0.294, 0.],
                        [2.75, 0.035, 0.14],
                    ),
                    (
                        self.cylinder,
                        self.shadow,
                        [0., 0.17, 0.],
                        [0.28, 0.32, 0.28],
                    ),
                    (self.cube, self.brass, [0.37, 0.40, 0.], [0.85, 0.11, 0.13]),
                    (
                        self.cylinder,
                        self.red,
                        [0.79, 0.58, 0.],
                        [0.19, 0.32, 0.19],
                    ),
                ] {
                    self.track_nodes
                        .push(self.shape(arm, geo, material, xyz, size));
                }
            }
            if r.kind == 5 {
                let q = r.at(0.65);
                if r.high_rails {
                    for side in [-1., 1.] {
                        let mount = group(self.track);
                        self.track_nodes.push(mount);
                        mount.set_position(
                            q.x + side * q.a.cos() * 0.48,
                            q.y - 0.245,
                            q.z + side * q.a.sin() * 0.48,
                        );
                        mount.set_f64("rotation.y", -q.a as f64);
                        let drum = group(mount);
                        self.track_nodes.push(drum);
                        self.drums.push((drum, motor_index));
                        let tyre = self.shape(
                            drum,
                            self.cylinder,
                            self.shadow,
                            [0., 0., 0.],
                            [0.28, 1.70, 0.28],
                        );
                        tyre.set_f64("rotation.x", 1.570796);
                        self.track_nodes.push(tyre);
                        for j in 0..6 {
                            let a = j as f32 * std::f32::consts::TAU / 6.;
                            let stripe = self.shape(
                                drum,
                                self.cube,
                                self.ivory,
                                [a.sin() * 0.283, a.cos() * 0.283, 0.],
                                [0.052, 0.014, 1.64],
                            );
                            stripe.set_f64("rotation.z", -a as f64);
                            self.track_nodes.push(stripe);
                        }
                        for z in [-1.06, 1.06] {
                            self.track_nodes.push(self.shape(
                                mount,
                                self.cube,
                                self.red,
                                [0., 0.05, z],
                                [0.76, 0.48, 0.34],
                            ));
                        }
                    }
                } else {
                    for side in [-1., 1.] {
                        let x = q.x - side * q.a.sin() * 1.03;
                        let z = q.z + side * q.a.cos() * 1.03;
                        self.track_nodes.push(self.shape(
                            self.track,
                            self.cylinder,
                            self.red,
                            [x, q.y + 0.04, z],
                            [0.57, 0.22, 0.57],
                        ));
                        self.track_nodes.push(self.shape(
                            self.track,
                            self.cylinder,
                            self.shadow,
                            [x, q.y + 0.31, z],
                            [0.46, 0.31, 0.46],
                        ));
                        self.track_nodes.push(self.shape(
                            self.track,
                            self.cylinder,
                            self.brass,
                            [x, q.y + 0.49, z],
                            [0.30, 0.065, 0.30],
                        ));
                        let spoke = self.shape(
                            self.track,
                            self.cube,
                            self.ivory,
                            [x, q.y + 0.533, z],
                            [0.54, 0.024, 0.074],
                        );
                        self.track_nodes.push(spoke);
                        self.flywheels.push((spoke, side, motor_index));
                    }
                }
                motor_index += 1;
            }
            if r.kind == 4 {
                self.lift(r);
            }
        }
        let end = road.last().unwrap().at(1.);
        self.socket = self.shape(
            self.track,
            self.cylinder,
            self.ivory,
            [end.x, end.y + 0.05, end.z],
            [0.40, 0.10, 0.40],
        );
        self.track_nodes.push(self.socket);
        self.socket.set_visible(false);
        let p = road[0].p;
        for side in [-1.22, 1.22] {
            self.track_nodes.push(self.shape(
                self.track,
                self.cylinder,
                self.brass,
                [p.x, p.y + 1., p.z + side],
                [0.085, 2., 0.085],
            ));
        }
        for i in 0..8 {
            for j in 0..2 {
                self.track_nodes.push(self.shape(
                    self.track,
                    self.cube,
                    if (i + j) % 2 == 0 {
                        self.ivory
                    } else {
                        self.shadow
                    },
                    [
                        p.x,
                        p.y + 1.84 + j as f32 * 0.30,
                        p.z - 1.06 + i as f32 * 0.30,
                    ],
                    [0.11, 0.30, 0.31],
                ));
            }
        }
        for b in buckets {
            xmin = xmin.min(b.x - b.radius);
            xmax = xmax.max(b.x + b.radius);
            zmin = zmin.min(b.z - b.radius);
            zmax = zmax.max(b.z + b.radius);
        }
        self.center = [
            (xmin + xmax) / 2.,
            0.9 + (highest - 3.6).max(0.) * 0.55,
            (zmin + zmax) / 2.,
        ];
        self.radius = ((xmax - xmin) * 0.5 + 2.).max((zmax - zmin) * 0.5 + 2.);
    }
    pub fn car(&mut self, model: u8, paint: u8) -> Model {
        let tier = match self.level {
            0..=2 => 0,
            3..=5 => 1,
            6..=7 => 2,
            _ => 3,
        };
        let chunks = self
            .car_assets
            .entry((model, tier))
            .or_insert_with(|| load_car(model, tier));
        let root = group(H::SCENE);
        let nodes = std::array::from_fn(|_| group(root));
        let finish = new(
            "MeshPhysicalMaterial",
            &format!(
                r#"[{{"color":{},"metalness":0.54,"roughness":0.24,"clearcoat":1,"clearcoatRoughness":0.12,"side":"double"}}]"#,
                PAINT[paint as usize]
            ),
        );
        let mut groups = vec![];
        for &(g, m, geo) in chunks.iter() {
            groups.push(mesh(
                nodes[g],
                geo,
                if m == 0 { finish } else { self.mats[m] },
            ))
        }
        Model {
            root,
            nodes,
            groups,
            finish,
        }
    }
}
fn load_car(model: u8, tier: u32) -> Vec<(usize, usize, H)> {
    let bytes = read_file(&format!("assets/car-{model}-{tier}.mesh")).unwrap();
    let mut at = 0;
    let word = |at: &mut usize| {
        let v = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap());
        *at += 4;
        v as usize
    };
    let count = word(&mut at);
    let mut out = Vec::new();
    for _ in 0..count {
        let gr = word(&mut at);
        let m = word(&mut at);
        let nv = word(&mut at);
        let ni = word(&mut at);
        let mut pos = Vec::with_capacity(nv * 3);
        let mut normals = Vec::with_capacity(nv * 3);
        let center = wheel_center(gr);
        for _ in 0..nv {
            for axis in 0..6 {
                let v = f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
                at += 4;
                if axis < 3 {
                    pos.push(v - center[axis])
                } else {
                    normals.push(v)
                }
            }
        }
        let ix: Vec<u16> = bytes[at..at + 2 * ni]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        at += 2 * ni;
        let g = new("BufferGeometry", "");
        g.set_attribute("position", 3, &pos, false);
        g.set_attribute("normal", 3, &normals, false);
        g.set_index(&ix);
        out.push((gr, m, g));
    }
    out
}
fn wheel_center(g: usize) -> [f32; 3] {
    if g < 4 {
        [0.; 3]
    } else {
        [
            if g < 6 { -0.81 } else { 0.81 },
            0.27,
            if g % 2 == 0 { 0.51 } else { -0.51 },
        ]
    }
}
impl Model {
    pub fn pose(&self, c: &Car, inspect: bool, explode: f32, angle: f32) {
        self.pose_at(c.p, c.rotation, c.spin, inspect, explode, angle)
    }
    pub fn pose_at(
        &self,
        p: crate::physics::Pose,
        rotation: [f32; 4],
        spins: [f32; 4],
        inspect: bool,
        explode: f32,
        angle: f32,
    ) {
        let q = if inspect {
            [0., (angle / 2.).sin(), 0., (angle / 2.).cos()]
        } else {
            rotation
        };
        self.root
            .set_trs(if inspect { [0.; 3] } else { [p.x, p.y, p.z] }, q, [1.; 3]);
        for (g, h) in self.nodes.iter().enumerate() {
            let mut v = wheel_center(g);
            let mut spin = 0.;

            match g {
                1 => v[1] += explode * 1.36,
                2 => v[1] += explode * 2.12,
                3 => v[1] += explode * 0.60,
                4..=7 => {
                    v[2] *= 1. + explode * 2.2;
                    spin = spins[g - 4]
                }
                _ => (),
            }
            h.set_trs(v, [0., 0., (spin * 0.5).sin(), (spin * 0.5).cos()], [1.; 3]);
        }
    }
    pub fn discard(self) {
        H::SCENE.remove(self.root);
        for h in self.groups.into_iter().chain(self.nodes) {
            h.release()
        }
        self.root.release();
        self.finish.call("dispose", "");
        self.finish.release()
    }
}
impl Studio {
    pub fn sky_frame(&self, dt: f64) {
        let args = format!("[{}]", dt.clamp(0., 0.1));
        self.sky.call("update", &args);
        self.weather.call("advance", &args);
        self.clouds.call("update", &self.cloud_inputs);
    }
    fn optics(&self) {
        self.sky.set(
            "requestedTier",
            match self.level {
                0 => r#""minimal""#,
                1..=2 => r#""reduced""#,
                _ => r#""full""#,
            },
        );
        // The screen-space shader handles capable devices. On reduced hardware
        // keep the cars visible through a translucent surface and glass.
        self.water.set_f64("transmission", 0.);
        self.water.set_f64("opacity", 0.24);
        self.water.set("transparent", "true");
        self.water.set("depthWrite", "false");
        let aa = match self.level {
            0 => r#""off""#,
            1..=3 => r#""fxaa""#,
            4..=6 => r#""smaa""#,
            _ => r#""msaa""#,
        };
        H::RENDERER.set("antialias", aa);
        if self.level >= 6 {
            H::RENDERER.set(
                "contactShadows",
                &format!(
                    r#"{{"light":{},"length":0.28,"thickness":0.16,"intensity":0.42,"steps":12}}"#,
                    href(self.sun)
                ),
            );
        } else {
            H::RENDERER.set("contactShadows", "null")
        }
    }
    pub fn motors(&mut self, sim: &World, dt: f32) {
        for (h, r) in &self.levers {
            if let Some((_, angle)) = sim.gate_state(r.owner) {
                h.set_f64("rotation.y", -(r.p.a + angle) as f64);
            }
        }
        for (h, i) in &self.drums {
            h.set_f64(
                "rotation.z",
                h.get_f64("rotation.z") - (sim.roller_speed(*i) * dt / 0.28) as f64,
            );
        }
        for (h, side, i) in &self.flywheels {
            let a = h.get_f64("rotation.y") - (*side * sim.roller_speed(*i) * dt / 0.46) as f64;
            h.set_f64("rotation.y", a);
        }
        for hub in &self.pulleys {
            hub.set_f64(
                "rotation.z",
                f64::from(-sim.time * LIFT_SPEED / LIFT_PULLEY_RADIUS),
            );
        }
        for (h, r, i) in &self.flights {
            let p = cleat(r, *i, sim.time);
            h.set_position(p.x, p.y, p.z);
        }
    }
    pub fn change_level(&mut self, level: u32) {
        self.water_source = load_water(level);
        self.level = level.min(9);
        self.optics();

        H::RENDERER.set(
            "shadowMap.enabled",
            if level >= 2 { "true" } else { "false" },
        );
        self.sun
            .set("castShadow", if level >= 2 { "true" } else { "false" });
        let res = if level < 4 {
            512
        } else if level < 8 {
            1024
        } else {
            2048
        };
        self.sun
            .call("shadow.mapSize.set", &format!("[{res},{res}]"));
        let loader = new("TextureLoader", "");
        let tex = loader.call_handle("load", &format!(r#"["assets/maple-{}.jpg"]"#, self.level));
        tex.set("colorSpace", r#""srgb""#);
        tex.set("wrapS", r#""repeat""#);
        tex.set("wrapT", r#""repeat""#);
        tex.call("repeat.set", "[3,3]");
        let prior = self.wood.get_handle("map");
        self.wood.set_handle("map", tex);
        prior.call("dispose", "");
        prior.release();
        loader.release();
    }
}
