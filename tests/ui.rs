use eframe::egui::{self, Event, PointerButton, Pos2, Rect, Vec2};
use simdisplay::app::DisplayApp;
fn frame(app: &mut DisplayApp, ctx: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440., 900.))),
            events,
            ..Default::default()
        },
        |ctx| app.show(ctx),
    )
}
fn text_pos(output: &egui::FullOutput, text: &str) -> Pos2 {
    output
        .shapes
        .iter()
        .find_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                if t.galley.text() == text {
                    return Some(t.pos + t.galley.size() / 2.);
                }
                None
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing UI text {text}"))
}
fn pointer(p: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}
#[test]
fn search_reaches_output_fields_from_common_category_and_adds_one() {
    let t = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, t.path().join("state.json"));
    frame(&mut app, &ctx, vec![]);
    let output = frame(&mut app, &ctx, vec![]);
    let search = text_pos(&output, "ค้นหาวิดเจ็ตทุกหมวด");
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(search), pointer(search, true)],
    );
    frame(&mut app, &ctx, vec![pointer(search, false)]);
    frame(
        &mut app,
        &ctx,
        vec![Event::Text("front left wing damage".into())],
    );
    let output = frame(&mut app, &ctx, vec![]);
    let button = text_pos(&output, "Damage · Front Left Wing Damage");
    let before = app.editor.layout.widgets.len();
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(button), pointer(button, true)],
    );
    frame(&mut app, &ctx, vec![pointer(button, false)]);
    assert_eq!(app.editor.layout.widgets.len(), before + 1);
    assert_eq!(
        app.editor.selected_mut().unwrap().kind,
        "f1.carDamage.carDamageData.frontLeftWingDamage"
    );
}
#[test]
fn clicking_library_adds_widget_and_drag_accumulates_across_frames() {
    let t = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, t.path().join("state.json"));
    frame(&mut app, &ctx, vec![]);
    // The growing catalog scrolls; use the same interaction as a user to reach its end.
    frame(
        &mut app,
        &ctx,
        vec![
            Event::PointerMoved(Pos2::new(90., 400.)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0., -700.),
                modifiers: Default::default(),
            },
        ],
    );
    // Allow smooth scrolling to settle as the widget catalog grows.
    for _ in 0..30 {
        frame(&mut app, &ctx, vec![]);
    }
    let o = frame(&mut app, &ctx, vec![]);
    let add = text_pos(&o, "Custom text");
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(add), pointer(add, true)],
    );
    frame(&mut app, &ctx, vec![pointer(add, false)]);
    assert_eq!(app.editor.layout.widgets.len(), 11);
    // Choose a clear title on the topmost newly-added widget.
    app.editor.layout.widgets.retain(|w| w.kind == "text");
    let o = frame(&mut app, &ctx, vec![]);
    let title = text_pos(&o, "Custom text");
    // Find the canvas title (the library has the same label); last occurrence belongs to canvas.
    let title = o
        .shapes
        .iter()
        .filter_map(|s| {
            if let egui::Shape::Text(t) = &s.shape {
                (t.galley.text() == "Custom text" && t.pos.x > 185. && t.pos.x < 1150.)
                    .then_some(t.pos + t.galley.size() / 2.)
            } else {
                None
            }
        })
        .min_by(|a, b| a.y.total_cmp(&b.y))
        .unwrap_or(title);
    let before = app.editor.layout.widgets[0].x;
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(title), pointer(title, true)],
    );
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(title + Vec2::new(20., 0.))],
    );
    for dx in [30., 40., 50., 60., 70., 80.] {
        frame(
            &mut app,
            &ctx,
            vec![Event::PointerMoved(title + Vec2::new(dx, 0.))],
        );
        frame(&mut app, &ctx, vec![]);
    }
    frame(
        &mut app,
        &ctx,
        vec![pointer(title + Vec2::new(80., 0.), false)],
    );
    let moved = app.editor.layout.widgets[0].x - before;
    assert!(
        moved > 150.,
        "80 screen pixels should move over 150 canvas pixels; moved {moved}"
    );
    app.editor.undo();
    assert_eq!(app.editor.layout.widgets[0].x, before);
}
#[test]
fn opacity_slider_drag_is_a_single_undo_step() {
    let t = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, t.path().join("state.json"));
    app.editor.add("speed");
    let before = app.editor.layout.clone();
    frame(&mut app, &ctx, vec![]);
    let o = frame(&mut app, &ctx, vec![]);
    let label = text_pos(&o, "Opacity %");
    let p = Pos2::new(1220., label.y);
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(p), pointer(p, true)],
    );
    for x in [1230., 1240., 1250., 1260.] {
        frame(&mut app, &ctx, vec![Event::PointerMoved(Pos2::new(x, p.y))]);
        frame(&mut app, &ctx, vec![]);
    }
    frame(&mut app, &ctx, vec![pointer(Pos2::new(1260., p.y), false)]);
    assert_ne!(app.editor.layout, before);
    app.editor.undo();
    assert_eq!(app.editor.layout, before);
}
#[cfg(unix)]
fn pump(app: &mut DisplayApp, ctx: &egui::Context, millis: u64) {
    let until = std::time::Instant::now() + std::time::Duration::from_millis(millis);
    while std::time::Instant::now() < until {
        frame(app, ctx, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
}
#[cfg(unix)]
#[test]
fn failed_autosave_retries_unchanged_workspace() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("state.json");
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, path.clone());
    app.editor.add("text");
    app.editor.layout.name = "retry unchanged".into();
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    pump(&mut app, &ctx, 700);
    assert!(!path.exists());
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    pump(&mut app, &ctx, 2600);
    let (_, w) = simdisplay::storage::Store::load(path);
    assert_eq!(w.active.name, "retry unchanged");
    assert_eq!(w.active.widgets.len(), 11);
}
#[cfg(unix)]
#[test]
fn failed_backup_keeps_recovery_control_for_retry() {
    use std::os::unix::fs::PermissionsExt;
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("state.json");
    std::fs::write(&path, "broken").unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, path.clone());
    frame(&mut app, &ctx, vec![]);
    let o = frame(&mut app, &ctx, vec![]);
    let label = "สำรองไฟล์เดิม แล้วเริ่มบันทึกใหม่";
    let p = text_pos(&o, label);
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(p), pointer(p, true)],
    );
    frame(&mut app, &ctx, vec![pointer(p, false)]);
    pump(&mut app, &ctx, 100);
    let o = frame(&mut app, &ctx, vec![]);
    let retry = text_pos(&o, label);
    std::fs::set_permissions(t.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken");
    frame(
        &mut app,
        &ctx,
        vec![Event::PointerMoved(retry), pointer(retry, true)],
    );
    frame(&mut app, &ctx, vec![pointer(retry, false)]);
    pump(&mut app, &ctx, 150);
    assert!(simdisplay::storage::Store::load(path).0.error.is_none());
}
#[test]
fn canvas_controls_follow_restored_layout_size() {
    let t = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, t.path().join("state.json"));
    app.editor.resize(1920, 480).unwrap();
    frame(&mut app, &ctx, vec![]);
    let o = frame(&mut app, &ctx, vec![]);
    text_pos(&o, "1920 × 480");
    app.editor.undo();
    frame(&mut app, &ctx, vec![]);
    text_pos(&frame(&mut app, &ctx, vec![]), "1920 × 1080");
}

#[test]
fn style_preset_and_clipboard_buttons_update_selection_and_undo() {
    let t = tempfile::tempdir().unwrap();
    let ctx = egui::Context::default();
    let mut app = DisplayApp::new(&ctx, t.path().join("state.json"));
    app.editor.selected = Some(app.editor.layout.widgets[1].id.clone());
    let original = app.editor.layout.clone();
    let click = |app: &mut DisplayApp, label: &str| {
        frame(app, &ctx, vec![]);
        let o = frame(app, &ctx, vec![]);
        let p = text_pos(&o, label);
        frame(app, &ctx, vec![Event::PointerMoved(p), pointer(p, true)]);
        frame(app, &ctx, vec![pointer(p, false)]);
    };
    click(&mut app, "Wheel LCD");
    assert_eq!(app.editor.layout.widgets[1].style.radius, 0.);
    assert_eq!(app.editor.layout.widgets[1].font, "condensed");
    app.editor.undo();
    assert_eq!(app.editor.layout, original);
    app.editor.redo();
    click(&mut app, "คัดลอกสไตล์");
    app.editor.selected = Some(app.editor.layout.widgets[2].id.clone());
    click(&mut app, "วางสไตล์");
    assert_eq!(
        app.editor.layout.widgets[2].style,
        app.editor.layout.widgets[1].style
    );
    app.editor.undo();
    assert_eq!(app.editor.layout.widgets[2], original.widgets[2]);
}
