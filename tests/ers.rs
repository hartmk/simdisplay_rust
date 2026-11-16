use eframe::egui::{self, Rect, pos2, vec2};
use simdisplay::{
    f1::parse_f1,
    model::{Layout, Widget},
    telemetry::{Data, Receiver, TelemetryState, demo},
    widgets::paint_widget,
};
use std::{
    net::UdpSocket,
    time::{Duration, Instant},
};

fn packet(year: u16, id: u8, size: usize, player: u8) -> Vec<u8> {
    let mut b = vec![0; size];
    b[..2].copy_from_slice(&year.to_le_bytes());
    b[2] = (year % 100) as u8;
    b[5] = 1;
    b[6] = id;
    b[27] = player;
    b
}
fn put_float(b: &mut [u8], offset: usize, value: f32) {
    b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn player_energy_power_and_mode_in_both_formats() {
    for (year, cars, stride) in [(2025, 22, 55), (2026, 24, 59)] {
        let player = cars - 1;
        let mut b = packet(year, 7, 29 + cars * stride, player as u8);
        let o = 29 + player * stride;
        put_float(&mut b, o + 33, 350_000.);
        put_float(&mut b, o + 37, 2_500_000.);
        b[o + 41] = 3;
        let d = parse_f1(&b).unwrap().data;
        assert_eq!(d["ersEnergy"], 2.5);
        assert_eq!(d["electricPower"], 350.);
        assert_eq!(d["ersDeployMode"], 3);
        assert!(parse_f1(&b[..b.len() - 1]).is_none());

        let mut state = TelemetryState::default();
        state.ingest(parse_f1(&b).unwrap(), 0);
        put_float(&mut b, o + 37, f32::NAN);
        put_float(&mut b, o + 33, -1.);
        b[o + 41] = 255;
        state.ingest(parse_f1(&b).unwrap(), 100);
        for key in ["ersEnergy", "electricPower", "ersDeployMode"] {
            assert!(state.snapshot(100)[key].is_null());
        }
        assert!(state.snapshot(2600).is_empty());
    }
}

#[test]
fn season_2026_packet_sizes_and_telemetry_offsets() {
    for (id, size, key) in [
        (1, 926, "sessionTime"),
        (2, 1399, "lapTime"),
        (6, 1448, "speed"),
        (7, 1445, "fuel"),
        (10, 1133, "tyreWear"),
        (11, 1460, "bestLap"),
    ] {
        let mut b = packet(2026, id, size, 23);
        if id == 11 {
            b[29] = 23;
        }
        assert!(parse_f1(&b).unwrap().data.contains_key(key));
        assert!(parse_f1(&b[..size - 1]).is_none());
    }
    let mut b = packet(2026, 6, 1448, 23);
    let o = 29 + 23 * 59;
    b[o..o + 2].copy_from_slice(&301u16.to_le_bytes());
    for (i, v) in [21., 22., 23., 24.].into_iter().enumerate() {
        put_float(&mut b, o + 39 + i * 4, v);
    }
    let d = parse_f1(&b).unwrap().data;
    assert_eq!(d["speed"], 301);
    assert_eq!(d["tyrePressures"], serde_json::json!([23., 24., 21., 22.]));
    b[27] = 24;
    assert!(parse_f1(&b).is_none());
    b[27] = 0;
    b[2] = 25;
    assert!(parse_f1(&b).is_none());
}

fn rendered(kind: &str, data: &Data) -> Vec<String> {
    let ctx = egui::Context::default();
    let widget = Widget::new(kind, &Layout::default());
    ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            paint_widget(
                ui.painter(),
                Rect::from_min_size(pos2(0., 0.), vec2(420., 240.)),
                &widget,
                data,
                1.,
            );
        });
    })
    .shapes
    .into_iter()
    .filter_map(|shape| match shape.shape {
        egui::Shape::Text(t) => Some(t.galley.text().into()),
        _ => None,
    })
    .collect()
}

#[test]
fn energy_widgets_render_units_unknowns_and_roundtrip() {
    let mut layout = Layout::default();
    let data = serde_json::json!({"ersEnergy":2.5,"electricPower":350.,"ersDeployMode":3})
        .as_object()
        .unwrap()
        .clone();
    for (kind, value, unit) in [
        ("ersEnergy", "2.50", "MJ"),
        ("electricPower", "350.0", "kW"),
        ("ersDeployMode", "BOOST", ""),
    ] {
        let text = rendered(kind, &data);
        assert!(text.contains(&value.into()));
        if !unit.is_empty() {
            assert!(text.contains(&unit.into()));
        }
        assert!(rendered(kind, &Data::new()).contains(&"—".into()));
        assert!(demo(0.).contains_key(kind));
        layout.widgets.push(Widget::new(kind, &layout));
    }
    assert_eq!(
        Layout::parse(&serde_json::to_string(&layout).unwrap()).unwrap(),
        layout
    );
}

#[test]
fn season_2026_energy_arrives_over_udp() {
    let receiver = Receiver::bind("127.0.0.1:0").unwrap();
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut b = packet(2026, 7, 1445, 23);
    put_float(&mut b, 29 + 23 * 59 + 37, 3_000_000.);
    sender.send_to(&b, receiver.local_addr()).unwrap();
    let start = Instant::now();
    while !receiver.snapshot().live && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(receiver.snapshot().data["ersEnergy"], 3.);
}
