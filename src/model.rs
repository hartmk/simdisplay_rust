use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CATALOG: [(&str, &str, f64); 22] = [
    ("rpmNumber", "RPM number", 100.),
    ("gear", "Gear", 180.),
    ("speed", "Speed", 112.),
    ("rpm", "Engine RPM", 70.),
    ("lapTime", "Current lap", 88.),
    ("bestLap", "Best lap", 88.),
    ("lastLapTime", "Last lap", 88.),
    ("lastLap", "Lap times", 80.),
    ("tyres", "Tyre temperature", 68.),
    ("fuel", "Fuel remaining", 76.),
    ("ersEnergy", "ERS battery energy", 76.),
    ("electricPower", "Electric motor power", 76.),
    ("ersDeployMode", "ERS deployment mode", 64.),
    ("position", "Position", 92.),
    ("lap", "Lap number", 92.),
    ("throttle", "Throttle", 72.),
    ("brake", "Brake", 72.),
    ("drsActive", "DRS active", 76.),
    ("tractionControl", "Traction control", 76.),
    ("abs", "ABS", 76.),
    ("sessionTime", "Session remaining", 76.),
    ("text", "Custom text", 58.),
];
pub fn catalog() -> impl Iterator<Item = (&'static str, &'static str, f64)> {
    CATALOG.into_iter().chain(crate::f1_output::catalog())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub name: String,
    pub background: String,
    pub widgets: Vec<Widget>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Widget {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub color: String,
    pub background: String,
    pub background_opacity: f64,
    pub font: String,
    pub font_size: f64,
    #[serde(default)]
    pub style: WidgetStyle,
    pub threshold: Option<f64>,
    pub warning_color: String,
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_indices: Vec<Option<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm_config: Option<RpmConfig>,
}
/// Presentation only: telemetry, geometry and warning rules remain independent.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct WidgetStyle {
    pub radius: f64,
    pub border_width: f64,
    pub border_color: String,
    pub padding: f64,
    pub show_label: bool,
    pub show_unit: bool,
    pub horizontal: String,
    pub vertical: String,
    pub label_size: f64,
    pub label_color: String,
    pub unit_size: f64,
    pub unit_color: String,
    pub label_position: String,
    pub rpm_shape: String,
}
impl Default for WidgetStyle {
    fn default() -> Self {
        Self {
            radius: 5.,
            border_width: 0.,
            border_color: "#627582".into(),
            padding: 4.,
            show_label: true,
            show_unit: true,
            horizontal: "center".into(),
            vertical: "center".into(),
            label_size: 14.,
            label_color: "#97aab5".into(),
            unit_size: 16.,
            unit_color: "#a0a0a0".into(),
            label_position: "top".into(),
            rpm_shape: "rounded".into(),
        }
    }
}
impl WidgetStyle {
    pub fn validate(&self) -> Result<(), String> {
        for (n, min, max) in [
            (self.radius, 0., 100.),
            (self.border_width, 0., 20.),
            (self.padding, 0., 100.),
            (self.label_size, 6., 100.),
            (self.unit_size, 6., 100.),
        ] {
            if !n.is_finite() || !(min..=max).contains(&n) {
                return Err("ขนาดตกแต่งวิดเจ็ตไม่ถูกต้อง".into());
            }
        }
        if ![&self.border_color, &self.label_color, &self.unit_color]
            .iter()
            .all(|c| valid_color(c))
            || !["left", "center", "right"].contains(&self.horizontal.as_str())
            || !["top", "center", "bottom"].contains(&self.vertical.as_str())
            || !["top", "bottom"].contains(&self.label_position.as_str())
            || !["rounded", "square", "led"].contains(&self.rpm_shape.as_str())
        {
            return Err("รูปแบบตกแต่งวิดเจ็ตไม่ถูกต้อง".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RpmConfig {
    pub unit: String,
    pub max_rpm: f64,
    pub use_game_max: bool,
    pub color_mode: String,
    pub stops: Vec<ColorStop>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ColorStop {
    pub at: f64,
    pub color: String,
}
pub struct RpmState {
    pub color: String,
    pub lit: usize,
    pub bars: Vec<String>,
}
pub fn valid_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|c| c.is_ascii_hexdigit())
}
impl Default for RpmConfig {
    fn default() -> Self {
        Self {
            unit: "percent".into(),
            max_rpm: 12500.,
            use_game_max: true,
            color_mode: "segments".into(),
            stops: vec![
                ColorStop {
                    at: 0.,
                    color: "#c4f562".into(),
                },
                ColorStop {
                    at: 67.,
                    color: "#ffca6a".into(),
                },
                ColorStop {
                    at: 88.,
                    color: "#ff654e".into(),
                },
            ],
        }
    }
}
impl RpmConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !["rpm", "percent"].contains(&self.unit.as_str())
            || !["segments", "current"].contains(&self.color_mode.as_str())
            || !self.max_rpm.is_finite()
            || self.max_rpm <= 0.
            || self.max_rpm > 100000.
            || self.stops.is_empty()
            || self.stops.len() > 12
        {
            return Err("ตั้งค่า RPM ไม่ถูกต้อง".into());
        }
        let mut prev = -1.;
        for s in &self.stops {
            if !s.at.is_finite()
                || s.at < 0.
                || s.at <= prev
                || (self.unit == "percent" && s.at > 100.)
                || !valid_color(&s.color)
            {
                return Err("ช่วงสี RPM ต้องเรียง ไม่ซ้ำ และอยู่ในช่วงที่กำหนด".into());
            }
            prev = s.at;
        }
        if self.stops[0].at != 0. {
            return Err("ช่วงแรกต้องเริ่มที่ 0".into());
        }
        Ok(())
    }
}
impl Default for Layout {
    fn default() -> Self {
        let mut l = Self::parse(include_str!("../assets/default-layout.json"))
            .expect("bundled layout is valid");
        for w in &mut l.widgets {
            w.id = uuid::Uuid::new_v4().to_string();
        }
        l
    }
}
impl Layout {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.encode_utf16().count() > 1_000_000 {
            return Err("ไฟล์ใหญ่เกินไป".into());
        }
        let mut v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let o = v.as_object_mut().ok_or("รูปแบบเลย์เอาต์ไม่ถูกต้อง")?;
        if !o.contains_key("width") && !o.contains_key("height") {
            o.insert("width".into(), 1920.into());
            o.insert("height".into(), 1080.into());
        }
        // JavaScript treats 1920 and 1920.0 as the same integer.
        for key in ["version", "width", "height"] {
            if let Some(n) = o.get(key).and_then(|v| v.as_f64())
                && n.is_finite()
                && n >= 0.
                && n <= u32::MAX as f64
                && n.fract() == 0.
            {
                o.insert(key.into(), serde_json::Value::from(n as u32));
            }
        }
        let ws = o
            .get_mut("widgets")
            .and_then(|v| v.as_array_mut())
            .ok_or("ไม่มี widgets")?;
        for w in ws {
            let w = w.as_object_mut().ok_or("ข้อมูลวิดเจ็ตไม่ถูกต้อง")?;
            // Required legacy nullable field must be present, even when null.
            if !w.contains_key("threshold") {
                return Err("ไม่มี threshold".into());
            }
            w.entry("backgroundOpacity").or_insert(100.into());
            if w.get("type").and_then(|v| v.as_str()) == Some("lastLap")
                && w.get("label").and_then(|v| v.as_str()) == Some("Last lap")
            {
                w.insert("label".into(), "Lap times".into());
            }
            if matches!(
                w.get("type").and_then(|v| v.as_str()),
                Some("rpm" | "rpmNumber")
            ) && !w.contains_key("rpmConfig")
            {
                let mut c = RpmConfig::default();
                if let Some(color) = w.get("color").and_then(|v| v.as_str()) {
                    c.stops[0].color = color.into();
                }
                if let Some(t) = w.get("threshold").and_then(|v| v.as_f64()) {
                    c.unit = "rpm".into();
                    let warning = w
                        .get("warningColor")
                        .and_then(|v| v.as_str())
                        .unwrap_or("#ff654e")
                        .to_owned();
                    c.stops = if t <= 0. {
                        vec![ColorStop {
                            at: 0.,
                            color: warning,
                        }]
                    } else {
                        vec![
                            c.stops[0].clone(),
                            ColorStop {
                                at: t,
                                color: warning,
                            },
                        ]
                    };
                }
                w.insert("rpmConfig".into(), serde_json::to_value(c).unwrap());
            }
        }
        let l: Self = serde_json::from_value(v).map_err(|e| e.to_string())?;
        l.validate()?;
        Ok(l)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.name.encode_utf16().count() > 80
            || !valid_color(&self.background)
            || self.widgets.len() > 100
        {
            return Err("รูปแบบเลย์เอาต์ไม่ถูกต้อง".into());
        }
        validate_size(self.width, self.height)?;
        let mut ids = HashSet::new();
        for w in &self.widgets {
            w.style.validate()?;
            if let Some(field) = crate::f1_output::field(&w.kind) {
                if w.output_indices.len() > field.dimensions.len()
                    || w.output_indices
                        .iter()
                        .enumerate()
                        .any(|(i, index)| index.is_some_and(|n| n >= field.dimensions[i].count))
                {
                    return Err("ลำดับข้อมูล Telemetry ไม่ถูกต้อง".into());
                }
            } else if !w.output_indices.is_empty() {
                return Err("วิดเจ็ตนี้ไม่มีรายการ Telemetry ให้เลือก".into());
            }
            if !ids.insert(&w.id)
                || w.id.encode_utf16().count() > 80
                || w.label.encode_utf16().count() > 120
                || !catalog().any(|c| c.0 == w.kind)
                || !["sans", "mono", "condensed"].contains(&w.font.as_str())
                || ![&w.color, &w.background, &w.warning_color]
                    .iter()
                    .all(|s| valid_color(s))
                || !w.font_size.is_finite()
                || !(16. ..=300.).contains(&w.font_size)
                || !w.background_opacity.is_finite()
                || !(0. ..=100.).contains(&w.background_opacity)
                || w.threshold.is_some_and(|t| !t.is_finite())
            {
                return Err("ข้อมูลวิดเจ็ตไม่ถูกต้อง".into());
            }
            if ![w.x, w.y, w.w, w.h].iter().all(|n| n.is_finite()) {
                return Err("พิกัดไม่ถูกต้อง".into());
            }
            let mut constrained = w.clone();
            constrained.constrain(self.width, self.height);
            if [w.x, w.y, w.w, w.h] != [constrained.x, constrained.y, constrained.w, constrained.h]
            {
                return Err("วิดเจ็ตอยู่นอกพื้นที่หน้าจอ".into());
            }
            if matches!(w.kind.as_str(), "rpm" | "rpmNumber") {
                w.rpm_config.as_ref().ok_or("ไม่มี RPM config")?.validate()?;
            }
        }
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        validate_size(width, height)?;
        let sx = width as f64 / self.width as f64;
        let sy = height as f64 / self.height as f64;
        for w in &mut self.widgets {
            w.x *= sx;
            w.y *= sy;
            w.w *= sx;
            w.h *= sy;
            w.constrain(width, height);
            if w.kind != "tyres" {
                w.font_size = (w.font_size * width.min(height) as f64
                    / self.width.min(self.height) as f64)
                    .round()
                    .clamp(16., 300.);
            }
        }
        self.width = width;
        self.height = height;
        Ok(())
    }
}
pub fn validate_size(w: u32, h: u32) -> Result<(), String> {
    if !(320..=7680).contains(&w) || !(240..=4320).contains(&h) {
        Err("ขนาดจอ: กว้าง 320–7680 สูง 240–4320".into())
    } else {
        Ok(())
    }
}
impl Widget {
    pub fn apply_style_preset(&mut self, preset: &str) {
        match preset {
            "wheel" => {
                self.style = WidgetStyle {
                    radius: 0.,
                    border_width: 1.,
                    border_color: "#81929c".into(),
                    padding: 3.,
                    label_size: 24.,
                    unit_size: 24.,
                    label_color: "#d4dce0".into(),
                    rpm_shape: "square".into(),
                    ..WidgetStyle::default()
                };
                self.background = "#080c10".into();
                self.background_opacity = 100.;
                self.color = "#f5f7fa".into();
                self.font = "condensed".into();
            }
            "minimal" => {
                self.style = WidgetStyle {
                    radius: 0.,
                    padding: 0.,
                    show_label: false,
                    show_unit: false,
                    ..WidgetStyle::default()
                };
                self.background_opacity = 0.;
                self.color = "#f5f7fa".into();
                self.font = "mono".into();
            }
            "card" => {
                self.style = WidgetStyle {
                    radius: 18.,
                    padding: 16.,
                    border_width: 1.,
                    border_color: "#33424e".into(),
                    show_label: false,
                    show_unit: false,
                    ..WidgetStyle::default()
                };
                self.background = "#151e27".into();
                self.background_opacity = 100.;
                self.color = "#f5f7fa".into();
                self.font = "mono".into();
            }
            _ => {}
        }
    }
    pub fn copy_style_from(&mut self, source: &Widget) {
        self.style = source.style.clone();
        self.color = source.color.clone();
        self.background = source.background.clone();
        self.background_opacity = source.background_opacity;
        self.font = source.font.clone();
    }

    pub fn new(kind: &str, l: &Layout) -> Self {
        let (_, label, size) = catalog().find(|c| c.0 == kind).expect("catalog widget");
        let mut w = Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            label: label.into(),
            x: 80.,
            y: 80.,
            w: 420.,
            h: if kind == "tyres" { 520. } else { 240. },
            color: "#f0f4f5".into(),
            background: "#151b20".into(),
            background_opacity: 100.,
            font: "sans".into(),
            font_size: size,
            style: WidgetStyle::default(),
            threshold: None,
            warning_color: "#ff654e".into(),
            locked: false,
            output_indices: Vec::new(),
            rpm_config: if matches!(kind, "rpm" | "rpmNumber") {
                Some(RpmConfig::default())
            } else {
                None
            },
        };
        w.constrain(l.width, l.height);
        w
    }
    pub fn constrain(&mut self, width: u32, height: u32) {
        let min_w = (160. * width as f64 / 1920.).round().clamp(48., 160.);
        let min_h = (100. * height as f64 / 1080.).round().clamp(32., 100.);
        self.w = self.w.round().clamp(min_w, width as f64);
        self.h = self.h.round().clamp(min_h, height as f64);
        self.x = self.x.round().clamp(0., width as f64 - self.w);
        self.y = self.y.round().clamp(0., height as f64 - self.h);
    }
    pub fn rpm_state(&self, rpm: Option<f64>, game_max: Option<f64>) -> RpmState {
        let fallback = RpmConfig::default();
        let c = self.rpm_config.as_ref().unwrap_or(&fallback);
        let max = if c.use_game_max {
            game_max
                .filter(|n| n.is_finite() && *n > 0.)
                .unwrap_or(c.max_rpm)
        } else {
            c.max_rpm
        };
        let rpm = rpm.filter(|n| n.is_finite() && *n >= 0.);
        let percent = rpm.unwrap_or(0.) / max * 100.;
        let color_at = |value: f64| {
            c.stops
                .iter()
                .rev()
                .find(|s| value >= s.at)
                .map(|s| s.color.clone())
                .unwrap_or(self.color.clone())
        };
        let color = if let Some(n) = rpm {
            color_at(if c.unit == "rpm" { n } else { percent })
        } else {
            self.color.clone()
        };
        let lit = (percent / 100. * 24.).ceil().clamp(0., 24.) as usize;
        let bars = (0..24)
            .map(|i| {
                if i >= lit {
                    "#2c343b".into()
                } else if c.color_mode == "current" {
                    color.clone()
                } else {
                    color_at(i as f64 / 24. * if c.unit == "percent" { 100. } else { max })
                }
            })
            .collect();
        RpmState { color, lit, bars }
    }
}
pub fn format_time(ms: Option<f64>) -> String {
    match ms.filter(|n| n.is_finite() && *n >= 0.) {
        Some(n) => {
            let n = n as u64;
            format!("{}:{:02}.{:03}", n / 60000, n / 1000 % 60, n % 1000)
        }
        None => "—".into(),
    }
}

/// Bound user text without splitting a Unicode scalar or discarding a valid prefix.
pub fn truncate_utf16(text: &str, max: usize) -> String {
    let mut units = 0;
    text.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= max
        })
        .collect()
}
