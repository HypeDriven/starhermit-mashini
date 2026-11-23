use starhermit_guest::{json::Json, module_call};

const POINTS: [(f32, f32); 7] = [(3., 2.), (3., 25.), (9., 19.), (14., 29.), (18., 27.), (13., 17.), (22., 17.)];

fn inside(x: f32, y: f32) -> bool {
    let mut hit = false;
    for (i, &(ax, ay)) in POINTS.iter().enumerate() {
        let (bx, by) = POINTS[(i + 1) % POINTS.len()];
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax { hit = !hit; }
    }
    hit
}

fn edge(x: f32, y: f32) -> bool {
    POINTS.iter().enumerate().any(|(i, &(ax, ay))| {
        let (bx, by) = POINTS[(i + 1) % POINTS.len()];
        let dx = bx - ax;
        let dy = by - ay;
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0., 1.);
        (x - ax - t * dx).hypot(y - ay - t * dy) < 1.7
    })
}

pub fn show(accent: u32, outline: u32) {
    let mut pixels = [0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let fill = inside(x as f32 + 0.5, y as f32 + 0.5);
            if !fill && !edge(x as f32 + 0.5, y as f32 + 0.5) { continue; }
            let color = if fill { accent } else { outline };
            let at = (y * 32 + x) * 4;
            pixels[at] = (color >> 16) as u8;
            pixels[at + 1] = (color >> 8) as u8;
            pixels[at + 2] = color as u8;
            pixels[at + 3] = 255;
        }
    }
    let args = Json::args()
        .object()
        .key("image").object()
            .key("width").num(32.)
            .key("height").num(32.)
            .key("data").u8s(&pixels)
        .end()
        .key("hotspot").array().num(3.).num(2.).end()
        .key("fallback").str("default")
        .end()
        .finish();
    module_call("starhermit:input", "setCursor", &args);
}
