use simdisplay::{editor::Editor, model::Layout};
#[test]
fn editor_commands_lock_and_history() {
    let mut e = Editor::new(Layout::default());
    let n = e.layout.widgets.len();
    e.add("speed");
    assert_eq!(e.layout.widgets.len(), n + 1);
    e.duplicate();
    assert_eq!(e.layout.widgets.len(), n + 2);
    let id = e.selected.clone().unwrap();
    e.layout
        .widgets
        .iter_mut()
        .find(|w| w.id == id)
        .unwrap()
        .locked = true;
    let before = e.layout.clone();
    e.nudge(10., 0.);
    assert_eq!(e.layout, before);
    e.delete();
    assert_eq!(e.layout.widgets.len(), n + 1);
    e.undo();
    assert_eq!(e.layout.widgets.len(), n + 2);
}
#[test]
fn a_drag_is_one_undo_step() {
    let mut e = Editor::new(Layout::default());
    e.add("speed");
    let original = e.layout.clone();
    e.begin_edit();
    for _ in 0..20 {
        e.selected_mut().unwrap().x += 1.;
    }
    e.end_edit();
    e.undo();
    assert_eq!(e.layout, original);
    e.redo();
    assert_eq!(e.selected_mut().unwrap().x, 100.);
}
#[test]
fn limit_and_resize_are_valid() {
    let mut e = Editor::new(Layout::default());
    for _ in 0..120 {
        e.add("text");
    }
    assert_eq!(e.layout.widgets.len(), 100);
    let old = e.layout.clone();
    e.resize(480, 1920).unwrap();
    assert!(e.layout.validate().is_ok());
    e.undo();
    assert_eq!(e.layout, old);
}
#[test]
fn every_widget_paints_demo_and_missing_data() {
    let ctx = eframe::egui::Context::default();
    for data in [simdisplay::telemetry::demo(4.), Default::default()] {
        let output = ctx.run(Default::default(), |ctx| {
            eframe::egui::CentralPanel::default().show(ctx, |ui| {
                let l = Layout::parse(include_str!("fixtures/all-widgets.json")).unwrap();
                for w in l.widgets {
                    let rect = eframe::egui::Rect::from_min_size(
                        eframe::egui::pos2(0., 0.),
                        eframe::egui::vec2(w.w as f32, w.h as f32),
                    );
                    simdisplay::widgets::paint_widget(ui.painter(), rect, &w, &data, 1.);
                }
            });
        });
        assert!(!output.shapes.is_empty());
    }
}
#[test]
fn keyboard_resize_respects_lock_and_canvas() {
    let mut e = Editor::new(Layout::default());
    e.add("speed");
    e.nudge_size(10., -10.);
    assert_eq!(
        (e.selected_mut().unwrap().w, e.selected_mut().unwrap().h),
        (430., 230.)
    );
    e.selected_mut().unwrap().locked = true;
    let before = e.layout.clone();
    e.nudge_size(10., 10.);
    assert_eq!(e.layout, before);
}

#[test]
fn copied_widget_survives_deleting_original_and_paste_is_undoable() {
    let mut e = Editor::new(Layout::default());
    e.selected = Some(e.layout.widgets[0].id.clone());
    e.selected_mut().unwrap().font = "system:Example-Regular".into();
    let original = e.selected_mut().unwrap().clone();
    let before_copy = e.layout.clone();
    assert!(e.copy().is_some());
    assert_eq!(e.layout, before_copy);
    assert!(!e.history.can_undo());
    e.delete();
    let before_paste = e.layout.clone();
    e.paste();
    let pasted = e.selected_mut().unwrap();
    assert_ne!(pasted.id, original.id);
    assert_eq!(pasted.font, original.font);
    assert_eq!(pasted.kind, original.kind);
    assert_eq!(pasted.style, original.style);
    e.layout.validate().unwrap();
    let after = e.layout.clone();
    e.undo();
    assert_eq!(e.layout, before_paste);
    e.redo();
    assert_eq!(e.layout, after);
}

#[test]
fn layers_move_one_step_preserve_selection_and_obey_edges() {
    let mut e = Editor::new(Layout::default());
    let original = e.layout.clone();
    let id = e.layout.widgets[0].id.clone();
    e.selected = Some(id.clone());
    e.layer(false);
    assert_eq!(e.layout, original);
    assert!(!e.history.can_undo());
    e.layer(true);
    assert_eq!(e.layout.widgets[1].id, id);
    assert_eq!(e.selected.as_ref(), Some(&id));
    e.undo();
    assert_eq!(e.layout, original);
    e.redo();
    e.layer(false);
    assert_eq!(e.layout, original);
}
