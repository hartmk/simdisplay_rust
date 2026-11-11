use eframe::egui::{Pos2, Vec2, pos2, vec2};
#[derive(Clone, Debug)]
pub struct Monitor {
    pub id: u32,
    pub label: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub primary: bool,
}
pub fn enumerate() -> Result<Vec<Monitor>, String> {
    display_info::DisplayInfo::all()
        .map(|ds| {
            ds.into_iter()
                .enumerate()
                .map(|(i, d)| Monitor {
                    id: d.id,
                    label: format!(
                        "Display {} · {} × {}{}",
                        i + 1,
                        d.width,
                        d.height,
                        if d.is_primary { " · Primary" } else { "" }
                    ),
                    x: d.x as f32,
                    y: d.y as f32,
                    width: d.width as f32,
                    height: d.height as f32,
                    scale: d.scale_factor,
                    primary: d.is_primary,
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}
impl Monitor {
    pub fn position(&self, pixels_per_point: f32, zoom: f32) -> Pos2 {
        let factor = if cfg!(target_os = "macos") {
            zoom
        } else {
            pixels_per_point
        };
        pos2(self.x / factor, self.y / factor)
    }
    pub fn window_size(&self, pixels_per_point: f32, zoom: f32) -> Vec2 {
        let factor = if cfg!(target_os = "macos") {
            zoom
        } else {
            pixels_per_point
        };
        vec2(self.width / factor, self.height / factor)
    }
}
/// Window geometry in desktop coordinates (macOS points, Windows physical pixels).
/// Convert only when issuing a command, because the race monitor may have another DPI.
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    origin: Pos2,
    size: Vec2,
}
impl Placement {
    pub fn capture(
        outer: eframe::egui::Rect,
        inner_size: Vec2,
        desktop_units_per_point: f32,
    ) -> Self {
        Self {
            origin: outer.min * desktop_units_per_point,
            size: inner_size * desktop_units_per_point,
        }
    }
    pub fn commands(&self, monitors: &[Monitor], desktop_units_per_point: f32) -> (Pos2, Vec2) {
        let title = eframe::egui::Rect::from_min_size(self.origin, vec2(self.size.x, 32.));
        let monitor = monitors
            .iter()
            .find(|m| {
                let bounds =
                    eframe::egui::Rect::from_min_size(pos2(m.x, m.y), vec2(m.width, m.height));
                let visible = bounds.intersect(title);
                visible.width() >= 100. && visible.height() >= 20.
            })
            .or_else(|| monitors.iter().find(|m| m.primary))
            .or(monitors.first());
        let (origin, size) = if let Some(m) = monitor {
            let size = vec2(
                self.size.x.min(m.width),
                self.size.y.min((m.height - 40.).max(100.)),
            );
            let origin = pos2(
                self.origin.x.clamp(m.x, m.x + (m.width - size.x).max(0.)),
                self.origin
                    .y
                    .clamp(m.y, m.y + (m.height - size.y - 32.).max(0.)),
            );
            (origin, size)
        } else {
            (pos2(30., 30.), vec2(1000., 680.))
        };
        let factor = desktop_units_per_point.max(0.1);
        (origin / factor, size / factor)
    }
}
pub fn desktop_units_per_point(pixels_per_point: f32, zoom: f32) -> f32 {
    if cfg!(target_os = "macos") {
        zoom
    } else {
        pixels_per_point
    }
}
