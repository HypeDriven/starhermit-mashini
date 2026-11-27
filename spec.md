# Mashini — running spec

Mashini is a die-cast toy car sandbox on a maple tabletop, built in Rust for the StarHermit engine
(1.53.0, WebGPU in browsers). Published on StarHermit as uploaded game `8d693360-e3f1-48f3-91e8-638834104c34`.

## Play

- The tabletop starts with a starter circuit and one red Sprint GT. Cars start at rest; red roller
  boosters (drums, eight speeds from 2 to 40 m/s) and conveyor lifts keep them moving.
- Tapping a car opens its model workshop: drag to turn it, **Take apart** spreads the body shell,
  glazing, seats and wheel assemblies with labelled callouts. During play, dragging a car nudges it,
  with a soft limit on fast swipes.
- **Garage**: pick a model, then a paint, then **Go** to place it at the start. A fleet holds up to 100
  cars; adding to a full fleet replaces the oldest.
- **Tracks**: pick a piece and its facing, tap the tabletop to place it. Matching connectors join at
  the same height. Tapping a placed piece replaces or removes it; boosters take **− Speed / + Speed**.
  The green switching junction has a brass lever that alternates routes after each car, or on tap.
- **Water bucket**: three glass bucket depths. Cars sink with bubbles; water resistance slows each
  submerged wheel.
- Cars teleport back to the start after three seconds in water, after settling off the road, or after
  60 s of simulation standing still. Departure and arrival are marked by amber and teal particle
  bursts that the scene can hide.
- **Options → Reset track** opens a grid of nine pictured courses: Pocket Oval, Splash Jump,
  Switchback Run, then the stunt courses Ribbon Leap, Waterfall Bend, Canyon Hop, High Dive,
  Comet Run and Cloud Cascade, which grow from 17 to 32 pieces and end in long drops into water.
  Choosing one starts it with one red Sprint GT; Back keeps the current layout and cars.
- **Toddler** (Options) hides the Tracks and Garage buttons; remembered between sessions. Layout and
  fleet are saved locally.
- After 15 s idle the camera follows a car with framing chosen by its motion (fast, falling, sinking)
  and hides the controls; any tap brings them back. Manual orbit and zoom can be enabled in Options.

## Simulation

- Rapier at 120 Hz (two substeps per 60 Hz fixed step). Each car is a compound body (chassis box, the
  tuned cabin block and a massless roof cap matching each model's roof and rails) with four wheels on
  axle joints. Cars are drawn interpolated between fixed steps using the engine's render
  interpolation, and the follow camera tracks the same interpolated pose.
- Starter courses are tuned so every model completes them; `tests/vehicle.rs` checks laps, splashes,
  fork routing and launch speeds for all three models.

## Rendering

- Quality follows the engine's graphics level: four car mesh sets, tiered road tessellation, sky
  (SkyAtmosphere) and VolumetricClouds, and cascaded sun shadows that follow the view.
- Bucket water uses the engine's `WaterMaterial` (refraction through scene colour/depth, absorption,
  Fresnel reflection) where screen effects are available, over a baked Alembic wave surface with
  impact-placed waves; lower levels use a translucent surface.
- Tone mapping is Khronos PBR Neutral at exposure 0.92 with a light S-curve grade and vignette.

## Platform

- `starhermit.txt` declares name, description, owner, cover, banner, tags and release notes;
  `favicon.png` is the page icon. Build with `sh tools/build.sh`; publish with
  `tools/upload-game.mjs mashini` from the games folder.

## Tests

- `cargo test --release --offline`: simulation and route tests.
- Engine render checks with play scripts: `starhermit . --render-check --guest --script tests/<name>.play`
  for `workshop`, `route`, `camera`, `starters` and `toddler`.

## Known gaps

- Shadows at the bucket floor are not verified on a GPU (headless checks run on a software renderer).
- No game-side Sound, Language or frame-rate options beyond what the engine's own Options provides.

Browser rendering uses the engine's asynchronous pipeline creation so newly encountered materials do not stall the page. Native play retains the engine's synchronous pipeline behavior.

On engine 1.53.0, browser play through WebGL 2 compiles a frame's shader programs together and holds that frame until they are linked, blends skinned meshes in one pass, reuses the window pass's depth, and uses a hardware WebGL 2 context when WebGPU's adapter is software.
