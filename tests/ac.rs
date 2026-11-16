use simdisplay::{
    ac::{self, Tracker},
    f1_output::field,
    model::{Layout, Widget},
};

fn pages() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut physics = vec![0; 580];
    let mut graphics = vec![0; 296];
    let mut statics = vec![0; 684];
    put_int(&mut physics, 0, 1);
    put_int(&mut graphics, 0, 1);
    put_int(&mut graphics, 4, 2);
    put_int(&mut statics, 412, 9000);
    (physics, graphics, statics)
}
fn put_int(b: &mut [u8], offset: usize, n: i32) {
    b[offset..offset + 4].copy_from_slice(&n.to_le_bytes());
}
fn put_float(b: &mut [u8], offset: usize, n: f32) {
    b[offset..offset + 4].copy_from_slice(&n.to_le_bytes());
}
fn put_text(b: &mut [u8], offset: usize, text: &str) {
    for (i, unit) in text.encode_utf16().chain(Some(0)).enumerate() {
        b[offset + i * 2..offset + i * 2 + 2].copy_from_slice(&unit.to_le_bytes());
    }
}

#[test]
fn shared_memory_layouts_are_windows_pack4_with_utf16() {
    assert_eq!(
        ac::packets().iter().map(|p| p.size).collect::<Vec<_>>(),
        vec![580, 296, 684]
    );
    assert_eq!(ac::catalog().count(), 130);
    assert_eq!(field("ac.physics.brakeTemp").unwrap().offset, 348);
    assert_eq!(field("ac.graphics.tyreCompound").unwrap().offset, 176);
    assert_eq!(field("ac.static.maxRpm").unwrap().offset, 412);
    let (p, g, s) = pages();
    for (p, g, s) in [
        (&p[..579], g.as_slice(), s.as_slice()),
        (p.as_slice(), &g[..295], s.as_slice()),
        (p.as_slice(), g.as_slice(), &s[..683]),
    ] {
        assert!(ac::parse(p, g, s).is_none());
    }
}

#[test]
fn common_dashboard_uses_ac_gears_litres_percentages_and_milliseconds() {
    let (mut p, mut g, mut s) = pages();
    put_float(&mut p, 4, 0.75);
    put_float(&mut p, 8, 0.25);
    put_float(&mut p, 12, 42.5);
    put_int(&mut p, 16, 4);
    put_int(&mut p, 20, 8000);
    put_float(&mut p, 28, 240.);
    put_int(&mut g, 132, 6);
    put_int(&mut g, 136, 2);
    put_int(&mut g, 140, 12345);
    put_int(&mut g, 144, 91234);
    put_int(&mut g, 148, 90123);
    put_float(&mut g, 152, 125000.);
    put_text(&mut g, 176, "Soft");
    put_text(&mut s, 68, "รถทดสอบ");
    put_text(&mut s, 134, "สนาม 🏁");
    let d = ac::parse(&p, &g, &s).unwrap();
    for (key, n) in [
        ("speed", 240.),
        ("rpm", 8000.),
        ("gear", 3.),
        ("throttle", 75.),
        ("brake", 25.),
        ("fuel", 42.5),
        ("maxRpm", 9000.),
        ("lap", 7.),
        ("position", 2.),
        ("lapTime", 12345.),
        ("lastLap", 91234.),
        ("bestLap", 90123.),
        ("sessionTime", 125.),
    ] {
        assert_eq!(d[key].as_f64(), Some(n), "{key}");
    }
    assert_eq!(d["fuelUnit"], "L");
    assert_eq!(d["ac.static.carModel"], "รถทดสอบ");
    assert_eq!(d["ac.static.track"], "สนาม 🏁");
    assert_eq!(d["tyreCompound"], "Soft");
    for (raw, gear) in [(0, -1.), (1, 0.), (2, 1.)] {
        put_int(&mut p, 16, raw);
        assert_eq!(ac::parse(&p, &g, &s).unwrap()["gear"].as_f64(), Some(gear));
    }
}

#[test]
fn ac_wheel_order_is_fl_fr_rl_rr_and_contact_arrays_are_four_by_three() {
    let (mut p, g, s) = pages();
    for (i, n) in [81., 82., 83., 84.].into_iter().enumerate() {
        put_float(&mut p, 152 + i * 4, n);
        put_float(&mut p, 88 + i * 4, 24. + i as f32);
        put_float(&mut p, 348 + i * 4, 500. + i as f32);
    }
    put_float(&mut p, 420 + 3 * 12 + 2 * 4, -4.5);
    let d = ac::parse(&p, &g, &s).unwrap();
    assert_eq!(
        d["tyres"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>(),
        vec![81., 82., 83., 84.]
    );
    assert_eq!(d["tyrePressures"][0].as_f64(), Some(24.));
    assert_eq!(d["brakeTemps"][3].as_f64(), Some(503.));
    let key = "ac.physics.tyreContactPoint";
    let mut widget = Widget::new(key, &Layout::default());
    widget.output_indices = vec![Some(3), Some(2)];
    assert_eq!(
        field(key).unwrap().value(&d, &widget).unwrap().as_f64(),
        Some(-4.5)
    );
}

#[test]
fn unknowns_do_not_become_zero_or_f1_assist_modes() {
    let (mut p, mut g, mut s) = pages();
    put_float(&mut p, 12, f32::NAN);
    put_float(&mut p, 28, f32::INFINITY);
    put_float(&mut p, 120, 0.98);
    put_float(&mut p, 204, 0.5);
    put_float(&mut p, 252, 0.5);
    put_int(&mut p, 16, 99);
    put_float(&mut g, 152, -1.);
    let d = ac::parse(&p, &g, &s).unwrap();
    for key in [
        "fuel",
        "speed",
        "gear",
        "lastLap",
        "bestLap",
        "sessionTime",
        "drsActive",
    ] {
        assert!(d[key].is_null(), "{key}");
    }
    for key in [
        "tyreWear",
        "tractionControl",
        "abs",
        "ersEnergy",
        "electricPower",
        "ersDeployMode",
    ] {
        assert!(!d.contains_key(key), "{key}");
    }
    assert!(d["ac.physics.tyreWear"][0].as_f64().unwrap() > 0.97);
    put_int(&mut s, 496, 1);
    put_float(&mut p, 200, 1.);
    assert_eq!(
        ac::parse(&p, &g, &s).unwrap()["drsActive"].as_f64(),
        Some(1.)
    );
    put_float(&mut p, 200, 0.5);
    assert!(ac::parse(&p, &g, &s).unwrap()["drsActive"].is_null());
}

#[test]
fn tracker_distinguishes_waiting_pause_replay_and_stale_counters() {
    let (mut p, mut g, s) = pages();
    let mut tracker = Tracker::default();
    assert_eq!(tracker.update(&p, &g, &s, 0).status, "LIVE");
    assert_eq!(tracker.update(&p, &g, &s, 2499).status, "LIVE");
    let stale = tracker.update(&p, &g, &s, 2500);
    assert_eq!(stale.status, "WAITING");
    assert!(stale.data.is_empty());
    // Reopening the same frozen page must not turn stale values live again.
    assert_eq!(tracker.update(&p, &g, &s, 5000).status, "WAITING");
    put_int(&mut p, 0, 2);
    assert_eq!(tracker.update(&p, &g, &s, 5001).status, "LIVE");
    for (status, label) in [(3, "PAUSED"), (1, "REPLAY"), (0, "WAITING"), (4, "ERROR")] {
        put_int(&mut g, 4, status);
        put_int(&mut g, 0, status);
        assert_eq!(tracker.update(&p, &g, &s, 5002).status, label);
    }
}

#[cfg(windows)]
mod windows_mapping {
    use super::*;
    use simdisplay::{ac::AcReceiver, named_memory::ReadMapping};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
        System::Memory::{
            CreateFileMappingW, FILE_MAP_READ, FILE_MAP_WRITE, MEMORY_MAPPED_VIEW_ADDRESS,
            MapViewOfFile, PAGE_READWRITE, UnmapViewOfFile,
        },
    };
    struct Writer {
        handle: HANDLE,
        view: MEMORY_MAPPED_VIEW_ADDRESS,
        length: usize,
    }
    impl Writer {
        fn new(name: &str, bytes: &[u8]) -> Self {
            let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let handle = unsafe {
                CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null(),
                    PAGE_READWRITE,
                    0,
                    bytes.len() as u32,
                    name.as_ptr(),
                )
            };
            assert!(!handle.is_null());
            let view =
                unsafe { MapViewOfFile(handle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, bytes.len()) };
            assert!(!view.Value.is_null());
            let writer = Self {
                handle,
                view,
                length: bytes.len(),
            };
            writer.write(bytes);
            writer
        }
        fn write(&self, bytes: &[u8]) {
            assert_eq!(bytes.len(), self.length);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    self.view.Value.cast::<u8>(),
                    bytes.len(),
                );
            }
        }
    }
    impl Drop for Writer {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(self.view);
                CloseHandle(self.handle);
            }
        }
    }
    #[test]
    fn real_windows_named_mapping_lifecycle_read_only_receiver_and_reconnect() {
        let prefix = format!("Local\\simdisplay-ac-test-{}", uuid::Uuid::new_v4());
        let names = [
            format!("{prefix}-physics"),
            format!("{prefix}-graphics"),
            format!("{prefix}-static"),
        ];
        let mut missing = AcReceiver::with_names(names.clone());
        assert_eq!(missing.snapshot().status, "WAITING");
        // A reader waiting for the game must not create a fake page itself.
        assert!(ReadMapping::open(&names[0], 580).unwrap().is_none());
        drop(missing);
        let (mut p, g, s) = pages();
        let partial = Writer::new(&names[0], &p);
        let mut receiver = AcReceiver::with_names(names.clone());
        assert_eq!(receiver.snapshot().status, "WAITING");
        drop(partial);
        assert!(
            ReadMapping::open(&names[0], 580).unwrap().is_none(),
            "partial connection must release its first mapping"
        );
        drop(receiver);
        put_float(&mut p, 28, 123.);
        let writers = [
            Writer::new(&names[0], &p),
            Writer::new(&names[1], &g),
            Writer::new(&names[2], &s),
        ];
        let mut receiver = AcReceiver::with_names(names.clone());
        let sample = receiver.snapshot();
        assert_eq!(sample.status, "LIVE");
        assert_eq!(sample.data["speed"].as_f64(), Some(123.));
        put_int(&mut p, 0, 2);
        put_float(&mut p, 28, 234.);
        writers[0].write(&p);
        assert_eq!(receiver.snapshot().data["speed"].as_f64(), Some(234.));
        drop(receiver);
        let mut reopened = AcReceiver::with_names(names.clone());
        assert_eq!(reopened.snapshot().status, "LIVE");
        drop(reopened);
        drop(writers);
        for (i, name) in names.iter().enumerate() {
            assert!(
                ReadMapping::open(name, ac::packets()[i].size)
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn ac_raw_pedal_and_bias_fields_preserve_ratios() {
    let (mut p, g, s) = pages();
    for (offset, key) in [
        (4, "gas"),
        (8, "brake"),
        (364, "clutch"),
        (564, "brakeBias"),
    ] {
        put_float(&mut p, offset, 0.5);
        let d = ac::parse(&p, &g, &s).unwrap();
        assert_eq!(d[&format!("ac.physics.{key}")].as_f64(), Some(0.5), "{key}");
    }
    let d = ac::parse(&p, &g, &s).unwrap();
    assert_eq!(d["throttle"].as_f64(), Some(50.));
    assert_eq!(d["brake"].as_f64(), Some(50.));
}

#[test]
fn curated_ac_fields_are_available_for_layouts_and_keep_array_dimensions() {
    let mut layout = Layout::default();
    layout.widgets.clear();
    for (_, keys) in ac::WIDGET_GROUPS {
        for key in *keys {
            layout.widgets.push(Widget::new(key, &layout));
        }
    }
    assert_eq!(layout.widgets.len(), 63);
    layout.validate().unwrap();
    let saved = serde_json::to_string(&layout).unwrap();
    assert_eq!(Layout::parse(&saved).unwrap(), layout);
    assert_eq!(
        field("ac.physics.rideHeight").unwrap().dimensions[0].count,
        2
    );
    assert_eq!(
        field("ac.physics.carDamage").unwrap().dimensions[0].count,
        5
    );
    assert_eq!(
        field("ac.physics.localVelocity").unwrap().dimensions[0].count,
        3
    );
}
