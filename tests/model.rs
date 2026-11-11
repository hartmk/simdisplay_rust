use simdisplay::{history::History, model::*};
#[test]
fn original_layouts_round_trip() {
    for text in [
        include_str!("fixtures/default.json"),
        include_str!("fixtures/all-widgets.json"),
        include_str!("fixtures/legacy.json"),
    ] {
        let l = Layout::parse(text).unwrap();
        assert_eq!(l.width, 1920);
        assert_eq!(l.height, 1080);
        assert_eq!(
            Layout::parse(&serde_json::to_string(&l).unwrap()).unwrap(),
            l
        );
    }
    let l = Layout::parse(include_str!("fixtures/legacy.json")).unwrap();
    assert_eq!(l.widgets[3].background_opacity, 100.0);
    let rpm = l.widgets.iter().find(|w| w.kind == "rpm").unwrap();
    assert_eq!(rpm.rpm_state(Some(9000.0), None).color, "#ff654e");
}
#[test]
fn rejects_invalid_imports() {
    let base: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/default.json")).unwrap();
    for (key, value) in [
        ("x", serde_json::json!(-1)),
        ("backgroundOpacity", serde_json::json!(101)),
        ("fontSize", serde_json::json!(0)),
        ("type", serde_json::json!("unknown")),
    ] {
        let mut l = base.clone();
        l["widgets"][0][key] = value;
        assert!(Layout::parse(&l.to_string()).is_err(), "{key}");
    }
    let mut l = base.clone();
    l["widgets"][1]["id"] = l["widgets"][0]["id"].clone();
    assert!(Layout::parse(&l.to_string()).is_err());
    let mut l = base;
    l["width"] = serde_json::json!(319);
    assert!(Layout::parse(&l.to_string()).is_err());
}
#[test]
fn resize_scales_locked_geometry_and_is_reversible() {
    let mut l = Layout::default();
    l.widgets[0].locked = true;
    let mut h = History::default();
    h.record(&l);
    l.resize(1920, 480).unwrap();
    assert_eq!(l.widgets[0].y, 18.0);
    assert!(l.validate().is_ok());
    h.undo(&mut l);
    assert_eq!(l.height, 1080);
    h.redo(&mut l);
    assert_eq!(l.height, 480);
    l.resize(1920, 1080).unwrap();
    assert_eq!(l.widgets[0].font_size, 70.0);
}
#[test]
fn history_is_bounded_and_new_edit_clears_redo() {
    let mut l = Layout::default();
    let mut h = History::default();
    for i in 0..65 {
        h.record(&l);
        l.name = i.to_string();
    }
    for _ in 0..70 {
        h.undo(&mut l);
    }
    assert_eq!(l.name, "4");
    h.redo(&mut l);
    assert_eq!(l.name, "5");
    h.record(&l);
    l.name = "new".into();
    h.redo(&mut l);
    assert_eq!(l.name, "new");
}
#[test]
fn rpm_boundaries_fallback_and_bar_modes() {
    let l = Layout::default();
    let mut w = Widget::new("rpm", &l);
    assert_eq!(w.rpm_state(Some(8375.0), None).color, "#ffca6a");
    assert_eq!(w.rpm_state(Some(11000.0), Some(12500.0)).lit, 22);
    assert_eq!(w.rpm_state(None, None).lit, 0);
    w.rpm_config.as_mut().unwrap().stops[0].at = 1.0;
    let mut l = l;
    l.widgets = vec![w];
    assert!(l.validate().is_err());
}
#[test]
fn integer_valued_json_floats_remain_compatible() {
    let text = include_str!("fixtures/default.json")
        .replace("\"version\": 1,", "\"version\": 1.0,")
        .replace("\"width\": 1920,", "\"width\": 1920.0,")
        .replace("\"height\": 1080,", "\"height\": 1080.0,");
    assert!(Layout::parse(&text).is_ok());
}
#[test]
fn utf16_label_limit_preserves_valid_prefix() {
    let text = "a".repeat(121);
    assert_eq!(truncate_utf16(&text, 120), "a".repeat(120));
    assert_eq!(truncate_utf16("a🙂b", 3), "a🙂");
}
