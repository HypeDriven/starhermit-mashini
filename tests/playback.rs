use mashini::{
    camera::IdleCamera,
    physics::{Bucket, WAVE_DURATION},
    water::Waves,
};

#[test]
fn later_impacts_add_to_existing_alembic_motion_at_every_tier() {
    for data in [
        include_bytes!("../assets/bucket-0.wabc").as_slice(),
        include_bytes!("../assets/bucket-1.wabc").as_slice(),
        include_bytes!("../assets/bucket-2.wabc").as_slice(),
    ] {
        let mut waves = Waves::open(data.to_vec()).unwrap();
        let mut bucket = Bucket {
            radius: 2.,
            ..Bucket::default()
        };
        bucket.strike(0.7, 0.1, 5.);
        bucket.advance(0.33);
        let mut first = vec![];
        waves.compose(&bucket.impacts, &mut first).unwrap();
        bucket.strike(-0.6, -0.25, 4.);
        let mut mixed = vec![];
        waves.compose(&bucket.impacts, &mut mixed).unwrap();
        assert_eq!(
            mixed, first,
            "starting the second impact changed an existing wave"
        );
        bucket.advance(0.17);
        let mut second = vec![];
        waves.compose(&bucket.impacts[..1], &mut first).unwrap();
        waves.compose(&bucket.impacts[1..], &mut second).unwrap();
        waves.compose(&bucket.impacts, &mut mixed).unwrap();
        assert!(first.iter().skip(1).step_by(3).any(|y| y.abs() > 0.03));
        assert!(second.iter().skip(1).step_by(3).any(|y| y.abs() > 0.03));
        for i in (1..mixed.len()).step_by(3) {
            assert!((mixed[i] - first[i] - second[i]).abs() < 1e-5);
        }
        bucket.advance(WAVE_DURATION - 0.49);
        assert_eq!(bucket.impacts.len(), 1);
        assert_eq!(bucket.impacts[0].id, 2);
    }
}

#[test]
fn idle_camera_waits_rotates_subjects_and_resets_on_input() {
    let mut camera = IdleCamera::default();
    camera.advance(14.99, true);
    assert!(camera.subject(1, 3).is_none());
    camera.advance(0.02, true);
    assert_eq!(camera.subject(1, 3).unwrap().0, 1);
    camera.advance(15., true);
    assert_eq!(camera.subject(1, 3).unwrap().0, 2);
    camera.advance(15., true);
    assert_eq!(camera.subject(1, 3).unwrap().0, 0);
    camera.reset();
    assert!(camera.subject(1, 3).is_none());
    camera.advance(30., false);
    assert!(camera.subject(1, 3).is_none());
}

#[test]
fn follow_camera_eases_into_speed_and_keeps_room_for_a_fall() {
    use mashini::camera::FollowCamera;
    let pose = [0., 10., 0.];
    let (mut eye, mut focus) = ([0., 14., 7.], pose);
    let mut rig = FollowCamera::default();
    for _ in 0..180 {
        rig.update(&mut eye, &mut focus, pose, [0.; 3], 0., false, 1., 1. / 60.);
    }
    let still = eye;
    rig.update(
        &mut eye,
        &mut focus,
        pose,
        [12., -9., 0.],
        0.,
        false,
        1.,
        1. / 60.,
    );
    assert!((eye[1] - still[1]).abs() < 0.05);
    for _ in 0..180 {
        rig.update(
            &mut eye,
            &mut focus,
            pose,
            [12., -9., 0.],
            0.,
            false,
            1.,
            1. / 60.,
        );
    }
    assert!(eye[1] > still[1] + 3. && eye[2] > still[2] + 4.);
    assert!(focus[0] > 2. && focus[1] < pose[1]);
}

#[test]
fn starter_grid_paginates_only_when_the_cards_do_not_fit() {
    use mashini::starter_grid::StarterGrid;
    for (width, height) in [(942., 468.), (482., 765.), (748., 946.)] {
        let grid = StarterGrid::fit(width, height, 9);
        assert_eq!(grid.pages, 1);
        assert!(grid.cols > 1 && grid.rows * grid.cols >= 9);
    }
    let grid = StarterGrid::fit(300., 385., 9);
    assert_eq!(grid.per_page, 6);
    assert_eq!(grid.pages, 2);
}
