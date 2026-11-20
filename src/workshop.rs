#[path = "scene.rs"]
mod scene;
use crate::physics::{
    BOOST_CODES, BOOSTS, BUCKET_RADIUS, Bucket, BucketDepth, Car, FORK_RIGHT, LAUNCH, LOOP, Pose,
    SPLASH_RADIUS, START, STARTERS, Segment, World, build_at, nudge, place_track, starter_bucket,
    switch_code,
};
use scene::{Model, NAMES, Studio};
use starhermit_guest::{
    Event, FrameInfo, Game, Handle, TouchPhase, audio, export_game, module_call, module_call_async,
    storage,
};
use std::{cell::Cell, rc::Rc};
const MAX_CARS: usize = 100;
const STARTER_CAR: (u8, u8) = (0, 0);
const CREAM: u32 = 0xfff5df;
const INK: u32 = 0x183e39;
const GO: u32 = 0x187047;
const RED: u32 = 0xbd4934;
#[derive(Copy, Clone, PartialEq)]
enum Page {
    Play,
    Track,
    Garage,
    Parts,
    Options,
    Starters,
    Bucket(BucketDepth),
    Place(u8, u8),
}
struct Toybox {
    studio: Option<Studio>,
    road: Vec<Segment>,
    kinds: Vec<u8>,
    anchors: Vec<Option<Pose>>,
    track_family: Option<u8>,
    new_layout: bool,
    track_sheet: Rc<Cell<Handle>>,
    starter_sheet: Rc<Cell<Handle>>,
    starter_page: usize,
    sim: World,
    launch_boost: u8,
    pending_switch: Option<(usize, u8)>,
    buckets: Vec<Bucket>,
    edit: Option<usize>,
    camera_enabled: bool,
    toddler: bool,
    grab: Option<(usize, f32, f64)>,
    undo: Vec<(Vec<u8>, Vec<Option<Pose>>, u8)>,
    cars: Vec<Car>,
    models: Vec<Model>,
    preview: Option<Model>,
    garage_sheet: Rc<Cell<Handle>>,
    page: Page,
    selected: usize,
    kind: u8,
    color: u8,
    paused: bool,
    idle_camera: crate::camera::IdleCamera,
    follow_camera: crate::camera::FollowCamera,
    /// Each car's pose before the last fixed step; render blends to the current one by the
    /// engine's interpolation so motion stays smooth whatever the display rate.
    previous: Vec<CarFrame>,
    frame_dt: f64,
    portal_seen: Vec<f32>,
    follow_now: Option<(usize, f32)>,
    wake_until: f64,
    collision_clip: Rc<Cell<Handle>>,
    splash_clip: Rc<Cell<Handle>>,
    bump_delay: f32,
    rev_clip: Rc<Cell<Handle>>,
    rev_delay: f32,
    apart: bool,
    spread: f32,
    angle: f32,
    elevation: f32,
    azimuth: f32,
    zoom: f32,
    cam: [f32; 3],
    target: [f32; 3],
    buttons: Vec<(f32, f32, f32, f32, u8)>,
    finger: Option<(f32, f32, f32, f32, bool)>,
    touch_time: f64,
    music: Rc<Cell<Handle>>,
    voice: Handle,
    click: Rc<Cell<Handle>>,
}
fn load_sheet(path: &str) -> Rc<Cell<Handle>> {
    let image = Rc::new(Cell::new(Handle::NONE));
    let ready = image.clone();
    let loader = starhermit_guest::new("TextureLoader", "");
    loader.call_async("loadAsync", &format!(r#"["{path}"]"#), move |result| {
        match result {
            Ok(v) => {
                let texture = v.handle();
                texture.set("colorSpace", r#""srgb""#);
                ready.set(texture)
            }
            Err(e) => starhermit_guest::log(starhermit_guest::Level::Error, &e),
        }
        loader.release();
    });
    image
}
fn clip(path: &str) -> Rc<Cell<Handle>> {
    let h = Rc::new(Cell::new(Handle::NONE));
    let dest = h.clone();
    module_call_async(
        "starhermit:audio",
        "loadClip",
        &format!(r#"["{path}"]"#),
        move |v| {
            if let Ok(v) = v {
                dest.set(v.handle())
            }
        },
    );
    h
}
fn d(name: &str, args: String) {
    module_call("starhermit:draw", name, &args);
}
fn text(t: &str, x: f32, y: f32, size: f32, col: u32, center: bool, serif: bool) {
    d(
        "text",
        format!(
            r#"["{t}",{x},{y},{{"size":{size},"color":{col},"weight":700,"align":"{}","font":"{}"}}]"#,
            if center { "center" } else { "start" },
            if serif { "serif" } else { "sans-serif" }
        ),
    )
}
fn rect(x: f32, y: f32, w: f32, h: f32, r: f32, c: u32) {
    d("roundRect", format!("[{x},{y},{w},{h},{r},{c}]"))
}
fn garage_card_height(f: &FrameInfo) -> f32 {
    let s = f.scale.max(0.8);
    (f.height as f32 * 0.17).clamp(48. * s, 80. * s)
}
impl Toybox {
    fn inspecting(&self) -> bool {
        matches!(self.page, Page::Garage | Page::Parts)
    }
    fn save(&self) {
        let mut v = vec![self.kinds.len() as u8];
        v.extend(&self.kinds);
        for c in &self.cars {
            v.extend([c.model, c.paint])
        }
        storage::set("alpine-v1", &v);
        let mut data = Vec::new();
        for (i, p) in self.anchors.iter().enumerate() {
            if let Some(p) = p {
                data.push(i as u8);
                for f in [p.x, p.y, p.z, p.a] {
                    data.extend(f.to_le_bytes());
                }
            }
        }
        storage::set("track-anchors", &data);
        let data: Vec<u8> = self
            .buckets
            .iter()
            .flat_map(|b| b.x.to_le_bytes().into_iter().chain(b.z.to_le_bytes()))
            .collect();
        storage::set("buckets", &data);
        storage::set(
            "bucket-depths",
            &self
                .buckets
                .iter()
                .map(|b| b.depth as u8)
                .collect::<Vec<_>>(),
        );
        storage::set(
            "catch-pools",
            &self
                .buckets
                .iter()
                .map(|b| {
                    if b.radius == crate::physics::STUNT_POOL_RADIUS {
                        2
                    } else {
                        (b.radius == SPLASH_RADIUS) as u8
                    }
                })
                .collect::<Vec<_>>(),
        );
        storage::set("launch-boost", &[self.launch_boost]);
        storage::set("camera", &[self.camera_enabled as u8]);
    }
    fn layout(kinds: &[u8], anchors: &[Option<Pose>], launch_boost: u8) -> Vec<Segment> {
        let mut road = build_at(kinds, anchors);
        for r in road.iter_mut().skip(kinds.len()).filter(|r| r.kind == 5) {
            r.boost = BOOSTS[launch_boost.saturating_sub(5) as usize];
        }
        road
    }
    fn reset_track(&mut self, index: usize) {
        self.pending_switch = None;
        self.kinds = STARTERS[index].1.to_vec();
        self.anchors = crate::physics::starter_anchors(index);
        self.launch_boost = 5;
        self.road = Self::layout(&self.kinds, &self.anchors, self.launch_boost);
        self.buckets = crate::physics::starter_buckets(index, &self.road).collect();
        self.sim = World::new(&self.road, &self.buckets);
        (self.kind, self.color) = STARTER_CAR;
        self.selected = 0;
        self.cars = vec![self.sim.spawn(self.kind, self.color, &self.road)];
        for model in self.models.drain(..) {
            model.discard();
        }
        let studio = self.studio.as_mut().unwrap();
        self.models.push(studio.car(self.kind, self.color));
        studio.road(&self.road, &self.buckets);
        studio.buckets(&self.buckets);
        self.undo.clear();
        self.finger = None;
        self.bump_delay = 0.;
        self.rev_delay = 0.;
        self.idle_camera.reset();
        self.grab = None;
        self.paused = false;
        self.page(Page::Play);
        self.save();
    }
    fn switch_route(&mut self, i: usize) {
        self.pending_switch = if self.pending_switch.is_some_and(|(owner, _)| owner == i) {
            None
        } else {
            Some((i, switch_code(self.kinds[i], !self.road[i].launch)))
        };
        self.poll_switch();
    }
    fn poll_switch(&mut self) {
        if let Some((i, target)) = self.pending_switch {
            if crate::physics::switch_clear(&self.road[i], &self.cars) {
                self.kinds[i] = target;
                self.road = Self::layout(&self.kinds, &self.anchors, self.launch_boost);
                self.sim.align_switch(&self.road[i]);
                self.pending_switch = None;
                self.save();
            }
        }
    }
    fn remember(&mut self) {
        self.undo
            .push((self.kinds.clone(), self.anchors.clone(), self.launch_boost));
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
    }
    fn apply(&mut self) {
        self.pending_switch = None;
        self.road = Self::layout(&self.kinds, &self.anchors, self.launch_boost);
        self.studio
            .as_mut()
            .unwrap()
            .road(&self.road, &self.buckets);
        self.sim
            .reconfigure(&self.road, &self.buckets, &mut self.cars);
        self.studio.as_mut().unwrap().buckets(&self.buckets);
        self.save()
    }
    fn preview(&mut self) {
        if let Some(m) = self.preview.take() {
            m.discard()
        }
        if self.inspecting() {
            let (k, p) = if self.page == Page::Garage {
                (self.kind, self.color)
            } else {
                let c = &self.cars[self.selected];
                (c.model, c.paint)
            };
            self.preview = Some(self.studio.as_mut().unwrap().car(k, p))
        }
    }
    fn page(&mut self, p: Page) {
        self.edit = None;
        self.track_family = None;
        if !matches!(p, Page::Place(..)) {
            self.new_layout = false;
        }
        self.page = p;
        self.apart = p == Page::Parts;
        self.spread = 0.;
        self.preview()
    }
    fn action(&mut self, id: u8) {
        if self.studio.is_none() {
            return;
        }
        if !self.click.get().is_none() {
            audio::play(self.click.get(), 0.35, false).release();
        }
        match id {
            0 => self.page(Page::Play),
            1 => self.page(if self.page == Page::Track {
                Page::Play
            } else {
                Page::Track
            }),
            2 => self.page(Page::Garage),
            3 => self.paused = !self.paused,
            5 => self.apart = !self.apart,
            6 => {
                self.zoom = (self.zoom * 0.8).max(0.45);
            }
            45 => self.zoom = (self.zoom * 1.25).min(1.8),
            46 => self.edit = None,
            7 => {
                if self.voice.is_none() {
                    if !self.music.get().is_none() {
                        self.voice = audio::play(self.music.get(), 0.5, true)
                    }
                } else {
                    self.voice.call("stop", "");
                    self.voice.release();
                    self.voice = Handle::NONE
                }
            }
            8 => self.page(Page::Options),
            48 => {
                self.starter_page = 0;
                self.page(Page::Starters)
            }
            109 => self.starter_page = self.starter_page.saturating_sub(1),
            110 => self.starter_page += 1,
            100..=108 => self.reset_track(usize::from(id - 100)),
            49 => {
                self.toddler = !self.toddler;
                storage::set("toddler", &[self.toddler as u8]);
            }
            9 => {
                self.camera_enabled = !self.camera_enabled;
                if !self.camera_enabled {
                    self.zoom = 1.;
                    self.azimuth = 0.44
                }
                self.save()
            }
            41 => self.page(Page::Bucket(BucketDepth::Shallow)),
            111..=113 => self.page(Page::Bucket(BucketDepth::from_index(id - 111))),
            42 => {
                if self.buckets.pop().is_some() {
                    self.apply()
                }
                self.page(Page::Play)
            }
            43 | 44 => {
                if let Some(i) = self.edit {
                    let branch = self.road[i].kind == 6;
                    let code = if branch {
                        self.launch_boost
                    } else {
                        self.kinds[i]
                    };
                    let codes: &[u8] = if crate::physics::DRUM_CODES.contains(&code) {
                        &crate::physics::DRUM_CODES
                    } else {
                        &BOOST_CODES
                    };
                    if let Some(k) = codes.iter().position(|c| *c == code) {
                        self.remember();
                        let value = codes[if id == 43 {
                            k.saturating_sub(1)
                        } else {
                            (k + 1).min(codes.len() - 1)
                        }];
                        if branch {
                            self.launch_boost = value;
                        } else {
                            self.kinds[i] = value;
                        }
                        self.apply()
                    }
                }
            }
            60..=64 => self.track_family = Some([0, 1, 3, 4, 5][(id - 60) as usize]),
            65 => self.track_family = None,
            66 => self.track_family = Some(FORK_RIGHT),
            70..=97 => {
                let code = if id >= 94 { FORK_RIGHT } else { (id - 70) / 4 };
                let direction = (id - 70) % 4;
                if let Some(i) = self.edit {
                    self.remember();
                    let mut pose = self.road[i].p;
                    pose.a = f32::from(direction) * std::f32::consts::FRAC_PI_2;
                    if i + 1 < self.kinds.len() && self.anchors[i + 1].is_none() {
                        self.anchors[i + 1] = Some(self.road[i + 1].p);
                    }
                    self.anchors[i] = Some(pose);
                    self.kinds[i] = code;
                    self.track_family = None;
                    self.apply();
                } else if self.new_layout || self.kinds.len() < 36 {
                    self.page(Page::Place(code, direction));
                }
            }
            16 => {
                if self.kinds.len() > 1 {
                    self.remember();
                    let i = self.edit.take().unwrap_or(self.kinds.len() - 1);
                    self.anchors.remove(i);
                    self.kinds.remove(i);
                    self.apply()
                }
            }
            17 => {
                if let Some((old, anchors, boost)) = self.undo.pop() {
                    self.launch_boost = boost;
                    self.edit = None;
                    self.kinds = old;
                    self.anchors = anchors;
                    self.track_family = None;
                    self.apply()
                }
            }
            18 => {
                self.edit = None;
                self.track_family = None;
                self.new_layout = true;
            }
            20..=22 => {
                self.kind = id - 20;
                self.preview()
            }
            30..=34 => {
                self.color = id - 30;
                self.preview()
            }
            40 => {
                if self.cars.len() >= MAX_CARS {
                    let old = self.cars.remove(0);
                    self.sim.remove(&old);
                    self.models.remove(0).discard()
                }
                self.cars
                    .push(self.sim.spawn(self.kind, self.color, &self.road));
                self.models
                    .push(self.studio.as_mut().unwrap().car(self.kind, self.color));
                self.selected = self.cars.len() - 1;
                self.page(Page::Play);
                self.save()
            }
            _ => {}
        }
    }
    fn button(&mut self, id: u8, label: &str, xy: [f32; 2], wh: [f32; 2], color: u32, s: f32) {
        let [x, y] = xy;
        let [w, h] = wh;
        rect(x, y + 2. * s, w, h, 12. * s, 0x9a876a);
        rect(x, y, w, h, 12. * s, color);
        text(
            label,
            x + w / 2.,
            y + h * 0.5 - 9. * s,
            (15. * s).min(w * 1.65 / label.chars().count().max(1) as f32),
            if color == CREAM { INK } else { CREAM },
            true,
            false,
        );
        self.buttons.push((x, y, w, h, id))
    }
    fn garage_cards(&mut self, area: [f32; 4], s: f32, paints: bool) {
        let [x, y, width, height] = area;
        let count = if paints { 5 } else { 3 };
        let gap = 5. * s;
        let w = (width - f32::from(count - 1) * gap) / f32::from(count);
        for i in 0..count {
            let (model, paint, id) = if paints {
                (self.kind, i, 30 + i)
            } else {
                (i, self.color, 20 + i)
            };
            let left = x + f32::from(i) * (w + gap);
            let selected = model == self.kind && paint == self.color;
            rect(left, y + 2. * s, w, height, 12. * s, 0x998568);
            rect(
                left,
                y,
                w,
                height,
                12. * s,
                if selected { GO } else { 0xcdbf9e },
            );
            let border = if selected { 3. * s } else { s };
            rect(
                left + border,
                y + border,
                w - 2. * border,
                height - 2. * border,
                10. * s,
                CREAM,
            );
            let iw = (w - 8. * s).min((height - 6. * s) * 2.);
            let ih = iw / 2.;
            let px = left + (w - iw) / 2.;
            let py = y + (height - ih) / 2.;
            if !self.garage_sheet.get().is_none() {
                d(
                    "image",
                    format!(
                        r#"[{},{px},{py},{iw},{ih},{{"source":{{"x":{},"y":{},"width":240,"height":120}}}}]"#,
                        scene::href(self.garage_sheet.get()),
                        u32::from(model) * 240,
                        u32::from(paint) * 120
                    ),
                );
            }
            if selected {
                let cx = left + w - 12. * s;
                let cy = y + 12. * s;
                d("circle", format!("[{cx},{cy},{},{GO}]", 8. * s));
                d(
                    "line",
                    format!(
                        "[{},{},{},{},{CREAM},{}]",
                        cx - 4. * s,
                        cy,
                        cx - s,
                        cy + 3. * s,
                        2. * s
                    ),
                );
                d(
                    "line",
                    format!(
                        "[{},{},{},{},{CREAM},{}]",
                        cx - s,
                        cy + 3. * s,
                        cx + 4. * s,
                        cy - 3. * s,
                        2. * s
                    ),
                );
            }
            self.buttons.push((left, y, w, height, id));
        }
    }
    fn track_picture(&mut self, code: u8, direction: u8, id: Option<u8>, area: [f32; 4], s: f32) {
        let [x, y, w, h] = area;
        rect(x, y + 2. * s, w, h, 10. * s, 0xab9575);
        rect(x, y, w, h, 10. * s, CREAM);
        let tex = self.track_sheet.get();
        if !tex.is_none() {
            let az = (self.cam[0] - self.target[0]).atan2(self.cam[2] - self.target[2]);
            let view = if code == 6 {
                0
            } else {
                ((az / std::f32::consts::TAU * 16.).round() as i32 + i32::from(direction) * 4)
                    .rem_euclid(16)
            };
            let iw = (w - 4. * s).min((h - 4. * s) * 4. / 3.);
            let ih = iw * 0.75;
            d(
                "image",
                format!(
                    r#"[{},{},{},{iw},{ih},{{"source":{{"x":{},"y":{},"width":128,"height":96}}}}]"#,
                    scene::href(tex),
                    x + (w - iw) * 0.5,
                    y + (h - ih) * 0.5,
                    view * 128,
                    u32::from(if code == FORK_RIGHT { 7 } else { code }) * 96
                ),
            );
        }
        if let Some(id) = id {
            self.buttons.push((x, y, w, h, id));
        }
    }
    fn starter_picker(
        &mut self,
        f: &FrameInfo,
        left: f32,
        top: f32,
        width: f32,
        bottom: f32,
        s: f32,
    ) {
        rect(0., 0., f.width as f32, f.height as f32, 0., 0xeee2c9);
        text(
            "Choose a track",
            left + 7. * s,
            top + 8. * s,
            22. * s,
            INK,
            false,
            true,
        );
        self.button(
            8,
            "↩ Back",
            [left + width - 78. * s, top],
            [78. * s, 44. * s],
            INK,
            s,
        );
        let y = top + 52. * s;
        let gap = 8. * s;
        let grid =
            crate::starter_grid::StarterGrid::fit(width / s, (bottom - y) / s, STARTERS.len());
        self.starter_page = self.starter_page.min(grid.pages - 1);
        let bottom = if grid.pages > 1 {
            let nav_y = bottom - 44. * s;
            let mid = left + width * 0.5;
            if self.starter_page > 0 {
                self.button(
                    109,
                    "◀",
                    [mid - 106. * s, nav_y],
                    [58. * s, 44. * s],
                    INK,
                    s,
                );
            }
            if self.starter_page + 1 < grid.pages {
                self.button(110, "▶", [mid + 48. * s, nav_y], [58. * s, 44. * s], GO, s);
            }
            for j in 0..grid.pages {
                let cx = mid + (j as f32 - (grid.pages - 1) as f32 * 0.5) * 16. * s;
                let color = if j == self.starter_page { GO } else { 0xad9879 };
                d(
                    "circle",
                    format!("[{cx},{},{},{color}]", nav_y + 22. * s, 5. * s),
                );
            }
            nav_y - 6. * s
        } else {
            bottom
        };
        let (cols, rows) = (grid.cols, grid.rows);
        let cw = (width - gap * (cols - 1) as f32) / cols as f32;
        let ch = (bottom - y - gap * (rows - 1) as f32) / rows as f32;
        let first = self.starter_page * grid.per_page;
        let visible = (STARTERS.len() - first).min(grid.per_page);
        let tex = self.starter_sheet.get();
        for (slot, (name, _)) in STARTERS.iter().skip(first).take(visible).enumerate() {
            let i = first + slot;
            let row_count = (visible - slot / cols * cols).min(cols);
            let left = left + (cols - row_count) as f32 * (cw + gap) * 0.5;
            let x = left + (slot % cols) as f32 * (cw + gap);
            let y = y + (slot / cols) as f32 * (ch + gap);
            rect(x, y + 2. * s, cw, ch, 12. * s, 0xbda989);
            rect(x, y, cw, ch, 12. * s, CREAM);
            let photo_h = ch - 30. * s;
            let iw = (cw - 8. * s).min((photo_h - 2. * s) * (480. / 260.));
            let ih = iw * (260. / 480.);
            if !tex.is_none() {
                d(
                    "image",
                    format!(
                        r#"[{},{},{},{iw},{ih},{{"source":{{"x":{},"y":{},"width":480,"height":260}}}}]"#,
                        scene::href(tex),
                        x + (cw - iw) * 0.5,
                        y + (photo_h - ih) * 0.5 + 2. * s,
                        i / 3 * 480,
                        i % 3 * 260
                    ),
                );
            }
            text(
                name,
                x + cw * 0.5,
                y + ch - 26. * s,
                17. * s,
                INK,
                true,
                true,
            );
            self.buttons.push((x, y, cw, ch, 100 + i as u8));
        }
    }
    fn hud(&mut self, f: &FrameInfo) {
        self.buttons.clear();
        let s = f.scale.max(0.8);
        let left = f.safe_area[3] + 9. * s;
        let top = f.safe_area[0] + 9. * s;
        let w = f.width as f32 - left - f.safe_area[1] - 9. * s;
        let bottom = f.height as f32 - f.safe_area[2] - 10. * s;
        let h = 44. * s;
        let gap = 5. * s;
        if self.page == Page::Starters {
            self.starter_picker(f, left, top, w, bottom, s);
            return;
        }
        let inspect = self.inspecting();
        let title = if matches!(self.page, Page::Place(..)) {
            "Place track"
        } else if self.page == Page::Track && self.track_family.is_some() {
            "Pick a piece"
        } else if self.new_layout {
            "New track"
        } else if self.page == Page::Track && self.edit.is_none() && self.kinds.len() >= 36 {
            "Track full"
        } else if inspect {
            if self.page == Page::Garage {
                NAMES[self.kind as usize]
            } else {
                NAMES[self.cars[self.selected].model as usize]
            }
        } else if let Some(i) = self.edit {
            [
                "Edit flat",
                "Edit left",
                "Edit right",
                "Edit slope",
                "Edit lift",
                "Edit boost",
                "Switch",
                "Edit launch",
            ][self.road[i].kind as usize]
        } else {
            "Mashini"
        };
        let compact = inspect || matches!(self.page, Page::Track | Page::Place(..));
        let tw = if compact { 166. } else { 132. } * s;
        rect(left, top, tw, 54. * s, 11. * s, INK);
        text(
            if inspect {
                "MODEL WORKSHOP"
            } else {
                "TABLETOP WORKSHOP"
            },
            left + 12. * s,
            top + 5. * s,
            9. * s,
            0xe7bb73,
            false,
            false,
        );
        text(
            title,
            left + 11. * s,
            top + 16. * s,
            if compact { 20. } else { 27. } * s,
            CREAM,
            false,
            true,
        );
        if inspect {
            self.button(0, "✓ Done", [left + w - 80. * s, top], [80. * s, h], INK, s)
        } else {
            let right = left + w;
            let mut controls_right = right;
            let mut controls_y = top;
            if self.page == Page::Play {
                let options_width = 72. * s;
                self.button(
                    8,
                    "Options",
                    [right - options_width, top],
                    [options_width, h],
                    INK,
                    s,
                );
                controls_right -= options_width + gap;
                if w < (166. + 72. + 2. * 44. + 3. * 5.) * s {
                    controls_right = right;
                    controls_y = top + 60. * s;
                }
            }
            self.button(
                7,
                "♫",
                [controls_right - 93. * s, controls_y],
                [44. * s, h],
                if self.voice.is_none() { CREAM } else { INK },
                s,
            );
            self.button(
                3,
                if self.paused { "▶" } else { "Ⅱ" },
                [controls_right - 44. * s, controls_y],
                [44. * s, h],
                if self.paused { GO } else { RED },
                s,
            )
        }
        let bw = w.min(620. * s);
        let x = left + (w - bw) / 2.;
        let row = |this: &mut Self, items: &[(u8, &str, u32)], y: f32| {
            let cell = (bw - gap * (items.len() - 1) as f32) / items.len() as f32;
            for (i, (id, label, col)) in items.iter().enumerate() {
                this.button(
                    *id,
                    label,
                    [x + i as f32 * (cell + gap), y],
                    [cell, h],
                    *col,
                    s,
                )
            }
        };
        match self.page {
            Page::Starters => {}
            Page::Play if self.toddler => {}
            Page::Play => row(
                self,
                &[(1, "↗  Tracks", INK), (2, "+  Garage", INK)],
                bottom - h,
            ),
            Page::Options => {
                let reset_y = bottom
                    - if self.camera_enabled {
                        3. * h + 2. * gap
                    } else {
                        2. * h + gap
                    };
                row(
                    self,
                    &[(
                        49,
                        if self.toddler {
                            "Toddler: on"
                        } else {
                            "Toddler: off"
                        },
                        if self.toddler { GO } else { INK },
                    )],
                    reset_y - h - gap,
                );
                row(self, &[(48, "Reset track", RED)], reset_y);
                row(
                    self,
                    &[
                        (
                            9,
                            if self.camera_enabled {
                                "Camera: on"
                            } else {
                                "Camera: off"
                            },
                            INK,
                        ),
                        (0, "Done", RED),
                    ],
                    bottom - h,
                );
                if self.camera_enabled {
                    row(
                        self,
                        &[(6, "Zoom in", INK), (45, "Zoom out", INK)],
                        bottom - 2. * h - gap,
                    )
                }
            }
            Page::Bucket(depth) => {
                row(
                    self,
                    &[(42, "Remove last bucket", INK), (0, "Cancel", INK)],
                    bottom - h,
                );
                let card_w = (w / 3. - gap).min(76. * s);
                let start = left + (w - 3. * card_w - 2. * gap) * 0.5;
                for i in 0..3u8 {
                    let x = start + f32::from(i) * (card_w + gap);
                    let y = bottom - 2. * h - gap;
                    rect(
                        x,
                        y,
                        card_w,
                        h,
                        9. * s,
                        if depth as u8 == i { GO } else { INK },
                    );
                    let width = (card_w * 0.60).min(36. * s);
                    let height = (14. + 7. * f32::from(i)) * s;
                    let bx = x + (card_w - width) * 0.5;
                    let by = y + h - 6. * s - height;
                    rect(bx, by, width, height, 4. * s, CREAM);
                    rect(
                        bx + 2. * s,
                        by + 3. * s,
                        width - 4. * s,
                        height - 5. * s,
                        3. * s,
                        0x57bed0,
                    );
                    self.buttons.push((x, y, card_w, h, 111 + i));
                }
                text(
                    "Tap the tabletop to place water",
                    f.width as f32 / 2.,
                    bottom - 2. * h - gap - 20. * s,
                    17. * s,
                    INK,
                    true,
                    false,
                );
            }
            Page::Place(code, direction) => {
                self.track_picture(code, direction, None, [x, bottom - h, h * 1.5, h], s);
                self.button(
                    1,
                    "↩ Back",
                    [x + bw - h * 2., bottom - h],
                    [h * 2., h],
                    INK,
                    s,
                );
                text(
                    "Tap the tabletop",
                    f.width as f32 / 2.,
                    bottom - h * 0.75,
                    16. * s,
                    INK,
                    true,
                    false,
                );
            }
            Page::Track => {
                let ch = (f.height as f32 * 0.12).clamp(44. * s, 78. * s);
                let base = bottom - h - gap;
                let family = self.track_family;
                let rows = if family.is_none() || family == Some(1) {
                    2
                } else {
                    1
                };
                let grid_y = base - rows as f32 * ch - (rows - 1) as f32 * gap;
                let count = if family.is_none() { 7 } else { rows * 4 };
                for n in 0..count {
                    let (code, dir, id, col, row, cols) = if let Some(code) = family {
                        let code = if code == 1 && n >= 4 { 2 } else { code };
                        let first = if code == FORK_RIGHT {
                            94
                        } else {
                            70 + code * 4
                        };
                        (code, n % 4, first + n % 4, n % 4, n / 4, 4)
                    } else {
                        let (code, id) = [
                            (0, 60),
                            (1, 61),
                            (3, 62),
                            (4, 63),
                            (5, 64),
                            (FORK_RIGHT, 66),
                            (6, 41),
                        ][n as usize];
                        if n < 3 {
                            (code, 0, id, n, 0, 3)
                        } else {
                            (code, 0, id, n - 3, 1, 4)
                        }
                    };
                    let cell = (bw - gap * (cols - 1) as f32) / cols as f32;
                    self.track_picture(
                        code,
                        dir,
                        Some(id),
                        [
                            x + col as f32 * (cell + gap),
                            grid_y + row as f32 * (ch + gap),
                            cell,
                            ch,
                        ],
                        s,
                    );
                }
                if family.is_some() {
                    row(self, &[(65, "↩ Back", INK), (0, "▶ Play", GO)], bottom - h);
                } else {
                    if let Some(i) = self.edit {
                        if self.road[i].kind == 5 || matches!(self.kinds[i], LOOP | LAUNCH) {
                            let piece = self.road[i];
                            let gain = if piece.kind == 6 {
                                BOOSTS[(self.launch_boost - 5) as usize]
                            } else {
                                piece.boost
                            };
                            let label = if piece.high_rails && piece.kind == 5 {
                                format!("Roller {gain} m/s")
                            } else {
                                format!("Boost +{gain} m/s")
                            };
                            row(
                                self,
                                &[
                                    (43, "− Speed", INK),
                                    (47, &label, RED),
                                    (44, "+ Speed", INK),
                                ],
                                grid_y - h - gap,
                            );
                        }
                    }
                    row(
                        self,
                        &[
                            (16, "− Remove", RED),
                            (17, "↶ Undo", INK),
                            if self.edit.is_some() {
                                (46, "＋ Add", INK)
                            } else {
                                (18, "New track", INK)
                            },
                            (0, "▶ Play", GO),
                        ],
                        bottom - h,
                    );
                }
            }
            Page::Garage => {
                let card_height = garage_card_height(f);
                self.garage_cards(
                    [x, bottom - 2. * (h + gap) - card_height, bw, card_height],
                    s,
                    false,
                );
                self.garage_cards([x, bottom - 2. * h - gap, bw, h], s, true);
                let y = bottom - h;
                rect(x, y + 2. * s, bw, h, 12. * s, 0x998568);
                rect(x, y, bw, h, 12. * s, GO);
                let mid = x + bw * 0.5;
                let cy = y + h * 0.5;
                d(
                    "polygon",
                    format!(
                        "[[[{},{}],[{},{}],[{},{}]],{}]",
                        mid - 29. * s,
                        cy - 10. * s,
                        mid - 29. * s,
                        cy + 10. * s,
                        mid - 12. * s,
                        cy,
                        CREAM
                    ),
                );
                text(
                    "Go!",
                    mid - 2. * s,
                    cy - 12. * s,
                    21. * s,
                    CREAM,
                    false,
                    false,
                );
                self.buttons.push((x, y, bw, h, 40));
            }
            Page::Parts => row(
                self,
                &[
                    (
                        5,
                        if self.apart {
                            "↧  Put together"
                        } else {
                            "↥  Take apart"
                        },
                        INK,
                    ),
                    (0, "▶  Back to track", GO),
                ],
                bottom - h,
            ),
        }
    }
    fn project(&self, p: [f32; 3], f: &FrameInfo) -> Option<(f32, f32, f32)> {
        let mut dir = std::array::from_fn::<_, 3, _>(|i| self.target[i] - self.cam[i]);
        let len = dir[0].hypot(dir[1]).hypot(dir[2]);
        if len < 0.001 {
            return None;
        }
        for d in &mut dir {
            *d /= len
        }
        let hz = dir[0].hypot(dir[2]);
        let right = [-dir[2] / hz, 0., dir[0] / hz];
        let up = [-right[2] * dir[1], hz, right[0] * dir[1]];
        let dot = |v: [f32; 3]| (0..3).map(|i| v[i] * (p[i] - self.cam[i])).sum::<f32>();
        let depth = dot(dir);
        if depth <= 0. {
            return None;
        }
        let k = f.height as f32 / (2. * 18.5f32.to_radians().tan() * depth);
        Some((
            f.width as f32 / 2. + k * dot(right),
            f.height as f32 / 2. - k * dot(up),
            k,
        ))
    }
    /// Bursts the engine's particles where a car leaves and where it reappears, once as each
    /// teleport starts (a car's `portal` timer jumps back up).
    fn portals(&mut self) {
        let Some(studio) = self.studio.as_ref() else {
            return;
        };
        self.portal_seen.resize(self.cars.len(), 0.);
        for (c, seen) in self.cars.iter().zip(self.portal_seen.iter_mut()) {
            if c.portal > *seen + 0.01 {
                for (p, color) in [(c.departure, 0xffbc54), (c.arrival, 0x2dbdad)] {
                    studio.portal_fx.call(
                        "burst",
                        &format!(
                            r#"[90,[{},{},{}],{{"color":{color}}}]"#,
                            p.x,
                            p.y + 0.32,
                            p.z
                        ),
                    );
                }
            }
            *seen = c.portal;
        }
    }
    fn callouts(&self, f: &FrameInfo) {
        if self.page != Page::Parts || self.spread < 0.78 {
            return;
        }
        let s = f.scale;
        let bw = 64. * s;
        let h = 25. * s;
        for (name, p, side, dy) in [
            ("Glass", [0., 2.98, 0.], -1., -0.25),
            ("Body", [0.6, 1.87, 0.], 1., -0.15),
            ("Seats", [0., 1.25, 0.], -1., 0.10),
            ("Wheels", [0.81, 0.27, 1.63], 1., 0.22),
        ] {
            let a = self.angle;
            let q = [
                p[0] * a.cos() + p[2] * a.sin(),
                p[1],
                -p[0] * a.sin() + p[2] * a.cos(),
            ];
            if let Some((px, py, _)) = self.project(q, f) {
                let x =
                    f.width as f32 * 0.5 + side * (f.width as f32 * 0.4).min(248. * s) - bw * 0.5;
                let y = f.height as f32 * (0.5 + dy);
                let end = if side < 0. { x + bw } else { x };
                d("line", format!("[{px},{py},{end},{y},{INK},{}]", 1.3 * s));
                d("circle", format!("[{px},{py},{},{CREAM}]", 2.8 * s));
                rect(x, y - h / 2., bw, h, 8. * s, INK);
                text(name, x + bw / 2., y - 7.5 * s, 12. * s, CREAM, true, false);
            }
        }
    }
    fn placement(&self, code: u8, direction: u8, x: f32, y: f32) -> Option<Pose> {
        let heading = f32::from(direction) * std::f32::consts::FRAC_PI_2;
        let [wx, wz] = self.plane(x, y, 0.)?;
        let road = if self.new_layout { &[][..] } else { &self.road };
        let mut pose = place_track(code, heading, wx, wz, road);
        let f = FrameInfo::now();
        let mut best = 16. * f.scale;
        // An elevated connector appears above its footprint on the tabletop.
        for piece in road {
            for end in [0., 1.] {
                let p = piece.at(end);
                if let Some((sx, sy, _)) = self.project([p.x, p.y, p.z], &f) {
                    let distance = (sx - x).hypot(sy - y);
                    if distance < best {
                        let candidate =
                            place_track(code, heading, p.x, p.z, std::slice::from_ref(piece));
                        let segments = build_at(&[code], &[Some(candidate)]);
                        if segments.iter().flat_map(|s| [s.at(0.), s.at(1.)]).any(|q| {
                            (q.x - p.x).hypot(q.z - p.z) < 0.01 && (q.y - p.y).abs() < 0.01
                        }) {
                            best = distance;
                            pose = candidate;
                        }
                    }
                }
            }
        }
        Some(pose)
    }
    fn tap(&mut self, x: f32, y: f32) {
        if let Some(id) = self
            .buttons
            .iter()
            .rev()
            .find(|(a, b, w, h, _)| x >= *a && x <= a + w && y >= *b && y <= b + h)
            .map(|v| v.4)
        {
            self.action(id);
            return;
        }
        if let Page::Place(code, direction) = self.page {
            if let Some(pose) = self.placement(code, direction, x, y) {
                if self.new_layout || self.kinds.len() < 36 {
                    self.remember();
                    if self.new_layout {
                        self.kinds.clear();
                        self.anchors.clear();
                    }
                    self.anchors.push(Some(pose));
                    self.kinds.push(code);
                    self.apply();
                    self.page(Page::Play);
                }
            }
            return;
        }
        if let Page::Bucket(depth) = self.page {
            if self.buckets.len() < 12 {
                if let Some([bx, bz]) = self.plane(x, y, 0.) {
                    self.buckets.push(Bucket {
                        depth,
                        radius: BUCKET_RADIUS,
                        x: bx,
                        z: bz,

                        ..Bucket::default()
                    });
                    self.apply();
                    self.page(Page::Play)
                }
            }
            return;
        }
        if self.inspecting() || matches!(self.page, Page::Options | Page::Starters) {
            return;
        }
        if self.page == Page::Track && self.pick_track(x, y) {
            return;
        }
        let f = FrameInfo::now();
        let mut near = None;
        for (i, c) in self.cars.iter().enumerate() {
            let Some((px, py, k)) = self.project([c.p.x, c.p.y + 0.45, c.p.z], &f) else {
                continue;
            };
            let dist = (x - px).hypot(y - py);
            if dist < (1.45 * k).max(25. * f.scale) && near.is_none_or(|(_, d)| dist < d) {
                near = Some((i, dist))
            }
        }
        if let Some((i, _)) = near {
            self.selected = i;
            self.page(Page::Parts)
        } else {
            self.pick_track(x, y);
        }
    }
    fn pick_track(&mut self, x: f32, y: f32) -> bool {
        let f = FrameInfo::now();
        if self.page == Page::Play {
            for r in self.road.iter().filter(|r| r.kind == 6) {
                let p = r.p;
                let q = r.lever(0., 0.6);
                let knob = [q.x, p.y + 0.8, q.z];
                if let Some((px, py, k)) = self.project(knob, &f) {
                    if (x - px).hypot(y - py) < (k * 0.65).max(14. * f.scale) {
                        self.switch_route(r.owner);
                        return true;
                    }
                }
            }
        }
        let mut nearest = None;
        for (i, r) in self.road.iter().enumerate() {
            for j in 0..=30 {
                let p = r.at(j as f32 / 30.);
                if let Some((px, py, k)) = self.project([p.x, p.y, p.z], &f) {
                    let d = (px - x).hypot(py - y) / k;
                    if d < 1.08 && nearest.is_none_or(|(_, best)| d < best) {
                        nearest = Some((i, d))
                    }
                }
            }
        }
        if let Some((i, _)) = nearest {
            let owner = self.road[i].owner;
            if self.page == Page::Play && matches!(self.road[i].kind, 6 | 8) {
                self.switch_route(owner);
                return true;
            }
            self.page(Page::Track);
            self.edit = Some(owner);
            true
        } else {
            false
        }
    }
    fn plane(&self, x: f32, y: f32, height: f32) -> Option<[f32; 2]> {
        let f = FrameInfo::now();
        let dx = self.target[0] - self.cam[0];
        let dz = self.target[2] - self.cam[2];
        let dy = self.target[1] - self.cam[1];
        let hz = dx.hypot(dz);
        let len = hz.hypot(dy);
        if hz < 0.001 {
            return None;
        }
        let k = 2. * 18.5f32.to_radians().tan() / f.height as f32;
        let u = (x - f.width as f32 / 2.) * k;
        let v = (f.height as f32 / 2. - y) * k;
        let ray_y = (dy + v * hz) / len;
        let t = (height - self.cam[1]) / ray_y;
        if !(0.01..500.).contains(&t) {
            return None;
        }
        let forward = (1. - v * dy / hz) / len;
        Some([
            self.cam[0] + t * (dx * forward - u * dz / hz),
            self.cam[2] + t * (dz * forward + u * dx / hz),
        ])
    }
    fn pointer(&mut self, x: f32, y: f32, kind: u8) {
        match kind {
            0 => {
                self.finger = Some((x, y, x, y, false));
                let f = FrameInfo::now();
                self.grab = None;
                if self.page == Page::Play
                    && !self
                        .buttons
                        .iter()
                        .any(|(a, b, w, h, _)| x >= *a && x <= a + w && y >= *b && y <= b + h)
                {
                    let mut best = f32::INFINITY;
                    for (i, c) in self.cars.iter().enumerate() {
                        if let Some((px, py, k)) = self.project([c.p.x, c.p.y + 0.45, c.p.z], &f) {
                            let d = (px - x).hypot(py - y);
                            if d < best && d < (1.4 * k).max(24. * f.scale) {
                                self.grab = Some((i, c.p.y + 0.45, f.time));
                                best = d
                            }
                        }
                    }
                }
            }
            1 => {
                if let Some((sx, sy, last, last_y, drag)) = self.finger {
                    let moved = drag || (x - sx).hypot(y - sy) > 7. * FrameInfo::now().scale;
                    if moved {
                        let d = (x - last) * 0.008 / FrameInfo::now().scale;
                        if self.inspecting() {
                            self.elevation = (self.elevation
                                + (y - last_y) * 0.006 / FrameInfo::now().scale)
                                .clamp(0.08, 1.15);
                            self.angle += d
                        } else if self.camera_enabled
                            && self.page != Page::Starters
                            && self.grab.is_none()
                        {
                            self.azimuth -= d
                        }
                    }
                    self.finger = Some((sx, sy, x, y, moved))
                }
            }
            _ => {
                if let Some((sx, sy, _, _, drag)) = self.finger.take() {
                    if let Some((i, height, time)) = self.grab.take() {
                        if drag || (sx - x).hypot(sy - y) > 7. * FrameInfo::now().scale {
                            if let (Some(a), Some(b)) =
                                (self.plane(sx, sy, height), self.plane(x, y, height))
                            {
                                let duration =
                                    ((FrameInfo::now().time - time) as f32).clamp(0.12, 1.);
                                self.sim.flick(
                                    &self.cars[i],
                                    nudge([(b[0] - a[0]) / duration, 0., (b[1] - a[1]) / duration]),
                                );
                            }
                        } else {
                            self.selected = i;
                            self.page(Page::Parts)
                        }
                    } else if !drag {
                        self.tap(x, y)
                    }
                }
            }
        }
    }
}
impl Game for Toybox {
    fn init() -> Self {
        std::panic::set_hook(Box::new(|e| starhermit_guest::report_panic(e)));
        let mut kinds = START.to_vec();
        let mut fleet = vec![STARTER_CAR];
        let mut upgrade_oval = false;
        let mut supply_bucket = true;
        let saved_anchors = storage::get("track-anchors");
        if let Some(v) = storage::get("alpine-v1") {
            if let Some(&n) = v.first() {
                let n = n as usize;
                if (1..=36).contains(&n)
                    && v.len() >= n + 3
                    && v[1..=n].iter().all(|k| *k <= crate::physics::CREST)
                {
                    kinds = v[1..=n].to_vec();
                    supply_bucket = saved_anchors.is_none() && kinds == [3, 0, 2, 2, 5, 4, 2, 2];
                    upgrade_oval = saved_anchors.is_none()
                        && (kinds == [14, 15, LOOP, 14, 15, 15]
                            || kinds == [14, 15, LAUNCH, 14, 15, 15]);
                    if supply_bucket || upgrade_oval {
                        kinds = START.to_vec();
                    }
                    let valid: Vec<_> = v[n + 1..]
                        .chunks_exact(2)
                        .filter(|p| p[0] < 3 && p[1] < 5)
                        .take(MAX_CARS)
                        .map(|p| (p[0], p[1]))
                        .collect();
                    if !valid.is_empty() {
                        fleet = valid
                    }
                }
            }
        }
        let launch_boost = storage::get("launch-boost")
            .and_then(|v| v.first().copied())
            .filter(|k| BOOST_CODES.contains(k))
            .unwrap_or(5);
        let mut anchors = vec![None; kinds.len()];
        if !upgrade_oval {
            for record in saved_anchors.unwrap_or_default().chunks_exact(17) {
                let at = |i| f32::from_le_bytes(record[i..i + 4].try_into().unwrap());
                let p = Pose {
                    x: at(1),
                    y: at(5),
                    z: at(9),
                    a: at(13),
                };
                if let Some(slot) = anchors.get_mut(record[0] as usize) {
                    if [p.x, p.y, p.z, p.a].iter().all(|v| v.is_finite()) {
                        *slot = Some(p);
                    }
                }
            }
        }
        let road = Self::layout(&kinds, &anchors, launch_boost);
        let depths = storage::get("bucket-depths").unwrap_or_default();
        let sizes = storage::get("catch-pools").unwrap_or_default();
        let mut buckets: Vec<Bucket> = storage::get("buckets")
            .unwrap_or_default()
            .chunks_exact(8)
            .take(12)
            .enumerate()
            .filter_map(|(i, v)| {
                let x = f32::from_le_bytes(v[..4].try_into().ok()?);
                let z = f32::from_le_bytes(v[4..].try_into().ok()?);
                (x.is_finite() && z.is_finite()).then_some(Bucket {
                    x,
                    z,
                    depth: BucketDepth::from_index(depths.get(i).copied().unwrap_or(0)),
                    radius: if sizes.get(i) == Some(&2) {
                        crate::physics::STUNT_POOL_RADIUS
                    } else if sizes.get(i) == Some(&1) {
                        SPLASH_RADIUS
                    } else {
                        BUCKET_RADIUS
                    },

                    ..Bucket::default()
                })
            })
            .collect();
        if upgrade_oval {
            for b in &mut buckets {
                if b.radius == SPLASH_RADIUS {
                    *b = starter_bucket(&road);
                }
            }
        }
        if supply_bucket {
            buckets.insert(0, starter_bucket(&road));
            buckets.truncate(12);
        }
        let mut sim = World::new(&road, &buckets);
        let cars = fleet
            .into_iter()
            .map(|(k, p)| sim.spawn(k, p, &road))
            .collect();
        Self {
            studio: None,
            sim,
            launch_boost,
            pending_switch: None,
            buckets,
            edit: None,
            toddler: storage::get("toddler").as_deref() == Some(&[1]),
            camera_enabled: storage::get("camera").as_deref() == Some(&[1]),
            grab: None,
            road,
            kinds,
            anchors,
            starter_page: 0,
            starter_sheet: load_sheet("assets/starter-tracks.jpg"),
            track_sheet: load_sheet("assets/track-picker.png"),
            track_family: None,
            new_layout: false,
            cars,
            models: vec![],
            undo: vec![],
            preview: None,
            garage_sheet: load_sheet("assets/garage-models.png"),
            page: Page::Play,
            selected: 0,
            kind: 0,
            color: 0,
            paused: false,
            idle_camera: Default::default(),
            follow_camera: Default::default(),
            previous: vec![],
            frame_dt: 0.,
            portal_seen: vec![],
            follow_now: None,
            wake_until: -1.,
            collision_clip: clip("assets/collision.wav"),
            splash_clip: clip("assets/splash.wav"),
            bump_delay: 0.,
            rev_clip: clip("assets/rev.wav"),
            rev_delay: 0.,
            apart: false,
            spread: 0.,
            angle: 0.07,
            elevation: 0.4,
            azimuth: 0.44,
            zoom: 1.,
            cam: [0.; 3],
            target: [0.; 3],
            buttons: vec![],
            finger: None,
            touch_time: -2.,
            music: clip("meadow.opus"),
            voice: Handle::NONE,
            click: clip("click.wav"),
        }
    }
    fn fixed_update(&mut self, dt: f64) {
        self.previous.clear();
        self.previous.extend(self.cars.iter().map(CarFrame::of));
        if !self.paused && !self.inspecting() && self.page != Page::Starters {
            self.poll_switch();
            for _ in 0..2 {
                self.sim.step(
                    &mut self.cars,
                    &self.road,
                    &mut self.buckets,
                    dt as f32 / 2.,
                );
            }
            let sounds = std::mem::take(&mut self.sim.sounds);
            self.bump_delay = (self.bump_delay - dt as f32).max(0.);
            self.rev_delay = (self.rev_delay - dt as f32).max(0.);
            if sounds.rev > 0. && self.rev_delay == 0. && !self.rev_clip.get().is_none() {
                audio::play(self.rev_clip.get(), 0.04 * sounds.rev.sqrt(), false).release();
                self.rev_delay = 0.85;
            }
            if sounds.splash > 0. && !self.splash_clip.get().is_none() {
                audio::play(self.splash_clip.get(), 0.48 * sounds.splash.sqrt(), false).release();
            }
            if sounds.bump > 0. && self.bump_delay == 0. && !self.collision_clip.get().is_none() {
                audio::play(self.collision_clip.get(), 0.38 * sounds.bump.sqrt(), false).release();
                self.bump_delay = 0.09;
            }
            for (i, r) in self
                .road
                .iter_mut()
                .enumerate()
                .filter(|(_, r)| r.kind == 6)
            {
                if let Some((launch, _)) = self.sim.gate_state(i) {
                    self.kinds[i] = switch_code(self.kinds[i], launch);
                    r.launch = launch;
                }
            }
            if let Some(studio) = self.studio.as_mut() {
                studio.motors(&self.sim, dt as f32);
            }
        }
    }
    fn event(&mut self, e: &Event) {
        if matches!(
            e,
            Event::Touch { .. }
                | Event::PointerButton { .. }
                | Event::PointerMove { .. }
                | Event::PointerDelta { .. }
                | Event::Wheel { .. }
                | Event::Key { .. }
                | Event::Action { .. }
                | Event::Focus { .. }
                | Event::Overlay { .. }
        ) {
            let watching = self
                .idle_camera
                .subject(self.selected, self.cars.len())
                .is_some();
            let now = FrameInfo::now().time;
            if watching {
                self.wake_until = now + 0.35;
            }
            self.idle_camera.reset();
            if now <= self.wake_until
                && matches!(
                    e,
                    Event::Touch {
                        phase: TouchPhase::Start,
                        ..
                    } | Event::PointerButton { pressed: true, .. }
                )
            {
                self.wake_until = -1.;
                self.grab = None;
                self.finger = None;
                if matches!(e, Event::Touch { .. }) {
                    self.touch_time = now;
                }
                return;
            }
        }
        match *e {
            Event::Touch {
                primary: true,
                phase,
                x,
                y,
                ..
            } => {
                self.touch_time = FrameInfo::now().time;
                match phase {
                    TouchPhase::Start => self.pointer(x, y, 0),
                    TouchPhase::Move => self.pointer(x, y, 1),
                    TouchPhase::End => self.pointer(x, y, 2),
                    _ => self.finger = None,
                }
            }
            Event::PointerButton {
                button: 0,
                pressed,
                x,
                y,
                ..
            } if FrameInfo::now().time - self.touch_time > 0.6 => {
                self.pointer(x, y, if pressed { 0 } else { 2 })
            }
            Event::PointerMove { x, y } if FrameInfo::now().time - self.touch_time > 0.6 => {
                self.pointer(x, y, 1)
            }
            Event::Key {
                code: "Escape",
                pressed: true,
                ..
            } => self.page(Page::Play),
            _ => {}
        }
    }
    fn render(&mut self, interpolation: f64) {
        let Some(studio) = self.studio.as_ref() else {
            return;
        };
        if self.inspecting() {
            return;
        }
        let t = interpolation.clamp(0., 1.) as f32;
        let mut drawn = None;
        for (i, (c, m)) in self.cars.iter().zip(&self.models).enumerate() {
            let shown = match self.previous.get(i) {
                Some(before) => before.toward(c, t),
                None => CarFrame::of(c),
            };
            m.pose_at(shown.p, shown.rotation, shown.spin, false, 0., 0.);
            if self.follow_now.map(|(index, _)| index) == Some(i) {
                drawn = Some(shown);
            }
        }
        if let (Some((index, t)), Some(car)) = (self.follow_now, drawn) {
            let f = FrameInfo::now();
            let aspect = f.width as f32 / f.height.max(1) as f32;
            let p = car.p;
            self.follow_camera.update(
                &mut self.cam,
                &mut self.target,
                [p.x, p.y, p.z],
                self.sim.velocity(&self.cars[index]),
                t,
                self.cars[index].wet,
                aspect,
                self.frame_dt as f32,
            );
            studio
                .camera
                .set_position(self.cam[0], self.cam[1], self.cam[2]);
            studio.camera.call("lookAt", &format!("{:?}", self.target));
        }
    }
    fn update(&mut self, dt: f64) {
        let f = FrameInfo::now();
        if f.width == 0 || f.height == 0 {
            return;
        }
        if self.studio.is_none() {
            let mut s = Studio::new(f.level.min(9));
            s.road(&self.road, &self.buckets);
            s.buckets(&self.buckets);
            self.models = self.cars.iter().map(|c| s.car(c.model, c.paint)).collect();
            self.studio = Some(s)
        }
        self.idle_camera.advance(
            dt,
            self.page == Page::Play && !self.paused && self.finger.is_none(),
        );
        let follow = self.idle_camera.subject(self.selected, self.cars.len());
        let inspect = self.inspecting();
        let studio = self.studio.as_mut().unwrap();
        if studio.level != f.level {
            studio.change_level(f.level);
            studio.buckets(&self.buckets);
            studio.road(&self.road, &self.buckets);
            for m in self.models.drain(..) {
                m.discard()
            }
            self.models = self
                .cars
                .iter()
                .map(|c| studio.car(c.model, c.paint))
                .collect();
            if let Some(m) = self.preview.take() {
                m.discard();
                let c = &self.cars[self.selected];
                self.preview = Some(if self.page == Page::Garage {
                    studio.car(self.kind, self.color)
                } else {
                    studio.car(c.model, c.paint)
                })
            }
        }
        studio.water_motion(&self.buckets);
        let marker = if let Some(i) = self.edit {
            self.road[i].at(0.5)
        } else {
            self.road[self.kinds.len() - 1].at(1.)
        };
        studio
            .socket
            .set_position(marker.x, marker.y + 0.085, marker.z);
        studio
            .socket
            .set_visible(self.page == Page::Track && self.edit.is_some());
        studio.world.set_visible(!inspect);
        studio.stage.set_visible(inspect);
        for m in &self.models {
            m.root.set_visible(!inspect);
        }
        self.frame_dt = dt;
        self.follow_now = None;
        let aspect = f.width as f32 / f.height as f32;
        studio.camera.set_f64("aspect", aspect as f64);
        studio.camera.call("updateProjectionMatrix", "");
        if inspect {
            self.spread +=
                (if self.apart { 1. } else { 0. } - self.spread) * (dt as f32 * 6.).min(1.);
            if self.finger.is_none() {
                self.angle += dt as f32 * 0.10
            }
            self.preview.as_ref().unwrap().pose(
                &self.cars[self.selected],
                true,
                self.spread,
                self.angle,
            );
            let mut d = (4.8 / aspect.min(1.3)).max(4.15) + self.spread * (4.15 / aspect.min(1.0));
            self.target = [0., 0.68 + self.spread * 0.74, 0.];
            let mut shift = 0.;
            if self.page == Page::Garage {
                let s = f.scale.max(0.8);
                let header = f.safe_area[0] + 66. * s;
                let cards = f.height as f32 - f.safe_area[2] - 108. * s - garage_card_height(&f);
                let available = (cards - header).max(60. * s);
                let fov = 2. * 18.5_f32.to_radians().tan();
                d = d.max(1.95 * f.height as f32 / (fov * available));
                shift = ((header + cards) / (2. * f.height as f32) - 0.5) * fov * d;
            }
            // Move the camera and its target along camera-up to frame the car above the controls.
            self.target[0] -= shift * 0.63 * self.elevation.sin();
            self.target[1] += shift * self.elevation.cos();
            self.target[2] -= shift * 0.80 * self.elevation.sin();
            self.cam = [
                self.target[0] + d * 0.63 * self.elevation.cos(),
                self.target[1] + d * self.elevation.sin(),
                self.target[2] + d * 0.80 * self.elevation.cos(),
            ]
        } else if let Some(subject) = follow {
            // Moved in render, from the same interpolated pose the car is drawn at.
            self.follow_now = Some(subject);
        } else {
            self.follow_camera = Default::default();
            let azimuth = self.azimuth + if aspect < 0.85 { 0.92 } else { 0. };
            let center = studio.center;
            let d = self.zoom
                * (studio.radius * (if aspect < 0.85 { 1.36 } else { 2.0 }) / aspect.min(1.7)
                    + studio.radius * 0.56);
            self.target = [
                center[0],
                center[1]
                    - if matches!(self.page, Page::Track | Page::Place(..)) {
                        2.2
                    } else {
                        0.2
                    },
                center[2],
            ];
            self.cam = [
                center[0] + azimuth.sin() * d,
                self.target[1] + d * 0.76,
                center[2] + azimuth.cos() * d,
            ]
        }
        studio
            .camera
            .set_position(self.cam[0], self.cam[1], self.cam[2]);
        studio.camera.call("lookAt", &format!("{:?}", self.target));
        studio.sky_frame(dt);
        self.portals();
        if follow.is_some() {
            self.buttons.clear();
            return;
        }
        if self.grab.is_some() {
            if let Some((sx, sy, x, y, true)) = self.finger {
                d(
                    "line",
                    format!("[{sx},{sy},{x},{y},{RED},{}]", 4. * f.scale),
                );
                d("circle", format!("[{x},{y},{},{RED}]", 7. * f.scale));
            }
        }
        self.callouts(&f);
        self.hud(&f)
    }
}
export_game!(Toybox);

/// A car's drawn pose at one fixed step.
#[derive(Clone, Copy)]
struct CarFrame {
    p: crate::physics::Pose,
    rotation: [f32; 4],
    spin: [f32; 4],
}
impl CarFrame {
    fn of(c: &Car) -> Self {
        Self { p: c.p, rotation: c.rotation, spin: c.spin }
    }
    /// `c` drawn `t` of the way from this frame to its current pose. A jump of more than
    /// half a metre (a teleport back to the start) is not blended.
    fn toward(&self, c: &Car, t: f32) -> Self {
        let mut out = Self::of(c);
        let (a, b) = (self.p, c.p);
        if (b.x - a.x).powi(2) + (b.y - a.y).powi(2) + (b.z - a.z).powi(2) > 0.25 {
            return out;
        }
        let mix = |x: f32, y: f32| x + (y - x) * t;
        out.p.x = mix(a.x, b.x);
        out.p.y = mix(a.y, b.y);
        out.p.z = mix(a.z, b.z);
        let q0 = self.rotation;
        let sign = if (0..4).map(|i| q0[i] * c.rotation[i]).sum::<f32>() < 0. { -1. } else { 1. };
        let mut q = [0.; 4];
        for i in 0..4 {
            q[i] = mix(q0[i], c.rotation[i] * sign);
        }
        let n = q.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
        out.rotation = q.map(|v| v / n);
        for i in 0..4 {
            if (c.spin[i] - self.spin[i]).abs() < std::f32::consts::PI {
                out.spin[i] = mix(self.spin[i], c.spin[i]);
            }
        }
        out
    }
}
