use mashini::physics::{Bucket, CLEATS, World, build, cleat};
const DT: f32 = 1. / 120.;
#[test]
fn ledge_preserves_momentum_tips_and_recovers() {
    let road = build(&[0]);
    let mut world = World::new(&road, &[]);
    let mut cars = [world.spawn(0, 0, &road)];
    world.flick(&cars[0], [5., 0., 0.]);
    let mut tipped = false;
    let mut landed = false;
    for _ in 0..7200 {
        world.step(&mut cars, &road, &mut [], DT);
        let c = &cars[0];
        if c.p.x > 0. && c.p.y < 2.8 {
            tipped |= c.rotation[2].abs() > 0.08;
            landed |= c.rest > 0.1
        }
        if landed && c.p.y > 3.3 {
            assert!(tipped);
            return;
        }
    }
    panic!(
        "no recovery; x={} y={} rest={} tipped={tipped}",
        cars[0].p.x, cars[0].p.y, cars[0].rest
    )
}
#[test]
fn launch_speed_depends_on_roller_setting() {
    let run = |code| {
        let road = build(&[code, 0, 0]);
        let mut world = World::new(&road, &[]);
        let mut cars = [world.spawn(0, 0, &road)];
        let mut fastest = 0f32;
        for _ in 0..400 {
            let x = cars[0].p.x;
            world.step(&mut cars, &road, &mut [], DT);
            fastest = fastest.max((cars[0].p.x - x) / DT)
        }
        fastest
    };
    let low = run(6);
    let high = run(11);
    assert!(high > low + 2., "{high} vs {low}");
}
#[test]
fn lift_returns_underneath_and_carries_a_car() {
    let road = build(&[4, 0]);
    let r = road[0];
    let path: Vec<_> = (0..300)
        .map(|i| cleat(&r, CLEATS / 2, i as f32 * DT))
        .collect();
    assert!(path.windows(2).any(|v| v[1].x < v[0].x - 0.01));
    let mut world = World::new(&road, &[]);
    let mut cars = [world.spawn(0, 0, &road)];
    let mut summit = 0f32;
    for _ in 0..450 {
        world.step(&mut cars, &road, &mut [], DT);
        summit = summit.max(cars[0].p.y)
    }
    assert!(summit > 6.25, "summit {summit}");
}
#[test]
fn water_detects_entry() {
    let road = build(&[0]);
    let mut buckets = [Bucket {
        radius: mashini::physics::BUCKET_RADIUS,
        x: 3.,
        z: -4.,

        ..Bucket::default()
    }];
    let mut w = World::new(&road, &buckets);
    let mut cars = [w.spawn(0, 0, &road)];
    w.flick(&cars[0], [5., 0., 0.]);
    let mut splash = false;
    for _ in 0..360 {
        w.step(&mut cars, &road, &mut buckets, DT);
        splash |= !buckets[0].impacts.is_empty()
    }
    assert!(splash);
}

#[test]
fn curve_deflects_a_rolling_car() {
    let road = build(&[0, 2, 0]);
    let mut w = World::new(&road, &[]);
    let mut cars = [w.spawn(0, 0, &road)];
    w.flick(&cars[0], [6.2, 0., 0.]);
    let mut turned = false;
    for _ in 0..740 {
        w.step(&mut cars, &road, &mut [], DT);
        let p = cars[0].p;
        turned |= p.x > 2. && p.z > 0. && p.y > 3.0;
    }
    assert!(turned, "end {:?}", cars[0].p);
}
#[test]
fn flick_changes_velocity_without_snapping_to_the_track() {
    let road = build(&[0]);
    let mut w = World::new(&road, &[]);
    let mut cars = [w.spawn(0, 0, &road)];
    w.flick(&cars[0], [0., 13., 9.]);
    let z = cars[0].p.z;
    for _ in 0..20 {
        w.step(&mut cars, &road, &mut [], DT);
    }
    assert!(cars[0].p.z > z + 1.0);
}
#[test]
fn automatic_fork_routes_all_models_around_loop_then_into_bucket() {
    use mashini::physics::{SOAK_TIME, START, starter_bucket};
    let road = build(&START);
    let gate = road.iter().position(|r| r.kind == 6).unwrap();
    assert!(road.iter().any(|r| r.kind == 8));
    for model in 0..3 {
        let mut buckets = [starter_bucket(&road)];
        let mut world = World::new(&road, &buckets);
        let mut cars = [world.spawn(model, 0, &road)];
        let mut looped = false;
        let mut armed_launch = false;
        let mut reached_ramp = false;
        let mut soaked = 0.0_f32;
        let mut returned = false;
        for _ in 0..4500 {
            world.step(&mut cars, &road, &mut buckets, DT);
            let c = &cars[0];
            let (splash, angle) = world.gate_state(gate).unwrap();
            armed_launch |= splash && angle < -0.3;
            looped |= armed_launch && c.p.x < -8.;
            reached_ramp |= looped && !splash && c.p.x > 15. && c.p.y > 2.5;
            soaked = soaked.max(c.soaked);
            if c.portal > 0. {
                returned = c.departure.x > 15. && c.arrival.x < 0.;
                break;
            }
        }
        assert!(
            reached_ramp && returned && soaked > SOAK_TIME - 2. * DT,
            "model {model}: loop={looped} jump={reached_ramp} soak={soaked} at {:?}",
            cars[0].p
        );
    }
}
#[test]
fn level_track_requires_a_push_and_coasts_to_rest() {
    let road = build(&[0; 10]);
    let mut w = World::new(&road, &[]);
    let mut cars = [w.spawn(0, 0, &road)];
    let start = cars[0].p.x;
    for _ in 0..360 {
        w.step(&mut cars, &road, &mut [], DT);
    }
    assert!((cars[0].p.x - start).abs() < 0.03);
    assert_eq!(w.sounds.rev, 0.);
    w.flick(&cars[0], [3., 0., 0.]);
    w.step(&mut cars, &road, &mut [], DT);
    assert!(std::mem::take(&mut w.sounds).rev > 0.);
    let mut distances = Vec::new();
    for _ in 0..15 {
        let x = cars[0].p.x;
        for _ in 0..120 {
            w.step(&mut cars, &road, &mut [], DT);
        }
        distances.push((cars[0].p.x - x).abs());
    }
    assert_eq!(w.sounds.rev, 0., "a coasting car should stay quiet");
    assert!(
        distances[0] > 1.
            && distances[3] < distances[0]
            && distances[12..].iter().all(|d| *d < 0.005),
        "{distances:?}"
    );
}

#[test]
fn drag_nudge_preserves_direction_and_limits_fast_swipes() {
    use mashini::physics::nudge;
    assert_eq!(nudge([0.; 3]), [0.; 3]);
    let gentle = nudge([1., 0., 0.])[0];
    let strong = nudge([10., 0., 0.])[0];
    assert!(gentle > 0. && strong > gentle && strong < gentle * 6.);
    let quick = nudge([-300., 0., 400.]);
    assert!(quick[0].hypot(quick[2]) <= 3.21 && (quick[2] / quick[0] + 4. / 3.).abs() < 0.001);
}

#[test]
fn rear_car_transfers_momentum_to_front_car() {
    let road = build(&[0, 0, 0]);
    let mut w = World::new(&road, &[]);
    let mut cars = [w.spawn(0, 0, &road), w.spawn(1, 1, &road)];
    assert!(cars[1].p.x - cars[0].p.x > 2.6);
    w.flick(&cars[0], [10., 0., 0.]);
    let mut top_speed = 0.0_f32;
    for _ in 0..30 {
        let before = cars[1].p.x;
        w.step(&mut cars, &road, &mut [], DT);
        top_speed = top_speed.max((cars[1].p.x - before) / DT);
    }
    assert!(top_speed > 2., "front car reached {top_speed}");
}

#[test]
fn removing_a_car_touching_a_booster_keeps_the_next_step_valid() {
    let road = build(&[5, 0]);
    let mut world = World::new(&road, &[]);
    let mut cars = vec![world.spawn(0, 0, &road)];
    for _ in 0..5 {
        world.step(&mut cars, &road, &mut [], DT);
    }
    world.remove(&cars.remove(0));
    world.step(&mut cars, &road, &mut [], DT);
}

#[test]
fn two_cars_can_land_in_one_bucket_without_restarting_the_first_splash() {
    let road = build(&[0]);
    let mut buckets = [Bucket {
        x: 4.,
        z: -4.,
        radius: 3.85,
        ..Default::default()
    }];
    let mut world = World::new(&road, &buckets);
    let mut cars = [world.spawn(0, 0, &road), world.spawn(1, 1, &road)];
    for car in &cars {
        world.flick(car, [5., 0., 0.]);
    }
    let mut first = None;
    let mut splashes = 0;
    for _ in 0..500 {
        world.step(&mut cars, &road, &mut buckets, DT);
        if std::mem::take(&mut world.sounds).splash > 0. {
            splashes += 1;
        }
        let waves = &buckets[0].impacts;
        if let Some(wave) = waves.first() {
            first.get_or_insert(wave.id);
        }
        if waves.len() == 2 {
            assert_eq!(Some(waves[0].id), first);
            assert!(waves[0].age > waves[1].age && splashes == 2);
            return;
        }
    }
    panic!("Expected overlapping car entries, heard {splashes}");
}

#[test]
fn track_picture_orientations_place_at_the_tap_and_join_at_matching_height() {
    use mashini::physics::{Pose, build_at, place_track};
    use std::f32::consts::FRAC_PI_2;
    for code in 0..=5 {
        for dir in 0..4 {
            let a = dir as f32 * FRAC_PI_2;
            let pose = place_track(code, a, 15., -9., &[]);
            let road = build_at(&[code], &[Some(pose)]);
            let mid = road[0].at(0.5);
            assert!((mid.x - 15.).hypot(mid.z + 9.) < 0.001);
            assert_eq!(road[0].p.a, a);
        }
    }
    let start = Pose {
        x: 1.,
        y: 2.,
        z: 3.,
        a: FRAC_PI_2,
    };
    let road = build_at(&[0], &[Some(start)]);
    let end = road[0].at(1.);
    let added = place_track(2, end.a, end.x, end.z, &road);
    assert!((added.x - end.x).hypot(added.z - end.z) < 0.001);
    assert_eq!(added.y, end.y);
}

#[test]
fn stationary_car_respawns_after_a_full_minute_since_its_last_move() {
    let road = build(&[0; 6]);
    let mut world = World::new(&road, &[]);
    let mut cars = [world.spawn(2, 3, &road)];
    for _ in 0..55 * 120 {
        world.step(&mut cars, &road, &mut [], DT);
        assert_eq!(cars[0].portal, 0.);
    }
    world.flick(&cars[0], [2., 0., 0.]);
    for _ in 0..60 * 120 {
        world.step(&mut cars, &road, &mut [], DT);
        assert_eq!(cars[0].portal, 0.);
    }
    for _ in 0..10 * 120 {
        world.step(&mut cars, &road, &mut [], DT);
        if cars[0].portal > 0. {
            assert_eq!((cars[0].model, cars[0].paint), (2, 3));
            assert!((cars[0].departure.x - cars[0].arrival.x).abs() > 1.);
            assert!((cars[0].arrival.x - road[0].at(0.19).x).abs() < 0.01);
            return;
        }
    }
    panic!("stopped car was never returned to the start");
}

#[test]
fn starter_racing_circuits_drive_two_laps() {
    use mashini::physics::STARTERS;
    for &(name, codes) in &STARTERS[..3] {
        if codes.contains(&mashini::physics::LOOP) {
            continue;
        }
        let road = build(codes);
        let finish = road.last().unwrap().at(1.);
        assert!((finish.x - road[0].p.x).hypot(finish.z - road[0].p.z) < 0.001);
        for model in 0..3 {
            let mut w = World::new(&road, &[]);
            let mut cars = [w.spawn(model, 0, &road)];
            let mut target = 1;
            for _ in 0..120 * 90 {
                w.step(&mut cars, &road, &mut [], DT);
                let car = &cars[0];
                assert_eq!(car.portal, 0., "{name} recovered near {:?}", car.departure);
                let center = road[target % road.len()].at(0.5);
                if (car.p.x - center.x).hypot(car.p.z - center.z) < 1.8 {
                    target += 1;
                }
                if target > 2 * road.len() {
                    break;
                }
            }
            assert!(
                target > 2 * road.len(),
                "{name} stopped before checkpoint {target} at {:?}",
                cars[0].p
            );
        }
    }
}

#[test]
fn stunt_courses_grow_and_all_three_cars_splash_after_the_final_drop() {
    use mashini::physics::{STARTERS, WATER, build_at, starter_anchors, starter_buckets};
    let mut last_length = build(STARTERS[2].1).iter().map(|s| s.len).sum::<f32>();
    let mut last_drop = 0.;
    for (i, &(name, codes)) in STARTERS.iter().enumerate().skip(3) {
        let road = build_at(codes, &starter_anchors(i));
        let main = &road[..codes.len()];
        let length = main.iter().map(|s| s.len).sum::<f32>();
        let lip = main.last().unwrap().at(1.);
        let drop = lip.y - WATER;
        let approach = &main[..main.len() - 1];
        assert!(
            approach.iter().any(|r| r.rise >= 1.) && approach.iter().any(|r| r.rise <= -1.),
            "{name} needs an uphill and downhill section before the lip"
        );
        assert!(
            length > last_length && drop + 0.001 >= last_drop && drop >= 4.,
            "{name}: {length}m long, drop {drop} after {last_drop}"
        );
        (last_length, last_drop) = (length, drop);
        for model in 0..3 {
            let mut pools: Vec<_> = starter_buckets(i, &road).collect();
            if name == "Waterfall Bend" {
                assert_eq!(pools[0].depth, mashini::physics::BucketDepth::ExtraDeep);
            }
            let mut world = World::new(&road, &pools);
            let mut cars = [world.spawn(model, 0, &road)];
            if name == "Cloud Cascade" {
                assert_eq!(world.velocity(&cars[0]), [0.; 3]);
                assert!(road.iter().all(|s| !matches!(s.kind, 4 | 5)));
            }
            let mut launch_speed = 0.0_f32;
            let mut launched = false;
            for _ in 0..120 * 80 {
                world.step(&mut cars, &road, &mut pools, DT);
                let p = cars[0].p;
                let distance = (p.x - lip.x) * lip.a.cos() + (p.z - lip.z) * lip.a.sin();
                if distance.abs() < 0.6
                    && (p.y - lip.y).abs() < 1.2
                    && (p.x - lip.x).hypot(p.z - lip.z) < 1.8
                {
                    let v = world.velocity(&cars[0]);
                    launch_speed = launch_speed.max(v[0].hypot(v[2]));
                }
                launched |= distance > 2. && p.y > WATER + 1.5;
                if cars[0].wet || cars[0].portal > 0. {
                    break;
                }
            }
            if name == "High Dive" {
                assert!(launch_speed > 14., "{name} {model}: {launch_speed}");
            }
            assert!(
                launched && cars[0].wet && pools[0].serial > 0,
                "{name}, car {model}: final position {:?}, pool {:?}, launched {launched}",
                cars[0].p,
                (pools[0].x, pools[0].z)
            );
        }
    }
}

#[test]
fn standalone_switch_sends_the_car_down_the_selected_exit() {
    use mashini::physics::{FORK_LEFT, FORK_RIGHT, START};
    for code in [FORK_LEFT, FORK_RIGHT] {
        let mut kinds = START;
        kinds[3] = code;
        let road = build(&kinds);
        assert_eq!(road.len(), kinds.len() + 1);
        let exits = [road[3].at(1.), road.last().unwrap().at(1.)];
        let mut world = World::new(&road, &[]);
        let mut cars = [world.spawn(0, 0, &road)];
        let mut reached = None;
        for _ in 0..2400 {
            world.step(&mut cars, &road, &mut [], DT);
            let p = cars[0].p;
            reached = exits
                .iter()
                .position(|e| (p.x - e.x).hypot(p.z - e.z) < 1.2);
            if reached.is_some() {
                break;
            }
        }
        assert_eq!(reached, Some(usize::from(code == FORK_LEFT)));
    }
}

#[test]
fn both_comet_run_splitters_send_each_car_to_the_matching_pool() {
    use mashini::physics::{LAUNCH, LOOP, STARTERS, build_at, starter_anchors, starter_buckets};
    let codes = STARTERS[7].1;
    let switches: Vec<_> = codes
        .iter()
        .enumerate()
        .filter(|(_, k)| **k == LOOP)
        .collect();
    assert_eq!(switches.len(), 2);
    for (branch, (owner, _)) in switches.into_iter().enumerate() {
        let mut route = codes.to_vec();
        route[owner] = LAUNCH;
        let road = build_at(&route, &starter_anchors(7));
        for model in 0..3 {
            let mut pools: Vec<_> = starter_buckets(7, &road).collect();
            assert_eq!(pools.len(), 3);
            let mut world = World::new(&road, &pools);
            let mut cars = [world.spawn(model, 0, &road)];
            for _ in 0..120 * 72 {
                world.step(&mut cars, &road, &mut pools, DT);
                if cars[0].wet || cars[0].portal > 0. {
                    break;
                }
            }
            assert!(
                pools[branch + 1].serial > 0,
                "Comet Run split {}, car {model}",
                branch + 1
            );
        }
    }
}
