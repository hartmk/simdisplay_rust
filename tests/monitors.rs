use eframe::egui::{Rect, pos2, vec2};
use simdisplay::monitors::{Monitor, Placement};
fn monitor(id: u32, x: f32, width: f32) -> Monitor {
    Monitor {
        id,
        label: id.to_string(),
        x,
        y: 0.,
        width,
        height: 1080.,
        scale: 1.,
        primary: id == 1,
    }
}
#[test]
fn restore_preserves_desktop_geometry_across_dpi() {
    let p = Placement::capture(
        Rect::from_min_size(pos2(100., 80.), vec2(1000., 700.)),
        vec2(1000., 680.),
        1.,
    );
    let (pos, size) = p.commands(&[monitor(1, 0., 1920.)], 2.);
    assert_eq!(pos, pos2(50., 40.));
    assert_eq!(size, vec2(500., 340.));
    let p = Placement::capture(
        Rect::from_min_size(pos2(50., 40.), vec2(500., 350.)),
        vec2(500., 340.),
        2.,
    );
    assert_eq!(
        p.commands(&[monitor(1, 0., 1920.)], 1.),
        (pos2(100., 80.), vec2(1000., 680.))
    );
}
#[test]
fn negative_coordinates_and_removed_origin_monitor() {
    let p = Placement::capture(
        Rect::from_min_size(pos2(-1600., 50.), vec2(1000., 700.)),
        vec2(1000., 680.),
        1.,
    );
    let primary = monitor(1, 0., 1920.);
    let left = monitor(2, -1920., 1920.);
    assert_eq!(
        p.commands(&[primary.clone(), left], 1.).0,
        pos2(-1600., 50.)
    );
    let (pos, size) = p.commands(&[primary], 1.);
    assert!(pos.x >= 0. && pos.y >= 0.);
    assert!(pos.x + size.x <= 1920.);
}
