use crate::model::{ColorStop, Widget};
use crate::widgets::color;
use eframe::egui::{self, Ui};
pub fn color_edit(ui: &mut Ui, label: &str, s: &mut String) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut c = color(s);
        if ui.color_edit_button_srgba(&mut c).changed() {
            *s = format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
        }
    });
}
pub fn widget_inspector(
    ui: &mut Ui,
    w: &mut Widget,
    width: u32,
    height: u32,
    rpm_error: &mut String,
) {
    ui.heading(&w.kind);
    ui.label("ชื่อ / ข้อความ");
    ui.text_edit_singleline(&mut w.label);
    if w.label.encode_utf16().count() > 120 {
        w.label = crate::model::truncate_utf16(&w.label, 120);
    }
    ui.separator();
    ui.label("POSITION & SIZE");
    ui.checkbox(&mut w.locked, "ล็อกตำแหน่ง");
    ui.add_enabled_ui(!w.locked, |ui| {
        egui::Grid::new("geometry").num_columns(2).show(ui, |ui| {
            for (label, v, max) in [
                ("X", &mut w.x, width as f64),
                ("Y", &mut w.y, height as f64),
                ("Width", &mut w.w, width as f64),
                ("Height", &mut w.h, height as f64),
            ] {
                ui.label(label);
                ui.add(egui::DragValue::new(v).range(0. ..=max).speed(1.));
                ui.end_row();
            }
        });
    });
    w.constrain(width, height);
    ui.separator();
    ui.label("APPEARANCE");
    color_edit(ui, "ตัวอักษร", &mut w.color);
    color_edit(ui, "พื้นหลัง", &mut w.background);
    ui.add(egui::Slider::new(&mut w.background_opacity, 0. ..=100.).text("Opacity %"));
    egui::ComboBox::from_id_salt("font")
        .selected_text(&w.font)
        .show_ui(ui, |ui| {
            for f in ["sans", "mono", "condensed"] {
                ui.selectable_value(&mut w.font, f.into(), f);
            }
        });
    ui.add(egui::Slider::new(&mut w.font_size, 16. ..=300.).text("Font size"));
    let mut enabled = w.threshold.is_some();
    if ui.checkbox(&mut enabled, "เปลี่ยนสีเมื่อค่า ≥ เกณฑ์").changed()
    {
        w.threshold = if enabled { Some(0.) } else { None };
    }
    if let Some(t) = &mut w.threshold {
        ui.add(egui::DragValue::new(t).speed(1.));
        color_edit(ui, "สีเตือน", &mut w.warning_color);
        ui.small("เวลารอบใช้มิลลิวินาที");
    }
    if let Some(config) = &mut w.rpm_config {
        ui.separator();
        ui.label("RPM COLORS");
        let old = config.clone();
        let old_unit = config.unit.clone();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut config.unit, "percent".into(), "% Max");
            ui.selectable_value(&mut config.unit, "rpm".into(), "RPM");
        });
        if old_unit != config.unit {
            for s in &mut config.stops {
                s.at = if config.unit == "rpm" {
                    s.at * config.max_rpm / 100.
                } else {
                    (s.at / config.max_rpm * 100.).min(100.)
                };
            }
        }
        ui.checkbox(&mut config.use_game_max, "ใช้รอบสูงสุดจากเกม");
        ui.horizontal(|ui| {
            ui.label("Max / fallback");
            ui.add(
                egui::DragValue::new(&mut config.max_rpm)
                    .range(1. ..=100000.)
                    .speed(100.),
            );
        });
        egui::ComboBox::from_id_salt("rpm-mode")
            .selected_text(&config.color_mode)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut config.color_mode, "segments".into(), "แยกสีแต่ละช่วง");
                ui.selectable_value(&mut config.color_mode, "current".into(), "สีตามรอบปัจจุบัน");
            });
        let mut remove = None;
        let len = config.stops.len();
        for (i, s) in config.stops.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        i != 0,
                        egui::DragValue::new(&mut s.at)
                            .range(
                                0. ..=if config.unit == "percent" {
                                    100.
                                } else {
                                    100000.
                                },
                            )
                            .speed(1.),
                    );
                    color_edit(ui, "", &mut s.color);
                    if ui
                        .add_enabled(i != 0 && len > 1, egui::Button::new("×"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            config.stops.remove(i);
        }
        if ui
            .add_enabled(config.stops.len() < 12, egui::Button::new("+ ช่วงสี"))
            .clicked()
        {
            let last = config.stops.last().unwrap().at;
            let max = if config.unit == "percent" {
                100.
            } else {
                100000.
            };
            if last < max {
                config.stops.push(ColorStop {
                    at: (last + if config.unit == "percent" { 5. } else { 500. }).min(max),
                    color: "#ff654e".into(),
                });
            }
        }
        if let Err(e) = config.validate() {
            *config = old;
            *rpm_error = e;
        } else if *config != old {
            rpm_error.clear();
        }
        if !rpm_error.is_empty() {
            ui.colored_label(egui::Color32::LIGHT_RED, rpm_error.as_str());
        }
        ui.small("ปรับแล้วบันทึกอัตโนมัติ · ช่วงแรกเริ่มที่ 0");
    }
}
