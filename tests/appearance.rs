use eframe::egui::{self, Rect, pos2, vec2};
use simdisplay::{model::Layout, telemetry::Data, widgets::paint_widget};

fn styled(style: serde_json::Value) -> Layout {
    let mut value = serde_json::to_value(Layout::default()).unwrap();
    value["widgets"][1]["style"] = style;
    Layout::parse(&value.to_string()).unwrap()
}

#[test]
fn style_survives_export_and_old_layouts_get_defaults() {
    let l = styled(
        serde_json::json!({"radius":0,"borderWidth":2,"borderColor":"#aabbcc","padding":12,"showLabel":false,"showUnit":false,"horizontal":"right","vertical":"bottom","labelSize":20,"labelColor":"#ffcc00","unitSize":12,"unitColor":"#00ccff","labelPosition":"bottom","rpmShape":"led"}),
    );
    let v = serde_json::to_value(&l).unwrap();
    assert_eq!(v["widgets"][1]["style"]["borderWidth"], 2.0);
    assert_eq!(v["widgets"][1]["style"]["horizontal"], "right");
    assert_eq!(Layout::parse(&v.to_string()).unwrap(), l);
    let old = Layout::parse(include_str!("fixtures/legacy.json")).unwrap();
    let v = serde_json::to_value(old).unwrap();
    assert_eq!(v["widgets"][0]["style"]["radius"], 5.0);
    assert_eq!(v["widgets"][0]["style"]["showLabel"], true);
}

#[test]
fn invalid_style_imports_are_rejected() {
    for (key, bad) in [
        ("padding", serde_json::json!(-1)),
        ("radius", serde_json::json!(101)),
        ("borderWidth", serde_json::json!(21)),
        ("borderColor", serde_json::json!("red")),
        ("labelSize", serde_json::json!(0)),
        ("unitSize", serde_json::json!(201)),
        ("horizontal", serde_json::json!("outside")),
        ("vertical", serde_json::json!("outside")),
        ("rpmShape", serde_json::json!("unknown")),
        ("labelPosition", serde_json::json!("outside")),
    ] {
        let mut v = serde_json::to_value(Layout::default()).unwrap();
        v["widgets"][0]["style"] = serde_json::json!({key: bad});
        assert!(Layout::parse(&v.to_string()).is_err(), "{key}");
    }
}

fn texts(style: serde_json::Value) -> Vec<(String, egui::Pos2)> {
    let mut l = styled(style);
    let w = &mut l.widgets[1];
    w.kind = "speed".into();
    w.label = "Speed".into();
    let ctx = egui::Context::default();
    let d: Data = serde_json::json!({"speed":123})
        .as_object()
        .unwrap()
        .clone();
    let output = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_widget(
                ui.painter(),
                Rect::from_min_size(pos2(0., 0.), vec2(420., 240.)),
                w,
                &d,
                1.,
            );
        });
    });
    output
        .shapes
        .into_iter()
        .filter_map(|s| match s.shape {
            egui::Shape::Text(t) => Some((t.galley.text().into(), t.pos)),
            _ => None,
        })
        .collect()
}

#[test]
fn hidden_labels_and_units_are_not_painted_and_alignment_moves_value() {
    let left = texts(
        serde_json::json!({"showLabel":false,"showUnit":false,"horizontal":"left","vertical":"top"}),
    );
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].0, "123");
    let right = texts(
        serde_json::json!({"showLabel":false,"showUnit":false,"horizontal":"right","vertical":"bottom"}),
    );
    assert!(right[0].1.x > left[0].1.x);
    assert!(right[0].1.y > left[0].1.y);
    let visible = texts(serde_json::json!({}));
    assert!(visible.iter().any(|(t, _)| t == "Speed"));
    assert!(visible.iter().any(|(t, _)| t == "km/h"));
}

#[test]
fn copying_presentation_keeps_data_identity_geometry_and_warning_rules_and_undoes() {
    use simdisplay::editor::Editor;
    let mut e = Editor::new(Layout::default());
    let mut source = e.layout.widgets[1].clone();
    source.apply_style_preset("wheel");
    let before = e.layout.widgets[2].clone();
    e.selected = Some(before.id.clone());
    e.paste_style(&source);
    let pasted = &e.layout.widgets[2];
    assert_eq!(pasted.style.radius, 0.);
    assert_eq!(pasted.style.border_width, 1.);
    assert_eq!(pasted.font, "condensed");
    assert_eq!(
        (&pasted.id, &pasted.kind, &pasted.label),
        (&before.id, &before.kind, &before.label)
    );
    assert_eq!(
        (pasted.x, pasted.y, pasted.w, pasted.h),
        (before.x, before.y, before.w, before.h)
    );
    assert_eq!(pasted.threshold, before.threshold);
    assert_eq!(pasted.rpm_config, before.rpm_config);
    assert_eq!(pasted.font_size, before.font_size);
    assert!(e.layout.validate().is_ok());
    e.undo();
    assert_eq!(e.layout.widgets[2], before);
    e.redo();
    assert_eq!(e.layout.widgets[2].style.border_width, 1.);
}

#[test]
fn label_stays_inside_padding_and_border_on_both_edges() {
    let top = texts(serde_json::json!({"borderWidth":20,"padding":30,"labelPosition":"top"}));
    let top_y = top.iter().find(|(t, _)| t == "Speed").unwrap().1.y;
    assert!(
        top_y >= 50.,
        "title must clear border and padding, got {top_y}"
    );
    let bottom = texts(serde_json::json!({"borderWidth":20,"padding":30,"labelPosition":"bottom"}));
    let bottom_y = bottom.iter().find(|(t, _)| t == "Speed").unwrap().1.y;
    assert!(
        bottom_y < 190.,
        "bottom title must clear border and padding, got {bottom_y}"
    );
}
