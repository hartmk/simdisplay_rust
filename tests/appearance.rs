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

fn widget_text_geometry(w: &simdisplay::model::Widget) -> Vec<(String, egui::Pos2, egui::Vec2)> {
    let ctx = egui::Context::default();
    simdisplay::widgets::configure_fonts(&ctx);
    let output = ctx.run(Default::default(), |ctx| {
        paint_widget(
            &ctx.layer_painter(egui::LayerId::background()),
            Rect::from_min_size(pos2(0., 0.), vec2(300., 150.)),
            w,
            &simdisplay::telemetry::demo(4.),
            1.,
        );
    });
    output
        .shapes
        .into_iter()
        .filter_map(|shape| {
            if let egui::Shape::Text(text) = shape.shape {
                Some((text.galley.text().into(), text.pos, text.galley.size()))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn label_edits_do_not_move_or_resize_existing_values() {
    for kind in [
        "speed",
        "gear",
        "rpmNumber",
        "lastLap",
        "tyres",
        "fuel",
        "ac.physics.gas",
        "text",
    ] {
        for original in [
            serde_json::json!({}),
            serde_json::json!({"showLabel":false}),
            serde_json::json!({"labelSize":30,"labelPosition":"bottom"}),
        ] {
            let mut layout = styled(original);
            let w = &mut layout.widgets[1];
            w.kind = kind.into();
            w.label = "Widget text".into();
            w.style.label_text = Some("Overlay heading".into());
            let before: Vec<_> = widget_text_geometry(w)
                .into_iter()
                .filter(|text| text.0 != w.style.label_text.as_deref().unwrap())
                .collect();
            for (visible, size, position) in [
                (false, 14., "top"),
                (true, 6., "top"),
                (true, 80., "bottom"),
            ] {
                w.style.label_text = Some(format!("Heading {size}"));
                w.style.show_label = visible;
                w.style.label_size = size;
                w.style.label_position = position.into();
                let rendered = widget_text_geometry(w);
                assert_eq!(
                    rendered
                        .iter()
                        .any(|text| text.0 == w.style.label_text.as_deref().unwrap()),
                    visible
                );
                let values: Vec<_> = rendered
                    .into_iter()
                    .filter(|text| text.0 != w.style.label_text.as_deref().unwrap())
                    .collect();
                assert_eq!(values, before, "label affected {kind}");
            }
        }
    }
}

#[test]
fn legacy_label_spacing_and_independent_label_survive_save_reload() {
    for (style, expected) in [
        (serde_json::json!({}), (22., 0.)),
        (
            serde_json::json!({"showLabel":false,"labelSize":40}),
            (0., 0.),
        ),
        (
            serde_json::json!({"labelSize":30,"labelPosition":"bottom"}),
            (0., 38.),
        ),
    ] {
        let mut layout = styled(style);
        let w = &mut layout.widgets[1];
        assert_eq!(
            (w.style.content_inset_top, w.style.content_inset_bottom),
            expected
        );
        w.style.label_text = Some("Custom overlay".into());
        w.style.show_label = !w.style.show_label;
        w.style.label_size = 60.;
        let before = widget_text_geometry(w);
        let restored = Layout::parse(&serde_json::to_string(&layout).unwrap()).unwrap();
        assert_eq!(restored, layout);
        assert_eq!(widget_text_geometry(&restored.widgets[1]), before);
    }
}

#[test]
fn style_presets_and_style_copy_keep_each_widgets_custom_label() {
    let layout = Layout::default();
    let mut source = layout.widgets[0].clone();
    source.style.label_text = Some("Source heading".into());
    let mut target = layout.widgets[1].clone();
    target.style.label_text = Some("My heading".into());
    target.copy_style_from(&source);
    assert_eq!(target.style.label_text.as_deref(), Some("My heading"));
    for preset in ["wheel", "minimal", "card"] {
        target.apply_style_preset(preset);
        assert_eq!(target.style.label_text.as_deref(), Some("My heading"));
    }
}
