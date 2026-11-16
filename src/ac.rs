//! Assetto Corsa (original PC game), Windows shared-memory telemetry.
use crate::{
    f1_output::{OutputPacket, read_dimensions},
    telemetry::Data,
};
use serde_json::{Value, json};
use std::sync::OnceLock;

pub fn packets() -> &'static [OutputPacket] {
    static PAGES: OnceLock<Vec<OutputPacket>> = OnceLock::new();
    PAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/ac-fields.json"))
            .expect("bundled AC page metadata")
    })
}
pub fn catalog() -> impl Iterator<Item = (&'static str, &'static str, f64)> {
    packets()
        .iter()
        .flat_map(|p| &p.fields)
        .map(|f| (f.key.as_str(), f.label.as_str(), 64.))
}
pub fn parse(physics: &[u8], graphics: &[u8], statics: &[u8]) -> Option<Data> {
    let pages = [physics, graphics, statics];
    if pages.iter().zip(packets()).any(|(b, p)| b.len() < p.size) {
        return None;
    }
    let mut data = Data::new();
    for (bytes, page) in pages.into_iter().zip(packets()) {
        for field in &page.fields {
            data.insert(
                field.key.clone(),
                read_dimensions(bytes, field, 0, field.offset),
            );
        }
    }
    let number = |key: &str| {
        data.get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
    };
    let positive = |key: &str| number(key).filter(|n| *n > 0.);
    let nonnegative = |key: &str| number(key).filter(|n| *n >= 0.);
    let ratio = |key: &str| number(key).filter(|n| (0. ..=100.).contains(n));
    let gear = number("ac.physics.gear")
        .filter(|n| (0. ..=10.).contains(n))
        .map(|n| n - 1.);
    let drs = if number("ac.static.hasDRS") == Some(1.) {
        number("ac.physics.drs").filter(|n| *n == 0. || *n == 1.)
    } else {
        None
    };
    let common = json!({
        "speed":nonnegative("ac.physics.speedKmh"), "gear":gear,
        "rpm":nonnegative("ac.physics.rpms"), "maxRpm":positive("ac.static.maxRpm"),
        "fuel":nonnegative("ac.physics.fuel"), "fuelUnit":"L",
        "throttle":ratio("ac.physics.gas"), "brake":ratio("ac.physics.brake"),
        "lapTime":nonnegative("ac.graphics.iCurrentTime"), "lastLap":positive("ac.graphics.iLastTime"),
        "bestLap":positive("ac.graphics.iBestTime"), "position":positive("ac.graphics.position"),
        "lap":nonnegative("ac.graphics.completedLaps").map(|n|n+1.),
        "sessionTime":nonnegative("ac.graphics.sessionTimeLeft").map(|n|n/1000.),
        "tyres":data.get("ac.physics.tyreCoreTemperature"),
        "tyrePressures":data.get("ac.physics.wheelsPressure"),
        "brakeTemps":data.get("ac.physics.brakeTemp"),
        "tyreCompound":data.get("ac.graphics.tyreCompound").and_then(Value::as_str).filter(|s|!s.is_empty()),
        "drsActive":drs
    });
    data.extend(common.as_object().unwrap().clone());
    Some(data)
}

pub struct AcSnapshot {
    pub data: Data,
    pub status: &'static str,
    pub error: Option<String>,
}
impl AcSnapshot {
    fn waiting() -> Self {
        Self {
            data: Data::new(),
            status: "WAITING",
            error: None,
        }
    }
    fn error(error: String) -> Self {
        Self {
            data: Data::new(),
            status: "ERROR",
            error: Some(error),
        }
    }
}
#[derive(Default)]
pub struct Tracker {
    counters: Option<(i32, i32)>,
    changed_at: u64,
}
impl Tracker {
    pub fn update(
        &mut self,
        physics: &[u8],
        graphics: &[u8],
        statics: &[u8],
        now: u64,
    ) -> AcSnapshot {
        let Some(data) = parse(physics, graphics, statics) else {
            return AcSnapshot::error("AC Shared Memory layout is incomplete".into());
        };
        let status = i32::from_le_bytes(graphics[4..8].try_into().unwrap());
        if status == 0 {
            return AcSnapshot::waiting();
        }
        if !(1..=3).contains(&status) {
            return AcSnapshot::error(format!("Unknown AC status {status}"));
        }
        let counters = (
            i32::from_le_bytes(physics[..4].try_into().unwrap()),
            i32::from_le_bytes(graphics[..4].try_into().unwrap()),
        );
        if self.counters != Some(counters) {
            self.counters = Some(counters);
            self.changed_at = now;
        }
        if now.saturating_sub(self.changed_at) >= 2500 {
            return AcSnapshot::waiting();
        }
        AcSnapshot {
            data,
            status: match status {
                1 => "REPLAY",
                3 => "PAUSED",
                _ => "LIVE",
            },
            error: None,
        }
    }
}

#[cfg(windows)]
mod receiver {
    use super::*;
    use crate::named_memory::ReadMapping;
    use std::time::{Duration, Instant};
    pub struct AcReceiver {
        names: [String; 3],
        pages: Option<[ReadMapping; 3]>,
        tracker: Tracker,
        start: Instant,
        retry_at: Instant,
        check_game_process: bool,
        game_running: bool,
        check_process_at: Instant,
        connection_error: Option<String>,
    }
    impl AcReceiver {
        pub fn new() -> Result<Self, String> {
            let mut receiver = Self::with_names([
                "Local\\acpmf_physics".into(),
                "Local\\acpmf_graphics".into(),
                "Local\\acpmf_static".into(),
            ]);
            receiver.check_game_process = true;
            Ok(receiver)
        }
        #[doc(hidden)]
        pub fn with_names(names: [String; 3]) -> Self {
            Self {
                names,
                pages: None,
                tracker: Tracker::default(),
                start: Instant::now(),
                retry_at: Instant::now(),
                check_game_process: false,
                game_running: false,
                check_process_at: Instant::now(),
                connection_error: None,
            }
        }
        fn connect(&self) -> Result<Option<[ReadMapping; 3]>, String> {
            let mut pages = Vec::new();
            for (name, page) in self.names.iter().zip(packets()) {
                match ReadMapping::open(name, page.size)? {
                    Some(mapping) => pages.push(mapping),
                    None => return Ok(None),
                }
            }
            Ok(Some(pages.try_into().ok().unwrap()))
        }
        pub fn snapshot(&mut self) -> AcSnapshot {
            if self.check_game_process {
                if Instant::now() >= self.check_process_at {
                    self.check_process_at = Instant::now() + Duration::from_millis(500);
                    match game_running() {
                        Ok(running) => self.game_running = running,
                        Err(error) => return AcSnapshot::error(error),
                    }
                }
                if !self.game_running {
                    self.pages = None;
                    self.tracker = Tracker::default();
                    self.connection_error = None;
                    return AcSnapshot::waiting();
                }
            }
            if self.pages.is_none() {
                if Instant::now() < self.retry_at {
                    return self
                        .connection_error
                        .clone()
                        .map(AcSnapshot::error)
                        .unwrap_or_else(AcSnapshot::waiting);
                }
                self.retry_at = Instant::now() + Duration::from_millis(500);
                match self.connect() {
                    Ok(pages) => {
                        self.pages = pages;
                        self.connection_error = None;
                    }
                    Err(e) => {
                        self.connection_error = Some(e.clone());
                        return AcSnapshot::error(e);
                    }
                }
            }
            let Some(pages) = &self.pages else {
                return AcSnapshot::waiting();
            };
            let copies: Option<Vec<_>> = pages
                .iter()
                .enumerate()
                .map(|(i, page)| page.copy(i < 2))
                .collect();
            let Some(copies) = copies else {
                return AcSnapshot::waiting();
            };
            let snapshot = self.tracker.update(
                &copies[0],
                &copies[1],
                &copies[2],
                self.start.elapsed().as_millis() as u64,
            );
            if snapshot.status == "WAITING" {
                self.pages = None;
            }
            snapshot
        }
    }
    fn game_running() -> Result<bool, String> {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            System::Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
                TH32CS_SNAPPROCESS,
            },
        };
        // AC and ACC share mapping names. Require the original AC simulator
        // executable so another game's pages cannot be misidentified as AC.
        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut available = unsafe { Process32FirstW(handle, &mut entry) } != 0;
        while available {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
            if name.eq_ignore_ascii_case("acs.exe") || name.eq_ignore_ascii_case("acs_x86.exe") {
                found = true;
                break;
            }
            available = unsafe { Process32NextW(handle, &mut entry) } != 0;
        }
        unsafe {
            CloseHandle(handle);
        }
        Ok(found)
    }
}
#[cfg(windows)]
pub use receiver::AcReceiver;
#[cfg(not(windows))]
pub struct AcReceiver;
#[cfg(not(windows))]
impl AcReceiver {
    pub fn new() -> Result<Self, String> {
        Err("Assetto Corsa Shared Memory requires Windows".into())
    }
    pub fn snapshot(&mut self) -> AcSnapshot {
        AcSnapshot::error("Assetto Corsa Shared Memory requires Windows".into())
    }
}
