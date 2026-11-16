use eframe::egui::{self, Rect, pos2, vec2};
use simdisplay::{
    f1::parse_f1,
    f1_output::{field, packets},
    model::{Layout, Widget, catalog},
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
    if matches!(id, 11 | 12) {
        b[29] = player;
    }
    b
}

#[test]
fn all_sixteen_packet_sizes_match_the_supplied_pdf() {
    let sizes = [
        1349, 753, 1285, 45, 1284, 1133, 1352, 1239, 1042, 954, 1041, 1460, 231, 273, 101, 1131,
    ];
    assert_eq!(packets().len(), 16);
    assert_eq!(packets().iter().map(|p| p.fields.len()).sum::<usize>(), 402);
    for (id, size) in sizes.into_iter().enumerate() {
        assert_eq!(packets()[id].size, size);
        let b = packet(id as u8, size, 21);
        let p = parse_f1(&b).unwrap();
        assert_eq!(p.data["f1.playerCarIndex"], 21);
        assert!(parse_f1(&b[..size - 1]).is_none());
        let mut oversized = b.clone();
        oversized.push(0);
        assert!(parse_f1(&oversized).is_none());
    }
}

fn selected(key: &str, data: &Data, indices: Vec<Option<usize>>) -> serde_json::Value {
    let mut widget = Widget::new(key, &Layout::default());
    widget.output_indices = indices;
    field(key)
        .unwrap()
        .value(data, &widget)
        .cloned()
        .unwrap_or_default()
}

#[test]
fn signed_motion_selects_player_and_explicit_opponent() {
    let mut b = packet(0, 1349, 21);
    b[29..33].copy_from_slice(&9f32.to_le_bytes());
    b[29 + 21 * 60..33 + 21 * 60].copy_from_slice(&(-12.5f32).to_le_bytes());
    let p = parse_f1(&b).unwrap();
    let key = "f1.motion.carMotionData.worldPositionX";
    assert_eq!(selected(key, &p.data, vec![]).as_f64(), Some(-12.5));
    assert_eq!(selected(key, &p.data, vec![Some(0)]).as_f64(), Some(9.));
    assert_eq!(
        field(key).unwrap().text(Some(&serde_json::json!(-12.5))),
        "-12.50"
    );
    b[29 + 21 * 60..33 + 21 * 60].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(selected(key, &parse_f1(&b).unwrap().data, vec![]).is_null());
}

#[test]
fn participants_use_57_byte_records_and_utf8_names_and_colour_indices() {
    let mut b = packet(4, 1284, 21);
    b[29] = 22;
    let base = 30 + 21 * 57;
    let name = "นักแข่ง".as_bytes();
    b[base + 7..base + 7 + name.len()].copy_from_slice(name);
    b[base + 44] = 4;
    b[base + 45 + 3 * 3 + 2] = 123;
    let p = parse_f1(&b).unwrap();
    assert_eq!(
        selected("f1.participants.participants.name", &p.data, vec![]),
        "นักแข่ง"
    );
    assert_eq!(
        selected(
            "f1.participants.participants.liveryColours.blue",
            &p.data,
            vec![None, Some(3)]
        ),
        123
    );
    b[29] = 23;
    assert!(parse_f1(&b).is_none());
}

#[test]
fn event_union_decodes_only_matching_variant_and_clears_old_payload() {
    let mut b = packet(3, 45, 0);
    b[29..33].copy_from_slice(b"FTLP");
    b[33] = 5;
    b[34..38].copy_from_slice(&91.234f32.to_le_bytes());
    let p = parse_f1(&b).unwrap();
    assert_eq!(p.data["f1.event.eventStringCode"], "FTLP");
    assert_eq!(p.data["f1.event.eventDetails.FastestLap.vehicleIdx"], 5);
    assert!(p.data["f1.event.eventDetails.Penalty.vehicleIdx"].is_null());
    let mut state = TelemetryState::default();
    state.ingest(p, 0);
    b[29..33].copy_from_slice(b"SSTA");
    state.ingest(parse_f1(&b).unwrap(), 100);
    assert!(state.snapshot(100)["f1.event.eventDetails.FastestLap.vehicleIdx"].is_null());
    assert_eq!(state.snapshot(7000)["f1.event.eventStringCode"], "SSTA");
}

#[test]
fn valid_counts_and_two_dimensional_lap_positions() {
    let mut b = packet(15, 1131, 21);
    b[29] = 2;
    b[30] = 7;
    b[31 + 22 + 21] = 4;
    let p = parse_f1(&b).unwrap();
    let key = "f1.lapPositions.positionForVehicleIdx";
    assert_eq!(selected(key, &p.data, vec![Some(1), None]), 4);
    assert!(selected(key, &p.data, vec![Some(2), None]).is_null());
    let mut history = packet(11, 1460, 21);
    history[30] = 1;
    history[36..40].copy_from_slice(&91234u32.to_le_bytes());
    let p = parse_f1(&history).unwrap();
    let key = "f1.sessionHistory.lapHistoryData.lapTimeInMS";
    assert_eq!(selected(key, &p.data, vec![]).as_f64(), Some(91234.));
    assert!(selected(key, &p.data, vec![Some(1)]).is_null());
    history[29] = 20;
    assert!(parse_f1(&history).is_none());
}

#[test]
fn slow_participants_survive_until_next_update_but_clear_with_session() {
    let mut state = TelemetryState::default();
    let mut b = packet(4, 1284, 0);
    b[29] = 1;
    state.ingest(parse_f1(&b).unwrap(), 0);
    assert!(
        state
            .snapshot(5000)
            .contains_key("f1.participants.participants.name")
    );
    assert!(
        !state
            .snapshot(6500)
            .contains_key("f1.participants.participants.name")
    );
    let mut b = packet(8, 1042, 0);
    b[29] = 1;
    state.ingest(parse_f1(&b).unwrap(), 0);
    assert!(
        state
            .snapshot(100000)
            .contains_key("f1.finalClassification.classificationData.position")
    );
    let mut b = packet(1, 753, 0);
    b[7] = 1;
    state.ingest(parse_f1(&b).unwrap(), 100001);
    assert!(
        !state
            .snapshot(100001)
            .contains_key("f1.finalClassification.classificationData.position")
    );
}

#[test]
fn every_output_widget_renders_demo_and_missing_values_and_roundtrips() {
    let ctx = egui::Context::default();
    let demo = demo(4.);
    let empty = Data::new();
    let mut layout = Layout::default();
    let mut keys = std::collections::HashSet::new();
    for (kind, _, _) in catalog() {
        assert!(keys.insert(kind), "duplicate catalog key: {kind}");
        if field(kind).is_none() {
            continue;
        }
        let mut widget = Widget::new(kind, &layout);
        for (i, dim) in field(kind).unwrap().dimensions.iter().enumerate() {
            widget.output_indices.resize(i + 1, None);
            widget.output_indices[i] = Some(dim.count - 1);
        }
        for data in [&demo, &empty] {
            let output = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    paint_widget(
                        ui.painter(),
                        Rect::from_min_size(pos2(0., 0.), vec2(420., 240.)),
                        &widget,
                        data,
                        1.,
                    );
                });
            });
            let texts: Vec<_> = output
                .shapes
                .into_iter()
                .filter_map(|s| match s.shape {
                    egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                    _ => None,
                })
                .collect();
            assert!(texts.len() >= 2, "{kind}");
            assert_eq!(texts.iter().any(|s| s == "—"), data.is_empty(), "{kind}");
        }
        layout.widgets = vec![widget];
        assert_eq!(
            Layout::parse(&serde_json::to_string(&layout).unwrap()).unwrap(),
            layout
        );
    }
    let mut widget = Widget::new("f1.carTelemetry.carTelemetryData.tyresPressure", &layout);
    widget.output_indices = vec![Some(22), Some(0)];
    layout.widgets = vec![widget];
    assert!(layout.validate().is_err());
}
