use crate::f1::{Packet, parse_f1};
use serde_json::{Map, Value, json};
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
pub type Data = Map<String, Value>;
#[derive(Default)]
pub struct TelemetryState {
    session: Option<u64>,
    fields: HashMap<String, (Value, u64)>,
    last_packet: Option<u64>,
}
impl TelemetryState {
    pub fn ingest(&mut self, p: Packet, now: u64) {
        if self.session != Some(p.session) {
            self.fields.clear();
            self.session = Some(p.session);
        }
        for (k, v) in p.data {
            self.fields.insert(k, (v, now));
        }
        self.last_packet = Some(now);
    }
    pub fn snapshot(&self, now: u64) -> Data {
        self.fields
            .iter()
            .filter(|(_, (_, t))| now.saturating_sub(*t) < 2500)
            .map(|(k, (v, _))| (k.clone(), v.clone()))
            .collect()
    }
    pub fn live(&self, now: u64) -> bool {
        self.last_packet
            .is_some_and(|t| now.saturating_sub(t) < 2500)
    }
}
pub struct Snapshot {
    pub data: Data,
    pub live: bool,
    pub error: Option<String>,
}
pub struct Receiver {
    state: Arc<Mutex<TelemetryState>>,
    error: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    start: Instant,
    address: SocketAddr,
}
impl Receiver {
    pub fn bind(address: &str) -> Result<Self, String> {
        let socket = UdpSocket::bind(address).map_err(|e| format!("UDP {address}: {e}"))?;
        socket
            .set_read_timeout(Some(Duration::from_millis(50)))
            .map_err(|e| e.to_string())?;
        let address = socket.local_addr().map_err(|e| e.to_string())?;
        let state = Arc::new(Mutex::new(TelemetryState::default()));
        let error = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let start = Instant::now();
        let (s, e, quit) = (state.clone(), error.clone(), stop.clone());
        let worker = thread::spawn(move || {
            let mut bytes = [0u8; 65536];
            while !quit.load(Ordering::Relaxed) {
                match socket.recv_from(&mut bytes) {
                    Ok((n, _)) => {
                        if let Some(p) = parse_f1(&bytes[..n]) {
                            s.lock()
                                .unwrap()
                                .ingest(p, start.elapsed().as_millis() as u64);
                        }
                    }
                    Err(err)
                        if matches!(
                            err.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(err) => {
                        *e.lock().unwrap() = Some(err.to_string());
                        break;
                    }
                }
            }
        });
        Ok(Self {
            state,
            error,
            stop,
            worker: Some(worker),
            start,
            address,
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    pub fn snapshot(&self) -> Snapshot {
        let now = self.start.elapsed().as_millis() as u64;
        let s = self.state.lock().unwrap();
        Snapshot {
            data: s.snapshot(now),
            live: s.live(now),
            error: self.error.lock().unwrap().clone(),
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.worker.take() {
            let _ = t.join();
        }
    }
}
pub fn demo(t: f64) -> Data {
    let speed = (180. + 108. * (t / 5.).sin()).round();
    json!({"drsActive":u8::from((t/5.).sin()>0.6),"tractionControl":1,"abs":1,"speed":speed,"gear":((speed/42.).floor()+1.).clamp(2.,8.),"rpm":(8000.+3400.*((t*1.3).sin()+1.)/2.).round(),"maxRpm":12500,"lapTime":(t*1000.)%93420.,"lastLap":93420,"bestLap":91874,"tyres":[88.+3.*(t/8.).sin(),90.+2.*(t/7.).sin(),86.+2.*(t/9.).sin(),87.+3.*(t/7.).sin()],"brakeTemps":[129.+20.*(t/5.).sin(),128.+18.*(t/5.).sin(),76.+10.*(t/5.).sin(),76.+9.*(t/5.).sin()],"tyrePressures":[25.1,24.9,24.5,24.4],"tyreWear":[0,0,0,0],"tyreCompound":"Dry compound","fuel":(42.8-t*0.003).max(0.),"fuelUnit":"L","position":4,"lap":12.+(t/93.42).floor(),"throttle":(65.+35.*(t/5.).sin()).round(),"brake":(-(t/5.).sin()).max(0.)*75.,"sessionTime":(2400.-t).max(0.)}).as_object().unwrap().clone()
}
