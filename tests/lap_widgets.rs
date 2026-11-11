use eframe::egui::{self, Color32, Rect, pos2, vec2};
use simdisplay::{
    model::{Layout, Widget},
    telemetry::Data,
    widgets::{color, paint_widget},
};
fn painted(kind: &str, data: &Data, threshold: Option<f64>) -> Vec<(String, Color32)> {
    let ctx = egui::Context::default();
    let mut w = Widget::new(kind, &Layout::default());
    w.threshold = threshold;
    let out = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_widget(
                ui.painter(),
                Rect::from_min_size(pos2(0., 0.), vec2(600., 240.)),
                &w,
                data,
                1.,
            )
        });
    });
    out.shapes
        .into_iter()
        .filter_map(|s| match s.shape {
            egui::Shape::Text(t) => Some((t.galley.text().into(), t.fallback_color)),
            _ => None,
        })
        .collect()
}
#[test]
fn separate_lap_widgets_roundtrip_without_changing_combined_widget() {
    let mut l = Layout::default();
    l.widgets.push(Widget::new("bestLap", &l));
    l.widgets.push(Widget::new("lastLapTime", &l));
    let restored = Layout::parse(&serde_json::to_string(&l).unwrap()).unwrap();
    assert_eq!(restored, l);
    assert_eq!(
        restored
            .widgets
            .iter()
            .find(|w| w.kind == "lastLap")
            .unwrap()
            .label,
        "Lap times"
    );
    assert_eq!(
        restored
            .widgets
            .iter()
            .find(|w| w.kind == "bestLap")
            .unwrap()
            .label,
        "Best lap"
    );
    assert_eq!(
        restored
            .widgets
            .iter()
            .find(|w| w.kind == "lastLapTime")
            .unwrap()
            .label,
        "Last lap"
    );
}
#[test]
fn lap_widgets_use_correct_telemetry_and_millisecond_warning_threshold() {
    let d = serde_json::json!({"bestLap":90234,"lastLap":93420,"lapTime":12345})
        .as_object()
        .unwrap()
        .clone();
    for (kind, expected, other) in [
        ("bestLap", "1:30.234", "1:33.420"),
        ("lastLapTime", "1:33.420", "1:30.234"),
    ] {
        let text = painted(kind, &d, Some(90000.));
        assert!(text.contains(&(expected.into(), color("#ff654e"))));
        assert!(!text.iter().any(|(s, _)| s == other || s == "0:12.345"));
    }
}
#[test]
fn absent_lap_times_display_dash_instead_of_zero() {
    for kind in ["bestLap", "lastLapTime"] {
        for data in [
            Data::new(),
            serde_json::json!({"bestLap":null,"lastLap":null})
                .as_object()
                .unwrap()
                .clone(),
        ] {
            let text = painted(kind, &data, None);
            assert!(text.iter().any(|(s, _)| s == "—"));
            assert!(!text.iter().any(|(s, _)| s == "0:00.000"));
        }
    }
}
