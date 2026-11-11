use simdisplay::{f1::parse_f1, telemetry::*};
use std::{
    net::UdpSocket,
    time::{Duration, Instant},
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
fn u16_at(b: &mut [u8], o: usize, n: u16) {
    b[o..o + 2].copy_from_slice(&n.to_le_bytes());
}
#[test]
fn telemetry_player_and_wheel_order() {
    let mut b = packet(6, 1352, 2);
    let o = 149;
    u16_at(&mut b, o, 285);
    b[o + 15] = 7;
    u16_at(&mut b, o + 16, 11200);
    b[o + 30..o + 34].copy_from_slice(&[80, 81, 90, 91]);
    let d = parse_f1(&b).unwrap().data;
    assert_eq!(d["speed"], 285);
    assert_eq!(d["rpm"], 11200);
    assert_eq!(d["tyres"], serde_json::json!([90, 91, 80, 81]));
}
#[test]
fn all_packet_kinds_and_bad_input() {
    for (id, n, key) in [
        (1, 753, "sessionTime"),
        (2, 1285, "lapTime"),
        (7, 1239, "fuel"),
        (10, 1041, "tyreWear"),
        (11, 1460, "bestLap"),
    ] {
        let b = packet(id, n, 0);
        assert!(parse_f1(&b).unwrap().data.contains_key(key));
        assert!(parse_f1(&b[..n - 1]).is_none());
    }
    let mut b = packet(6, 1352, 0);
    b[5] = 2;
    assert!(parse_f1(&b).is_none());
    b[5] = 1;
    b[27] = 22;
    assert!(parse_f1(&b).is_none());
    b[27] = 0;
    b[31..35].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(parse_f1(&b).is_none());
    assert!(parse_f1(&[0; 4]).is_none());
}
#[test]
fn valid_best_lap_and_fuel_units() {
    let mut b = packet(11, 1460, 2);
    b[29] = 2;
    b[30] = 3;
    b[32] = 2;
    b[50..54].copy_from_slice(&90234u32.to_le_bytes());
    b[63] = 1;
    assert_eq!(parse_f1(&b).unwrap().data["bestLap"], 90234);
    b[63] = 0;
    assert!(parse_f1(&b).unwrap().data["bestLap"].is_null());
    b[29] = 1;
    assert!(parse_f1(&b).is_none());
    let b = packet(7, 1239, 0);
    assert_eq!(parse_f1(&b).unwrap().data["fuelUnit"], "kg");
}
#[test]
fn per_field_expiry_and_session_reset() {
    let mut s = TelemetryState::default();
    s.ingest(parse_f1(&packet(6, 1352, 0)).unwrap(), 0);
    s.ingest(parse_f1(&packet(7, 1239, 0)).unwrap(), 2000);
    assert!(s.snapshot(2499).contains_key("speed"));
    assert!(!s.snapshot(2500).contains_key("speed"));
    assert!(s.snapshot(2500).contains_key("fuel"));
    let mut b = packet(1, 753, 0);
    b[7] = 1;
    s.ingest(parse_f1(&b).unwrap(), 2600);
    assert!(!s.snapshot(2600).contains_key("fuel"));
    assert!(s.snapshot(5100).is_empty());
}
#[test]
fn loopback_receiver_and_shutdown() {
    let r = Receiver::bind("127.0.0.1:0").unwrap();
    let address = r.local_addr();
    let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
    sender.send_to(&packet(6, 1352, 0), address).unwrap();
    let start = Instant::now();
    while !r.snapshot().data.contains_key("speed") && start.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(r.snapshot().data.contains_key("speed"));
    assert!(Receiver::bind(&address.to_string()).is_err());
    drop(r);
    assert!(Receiver::bind(&address.to_string()).is_ok());
}
