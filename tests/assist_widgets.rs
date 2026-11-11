use eframe::egui::{self, Rect, pos2, vec2};
use simdisplay::{
    f1::parse_f1,
    model::{Layout, Widget},
    telemetry::{Data, TelemetryState, demo},
    widgets::paint_widget,
};
fn packet(id: u8, size: usize, player: u8) -> Vec<u8> {
    let mut b = vec![0; size];
    b[..2].copy_from_slice(&2025u16.to_le_bytes());
    b[2] = 25;
    b[5] = 1;
    b[6] = id;
    b[27] = player;
    b
}
#[test]
fn reads_player_drs_and_assist_settings_from_correct_packets() {
    let mut b = packet(6, 1352, 2);
    b[29 + 2 * 60 + 18] = 1;
    assert_eq!(parse_f1(&b).unwrap().data["drsActive"], 1);
    let mut b = packet(7, 1239, 2);
    b[29 + 2 * 55] = 2;
    b[29 + 2 * 55 + 1] = 1;
    b[29 + 2 * 55 + 22] = 1;
    let d = parse_f1(&b).unwrap().data;
    assert_eq!(d["tractionControl"], 2);
    assert_eq!(d["abs"], 1);
    assert!(
        !d.contains_key("drsActive"),
        "DRS allowed is not DRS active"
    );
    b[29 + 2 * 55] = 0;
    b[29 + 2 * 55 + 1] = 0;
    let d = parse_f1(&b).unwrap().data;
    assert_eq!(d["tractionControl"], 0);
    assert_eq!(d["abs"], 0);
}
#[test]
fn unknown_assist_codes_clear_previous_state_and_fields_expire_independently() {
    let mut s = TelemetryState::default();
    let mut b = packet(6, 1352, 1);
    b[29 + 60 + 18] = 1;
    s.ingest(parse_f1(&b).unwrap(), 0);
    let mut c = packet(7, 1239, 1);
    c[29 + 55] = 1;
    c[29 + 55 + 1] = 1;
    s.ingest(parse_f1(&c).unwrap(), 2000);
    let d = s.snapshot(2500);
    assert!(!d.contains_key("drsActive"));
    assert_eq!(d["tractionControl"], 1);
    assert_eq!(d["abs"], 1);
    b[29 + 60 + 18] = 255;
    c[29 + 55] = 255;
    c[29 + 55 + 1] = 255;
    s.ingest(parse_f1(&b).unwrap(), 2600);
    s.ingest(parse_f1(&c).unwrap(), 2600);
    let d = s.snapshot(2600);
    for k in ["drsActive", "tractionControl", "abs"] {
        assert!(d.get(k).unwrap().is_null());
    }
    assert!(s.snapshot(5100).is_empty());
}
fn texts(kind: &str, value: serde_json::Value) -> Vec<String> {
    let ctx = egui::Context::default();
    let w = Widget::new(kind, &Layout::default());
    let mut d = Data::new();
    d.insert(kind.into(), value);
    let o = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_widget(
                ui.painter(),
                Rect::from_min_size(pos2(0., 0.), vec2(420., 240.)),
                &w,
                &d,
                1.,
            )
        });
    });
    o.shapes
        .into_iter()
        .filter_map(|s| match s.shape {
            egui::Shape::Text(t) => Some(t.galley.text().into()),
            _ => None,
        })
        .collect()
}
#[test]
fn assist_widgets_render_named_states_and_unknown_as_dash() {
    for (kind, value, label) in [
        ("drsActive", 0, "OFF"),
        ("drsActive", 1, "ACTIVE"),
        ("tractionControl", 0, "OFF"),
        ("tractionControl", 1, "MEDIUM"),
        ("tractionControl", 2, "FULL"),
        ("abs", 0, "OFF"),
        ("abs", 1, "ON"),
    ] {
        assert!(texts(kind, value.into()).contains(&label.into()));
    }
    for kind in ["drsActive", "tractionControl", "abs"] {
        assert!(texts(kind, serde_json::Value::Null).contains(&"—".into()));
        assert!(texts(kind, 255.into()).contains(&"—".into()));
    }
}
#[test]
fn assist_widgets_roundtrip_and_demo_provides_valid_states() {
    let mut l = Layout::default();
    for kind in ["drsActive", "tractionControl", "abs"] {
        l.widgets.push(Widget::new(kind, &l));
    }
    assert_eq!(
        Layout::parse(&serde_json::to_string(&l).unwrap()).unwrap(),
        l
    );
    for t in [0., 5., 15., 25.] {
        let d = demo(t);
        assert!([0, 1].contains(&d["drsActive"].as_u64().unwrap()));
        assert!([0, 1, 2].contains(&d["tractionControl"].as_u64().unwrap()));
        assert!([0, 1].contains(&d["abs"].as_u64().unwrap()));
    }
}
