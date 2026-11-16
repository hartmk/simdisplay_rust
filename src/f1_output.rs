//! Packed F1 25 output fields derived from the supplied EA v3 structures.
use crate::{model::Widget, telemetry::Data};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Deserialize)]
pub struct OutputPacket {
    pub id: u8,
    pub size: usize,
    pub group: String,
    pub fields: Vec<OutputField>,
}
#[derive(Deserialize)]
pub struct OutputField {
    pub key: String,
    pub label: String,
    pub offset: usize,
    pub kind: String,
    pub dimensions: Vec<Dimension>,
    pub unit: String,
    pub description: String,
    #[serde(default)]
    pub text_len: usize,
    pub states: HashMap<String, String>,
    pub event: Option<String>,
}
#[derive(Deserialize)]
pub struct Dimension {
    pub count: usize,
    pub stride: usize,
    pub label: String,
    #[serde(default)]
    pub player: bool,
    pub valid_count_offset: Option<usize>,
}
pub fn packets() -> &'static [OutputPacket] {
    static PACKETS: OnceLock<Vec<OutputPacket>> = OnceLock::new();
    PACKETS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/f1-2025-fields.json"))
            .expect("bundled F1 output metadata")
    })
}
pub fn field(key: &str) -> Option<&'static OutputField> {
    static FIELDS: OnceLock<HashMap<&str, &OutputField>> = OnceLock::new();
    FIELDS
        .get_or_init(|| {
            packets()
                .iter()
                .flat_map(|p| &p.fields)
                .map(|f| (f.key.as_str(), f))
                .collect()
        })
        .get(key)
        .copied()
}
pub fn catalog() -> impl Iterator<Item = (&'static str, &'static str, f64)> {
    packets()
        .iter()
        .flat_map(|p| &p.fields)
        .map(|f| (f.key.as_str(), f.label.as_str(), 64.))
}

pub fn decode(bytes: &[u8]) -> Option<Data> {
    if bytes.len() < 29
        || bytes[..2] != 2025u16.to_le_bytes()
        || bytes[2] != 25
        || bytes[5] != 1
        || bytes[27] >= 22
    {
        return None;
    }
    let packet = packets()
        .iter()
        .find(|p| p.id == bytes[6] && p.size == bytes.len())?;
    if packet.fields.iter().flat_map(|f| &f.dimensions).any(|dim| {
        dim.valid_count_offset
            .is_some_and(|o| bytes[o] as usize > dim.count)
    }) {
        return None;
    }
    // These packets cycle through every car. Keep the primary player's history/tyres.
    if matches!(packet.id, 11 | 12) && bytes[29] != bytes[27] {
        return None;
    }
    let mut data = Data::new();
    for f in &packet.fields {
        let applicable = f
            .event
            .as_ref()
            .is_none_or(|code| bytes.get(29..33) == Some(code.as_bytes()));
        data.insert(
            f.key.clone(),
            if applicable {
                read_dimensions(bytes, f, 0, f.offset)
            } else {
                Value::Null
            },
        );
    }
    data.insert("f1.playerCarIndex".into(), json!(bytes[27]));
    Some(data)
}
pub fn expiry_ms(key: &str) -> Option<u64> {
    if key.starts_with("f1.event.") || key.starts_with("f1.finalClassification.") {
        None // Sent only on events / once at the finish; reset with the session.
    } else if key.starts_with("f1.participants.") {
        Some(6500) // The game publishes participants only every five seconds.
    } else {
        Some(2500)
    }
}

pub fn demo() -> &'static Data {
    static DEMO: OnceLock<Data> = OnceLock::new();
    DEMO.get_or_init(|| {
        let mut data = Data::new();
        for packet in packets() {
            for field in &packet.fields {
                let value = if field.kind == "char" {
                    json!(if field.text_len == 4 {
                        "FTLP"
                    } else {
                        "Demo Driver"
                    })
                } else if !field.states.is_empty() {
                    json!(
                        field
                            .states
                            .keys()
                            .filter_map(|s| s.parse::<i64>().ok())
                            .min()
                            .unwrap_or(0)
                    )
                } else {
                    json!(match field.unit.as_str() {
                        "ms" => 93420.,
                        "°C" => 88.,
                        "PSI" => 25.,
                        "RPM" => 11200.,
                        "J" => 2500000.,
                        "W" => 120000.,
                        "kg" => 42.8,
                        "%" => 65.,
                        "km/h" => 285.,
                        "G" => 1.2,
                        _ => 1.,
                    })
                };
                let value = field
                    .dimensions
                    .iter()
                    .rev()
                    .fold(value, |v, dim| Value::Array(vec![v; dim.count]));
                data.insert(field.key.clone(), value);
            }
        }
        data.insert("f1.playerCarIndex".into(), json!(0));
        data
    })
}
fn read_dimensions(bytes: &[u8], field: &OutputField, level: usize, offset: usize) -> Value {
    if let Some(dim) = field.dimensions.get(level) {
        let valid = dim
            .valid_count_offset
            .map(|o| bytes[o + offset - field.offset] as usize)
            .unwrap_or(dim.count)
            .min(dim.count);
        return Value::Array(
            (0..dim.count)
                .map(|i| {
                    if i < valid {
                        read_dimensions(bytes, field, level + 1, offset + i * dim.stride)
                    } else {
                        Value::Null
                    }
                })
                .collect(),
        );
    }
    // Checked slices keep even malformed metadata/input from reading out of bounds.
    let n = match field.kind.as_str() {
        "char" => {
            return bytes
                .get(offset..offset + field.text_len)
                .map(|s| {
                    let end = s.iter().position(|b| *b == 0).unwrap_or(s.len());
                    json!(String::from_utf8_lossy(&s[..end]))
                })
                .unwrap_or(Value::Null);
        }
        "uint8" | "NEuint8" => return bytes.get(offset).map(|n| json!(*n)).unwrap_or(Value::Null),
        "int8" => {
            return bytes
                .get(offset)
                .map(|n| json!(*n as i8))
                .unwrap_or(Value::Null);
        }
        "uint16" => bytes
            .get(offset..offset + 2)
            .map(|s| u16::from_le_bytes(s.try_into().unwrap()) as f64),
        "int16" => bytes
            .get(offset..offset + 2)
            .map(|s| i16::from_le_bytes(s.try_into().unwrap()) as f64),
        "uint32" | "uint" | "NEuint" => bytes
            .get(offset..offset + 4)
            .map(|s| u32::from_le_bytes(s.try_into().unwrap()) as f64),
        "float" => bytes
            .get(offset..offset + 4)
            .map(|s| f32::from_le_bytes(s.try_into().unwrap()) as f64),
        "double" => bytes
            .get(offset..offset + 8)
            .map(|s| f64::from_le_bytes(s.try_into().unwrap())),
        _ => None,
    };
    n.filter(|n| n.is_finite())
        .map(|n| json!(n))
        .unwrap_or(Value::Null)
}

impl OutputField {
    pub fn value<'a>(&self, data: &'a Data, widget: &Widget) -> Option<&'a Value> {
        let mut value = data.get(&self.key)?;
        for (i, dim) in self.dimensions.iter().enumerate() {
            let index = widget
                .output_indices
                .get(i)
                .copied()
                .flatten()
                .unwrap_or_else(|| {
                    if dim.player {
                        data.get("f1.playerCarIndex")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as usize
                    } else {
                        0
                    }
                });
            value = value.as_array()?.get(index)?;
        }
        (!value.is_null()).then_some(value)
    }
    pub fn text(&self, value: Option<&Value>) -> String {
        match value {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(v) if v.is_number() => {
                let n = v.as_f64().unwrap();
                if let Some(state) = self.states.get(&(n as i64).to_string()) {
                    return state.clone();
                }
                if self.unit == "ms" && !self.key.contains("MSPart") && n >= 0. {
                    return crate::model::format_time(Some(n));
                }
                if matches!(self.kind.as_str(), "float" | "double") {
                    format!("{n:.2}")
                } else {
                    format!("{n:.0}")
                }
            }
            _ => "—".into(),
        }
    }
}
