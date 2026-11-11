use serde_json::{Map, Value, json};
#[derive(Debug)]
pub struct Packet {
    pub session: u64,
    pub data: Map<String, Value>,
}
pub fn parse_f1(b: &[u8]) -> Option<Packet> {
    if b.len() < 29 {
        return None;
    }
    let u16_at = |o| u16::from_le_bytes([b[o], b[o + 1]]);
    let u32_at = |o| u32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let float = |o| f32::from_le_bytes(b[o..o + 4].try_into().unwrap()) as f64;
    if u16_at(0) != 2025 || b[2] != 25 || b[5] != 1 || b[27] >= 22 {
        return None;
    }
    let player = b[27] as usize;
    let wheels = [2, 3, 0, 1];
    let d = match (b[6], b.len()) {
        (6, 1352) => {
            let o = 29 + player * 60;
            let throttle = float(o + 2) * 100.;
            let brake = float(o + 10) * 100.;
            let pressures = wheels.map(|i| float(o + 40 + i * 4));
            if !throttle.is_finite()
                || !brake.is_finite()
                || !pressures.iter().all(|n| n.is_finite())
            {
                return None;
            }
            json!({"speed":u16_at(o),"gear":b[o+15] as i8,"rpm":u16_at(o+16),"drsActive":enum_value(b[o+18],1),"throttle":throttle,"brake":brake,"tyres":wheels.map(|i|b[o+30+i]),"brakeTemps":wheels.map(|i|u16_at(o+22+i*2)),"tyrePressures":pressures})
        }
        (2, 1285) => {
            let o = 29 + player * 57;
            json!({"lastLap":nonzero(u32_at(o)),"lapTime":u32_at(o+4),"position":nonzero(b[o+32] as u32),"lap":nonzero(b[o+33] as u32)})
        }
        (7, 1239) => {
            let o = 29 + player * 55;
            let fuel = float(o + 5);
            if !fuel.is_finite() {
                return None;
            }
            let compound = match b[o + 26] {
                7 => Some("Intermediate"),
                8 => Some("Wet"),
                16 => Some("Soft"),
                17 => Some("Medium"),
                18 => Some("Hard"),
                _ => None,
            };
            json!({"tractionControl":enum_value(b[o],2),"abs":enum_value(b[o+1],1),"fuel":fuel,"fuelUnit":"kg","maxRpm":u16_at(o+17),"tyreCompound":compound})
        }
        (10, 1041) => {
            let o = 29 + player * 46;
            let wear = wheels.map(|i| float(o + i * 4));
            if !wear.iter().all(|n| n.is_finite()) {
                return None;
            }
            json!({"tyreWear":wear})
        }
        (11, 1460) => {
            if b[29] as usize != player || b[30] > 100 {
                return None;
            }
            let lap = b[32] as usize;
            let best = if lap > 0 && lap <= b[30] as usize {
                let o = 36 + (lap - 1) * 14;
                if b[o + 13] & 1 != 0 {
                    nonzero(u32_at(o))
                } else {
                    None
                }
            } else {
                None
            };
            json!({"bestLap":best})
        }
        (1, 753) => json!({"sessionTime":u16_at(38)}),
        _ => return None,
    };
    Some(Packet {
        session: u64::from_le_bytes(b[7..15].try_into().unwrap()),
        data: d.as_object().unwrap().clone(),
    })
}
fn nonzero(n: u32) -> Option<u32> {
    if n == 0 { None } else { Some(n) }
}

// Unknown codes explicitly clear stale values instead of claiming OFF.
fn enum_value(value: u8, max: u8) -> Option<u8> {
    (value <= max).then_some(value)
}
