use crate::{
    model::{Widget, format_time},
    telemetry::Data,
};
use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Painter, Rect, Stroke, StrokeKind, pos2, vec2,
};
pub fn color(s: &str) -> Color32 {
    if crate::model::valid_color(s) {
        Color32::from_rgb(
            u8::from_str_radix(&s[1..3], 16).unwrap(),
            u8::from_str_radix(&s[3..5], 16).unwrap(),
            u8::from_str_radix(&s[5..7], 16).unwrap(),
        )
    } else {
        Color32::WHITE
    }
}
pub fn number(d: &Data, k: &str) -> Option<f64> {
    d.get(k).and_then(|v| v.as_f64()).filter(|v| v.is_finite())
}
fn numeric(v: Option<f64>, digits: usize) -> String {
    v.filter(|n| *n >= 0.)
        .map(|n| format!("{n:.digits$}"))
        .unwrap_or("—".into())
}
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "thai".into(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSansThai.ttf")).into(),
    );
    fonts.font_data.insert(
        "condensed".into(),
        egui::FontData::from_static(include_bytes!(
            "../assets/fonts/BarlowCondensed-SemiBold.ttf"
        ))
        .into(),
    );
    for names in fonts.families.values_mut() {
        names.push("thai".into());
    }
    fonts.families.insert(
        FontFamily::Name("condensed".into()),
        vec!["condensed".into(), "thai".into()],
    );
    ctx.set_fonts(fonts);
}
fn font(w: &Widget, size: f32) -> FontId {
    FontId::new(
        size.max(1.),
        match w.font.as_str() {
            "mono" => FontFamily::Monospace,
            "condensed" => FontFamily::Name("condensed".into()),
            _ => FontFamily::Proportional,
        },
    )
}
fn fitted(p: &Painter, r: Rect, text: &str, f: FontId, c: Color32) {
    fitted_aligned(p, r, text, f, c, Align2::CENTER_CENTER);
}
fn fitted_aligned(p: &Painter, r: Rect, text: &str, mut f: FontId, c: Color32, align: Align2) {
    if r.width() <= 0. || r.height() <= 0. {
        return;
    }
    let g = p.layout_no_wrap(text.into(), f.clone(), c);
    let factor = (r.width() / g.size().x.max(1.))
        .min(r.height() / g.size().y.max(1.))
        .min(1.);
    f.size *= factor;
    let at = pos2(
        match align.x() {
            egui::Align::Min => r.left(),
            egui::Align::Max => r.right(),
            _ => r.center().x,
        },
        match align.y() {
            egui::Align::Min => r.top(),
            egui::Align::Max => r.bottom(),
            _ => r.center().y,
        },
    );
    p.text(at, align, text, f, c);
}
fn alignment(w: &Widget) -> Align2 {
    Align2([
        match w.style.horizontal.as_str() {
            "left" => egui::Align::Min,
            "right" => egui::Align::Max,
            _ => egui::Align::Center,
        },
        match w.style.vertical.as_str() {
            "top" => egui::Align::Min,
            "bottom" => egui::Align::Max,
            _ => egui::Align::Center,
        },
    ])
}
fn value_with_unit(
    p: &Painter,
    r: Rect,
    w: &Widget,
    text: &str,
    unit: &str,
    c: Color32,
    scale: f32,
) {
    let mut value_rect = r;
    let unit_height = ((w.style.unit_size as f32 + 2.) * scale).min(r.height() * 0.3);
    if w.style.show_unit && !unit.is_empty() {
        value_rect.max.y -= unit_height;
    }
    fitted_aligned(
        p,
        value_rect,
        text,
        font(w, w.font_size as f32 * scale),
        c,
        alignment(w),
    );
    if w.style.show_unit && !unit.is_empty() {
        let unit_rect = Rect::from_min_max(pos2(r.left(), value_rect.bottom()), r.max);
        fitted_aligned(
            p,
            unit_rect,
            unit,
            font(w, w.style.unit_size as f32 * scale),
            color(&w.style.unit_color),
            Align2([alignment(w).x(), egui::Align::Max]),
        );
    }
}
pub fn paint_widget(p: &Painter, r: Rect, w: &Widget, d: &Data, scale: f32) {
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    let bg = color(&w.background);
    p.rect_filled(
        r,
        (w.style.radius as f32 * scale).min(r.width().min(r.height()) / 2.),
        Color32::from_rgba_unmultiplied(
            bg.r(),
            bg.g(),
            bg.b(),
            (w.background_opacity * 2.55).round() as u8,
        ),
    );
    if w.style.border_width > 0. {
        p.rect_stroke(
            r,
            (w.style.radius as f32 * scale).min(r.width().min(r.height()) / 2.),
            Stroke::new(
                w.style.border_width as f32 * scale,
                color(&w.style.border_color),
            ),
            StrokeKind::Inside,
        );
    }
    let margin = ((w.style.padding + w.style.border_width) as f32 * scale)
        .min((r.width().min(r.height()) * 0.5 - 1.).max(0.));
    let inner = r.shrink(margin);
    let title_h = if w.style.show_label {
        ((w.style.label_size as f32 + 8.) * scale).min(inner.height() * 0.25)
    } else {
        0.
    };
    let bottom_label = w.style.label_position == "bottom";
    let title = Rect::from_min_max(
        pos2(
            inner.left(),
            if bottom_label {
                inner.bottom() - title_h
            } else {
                inner.top()
            },
        ),
        pos2(
            inner.right(),
            if bottom_label {
                inner.bottom()
            } else {
                inner.top() + title_h
            },
        ),
    );
    if w.style.show_label {
        fitted_aligned(
            &p,
            title,
            &w.label,
            font(w, w.style.label_size as f32 * scale),
            color(&w.style.label_color),
            Align2::LEFT_CENTER,
        );
    }
    let body = Rect::from_min_max(
        pos2(
            inner.left(),
            inner.top() + if bottom_label { 0. } else { title_h },
        ),
        pos2(
            inner.right(),
            inner.bottom() - if bottom_label { title_h } else { 0. },
        ),
    );
    // Keep the legacy `lastLap` widget as the combined three-row display.
    let key = if w.kind == "lastLapTime" {
        "lastLap"
    } else {
        &w.kind
    };
    let value = number(d, key);
    let base = if w.threshold.is_some_and(|t| value.is_some_and(|n| n >= t)) {
        color(&w.warning_color)
    } else {
        color(&w.color)
    };
    match w.kind.as_str() {
        "tyres" => paint_tyres(&p, body, w, d),
        "rpm" | "rpmNumber" => {
            let state = w.rpm_state(number(d, "rpm"), number(d, "maxRpm"));
            if w.kind == "rpm" {
                let height = body.height();
                let gap = (5. * scale).min(body.width() / 60.);
                let width = (body.width() - gap * 23.) / 24.;
                for (i, c) in state.bars.iter().enumerate() {
                    let cell = Rect::from_min_size(
                        body.min + vec2(i as f32 * (width + gap), 0.),
                        vec2(width, height),
                    );
                    if w.style.rpm_shape == "led" {
                        p.circle_filled(cell.center(), width.min(height) * 0.5, color(c));
                    } else {
                        p.rect_filled(
                            cell,
                            if w.style.rpm_shape == "square" {
                                0.
                            } else {
                                width.min(height) * 0.18
                            },
                            color(c),
                        );
                    }
                }
            } else {
                fitted_aligned(
                    &p,
                    body,
                    &numeric(number(d, "rpm"), 0),
                    font(w, w.font_size as f32 * scale),
                    color(&state.color),
                    alignment(w),
                );
            }
        }
        "lastLap" => {
            for (i, (label, key)) in [
                ("Last lap", "lastLap"),
                ("Best lap", "bestLap"),
                ("Current lap", "lapTime"),
            ]
            .iter()
            .enumerate()
            {
                let row = Rect::from_min_size(
                    body.min + vec2(0., body.height() * i as f32 / 3.),
                    vec2(body.width(), body.height() / 3.),
                );
                let label_rect = Rect::from_min_max(
                    row.min,
                    pos2(row.left() + row.width() * 0.34, row.bottom()),
                );
                fitted(
                    &p,
                    label_rect,
                    label,
                    FontId::proportional(20. * scale),
                    Color32::GRAY,
                );
                let value_rect = Rect::from_min_max(pos2(label_rect.right(), row.top()), row.max);
                let n = number(d, key);
                let c = if w.threshold.is_some_and(|t| n.is_some_and(|v| v >= t)) {
                    color(&w.warning_color)
                } else {
                    base
                };
                fitted_aligned(
                    &p,
                    value_rect,
                    &format_time(n),
                    font(w, w.font_size as f32 * scale * 0.65),
                    c,
                    alignment(w),
                );
            }
        }
        "throttle" | "brake" => {
            let bar_h = (10. * scale).min(body.height() * 0.15);
            let bar = Rect::from_min_size(
                pos2(body.left(), body.bottom() - bar_h),
                vec2(body.width(), bar_h),
            );
            p.rect_filled(bar, 2., Color32::from_gray(40));
            if let Some(n) = value {
                p.rect_filled(
                    Rect::from_min_size(
                        bar.min,
                        vec2(bar.width() * n.clamp(0., 100.) as f32 / 100., bar.height()),
                    ),
                    2.,
                    base,
                );
            }
            let mut text_rect = body;
            text_rect.max.y -= (bar_h + 4. * scale).min(body.height() * 0.25);
            value_with_unit(&p, text_rect, w, &numeric(value, 0), "%", base, scale);
        }
        _ => {
            let text = match w.kind.as_str() {
                "drsActive" => match value {
                    Some(0.) => "OFF",
                    Some(1.) => "ACTIVE",
                    _ => "—",
                }
                .into(),
                "tractionControl" => match value {
                    Some(0.) => "OFF",
                    Some(1.) => "MEDIUM",
                    Some(2.) => "FULL",
                    _ => "—",
                }
                .into(),
                "abs" => match value {
                    Some(0.) => "OFF",
                    Some(1.) => "ON",
                    _ => "—",
                }
                .into(),
                "gear" => match value {
                    Some(v) if v < 0. => "R".into(),
                    Some(0.) => "N".into(),
                    _ => numeric(value, 0),
                },
                "lapTime" | "bestLap" | "lastLapTime" => format_time(value),
                "sessionTime" => value
                    .map(|v| format!("{}:{:02}", v.max(0.) as u64 / 60, v.max(0.) as u64 % 60))
                    .unwrap_or("—".into()),
                "fuel" => numeric(value, 1),
                "text" => w.label.clone(),
                "speed" => numeric(value, 0),
                _ => numeric(value, 0),
            };
            let unit = match w.kind.as_str() {
                "speed" => "km/h",
                "fuel" => d.get("fuelUnit").and_then(|v| v.as_str()).unwrap_or(""),
                _ => "",
            };
            value_with_unit(&p, body, w, &text, unit, base, scale);
        }
    }
}
fn paint_tyres(p: &Painter, r: Rect, w: &Widget, d: &Data) {
    let compact = r.width() / r.height().max(1.) > 2.2;
    let size = if compact {
        vec2(880., 210.)
    } else {
        vec2(440., 470.)
    };
    let s = (r.width() / size.x).min(r.height() / size.y);
    let origin = r.center() - size * s / 2.;
    let at = |x: f32, y: f32| origin + vec2(x, y) * s;
    let text = |x: f32, y: f32, t: String, base: f32, c: Color32, max: f32| {
        let rr = Rect::from_center_size(at(x, y), vec2(max * s, base * 1.4 * s));
        fitted(p, rr, &t, font(w, base * s * (w.font_size as f32 / 68.)), c);
    };
    let array = |k: &str, i: usize| d.get(k).and_then(|v| v.get(i)).and_then(|v| v.as_f64());
    for (i, name) in ["FL", "FR", "RL", "RR"].iter().enumerate() {
        let right = i % 2 == 1;
        let rear = i > 1;
        let shift = if compact && rear { 440. } else { 0. };
        let x = if right { 245. } else { 125. } + shift;
        let y = if compact {
            40.
        } else if rear {
            280.
        } else {
            55.
        };
        let outer = if right { 375. } else { 65. } + shift;
        let available = array("tyres", i).is_some();
        let tyre = Rect::from_min_size(at(x, y), vec2(70., 132.) * s);
        p.rect(
            tyre,
            egui::CornerRadius::same((17. * s).clamp(0., 255.) as u8),
            if available {
                Color32::from_rgb(7, 170, 95)
            } else {
                Color32::from_gray(50)
            },
            Stroke::new(s, Color32::from_rgb(165, 210, 190)),
            StrokeKind::Inside,
        );
        for n in 0..6 {
            p.line_segment(
                [
                    at(x + 12. + n as f32 * 9., y + 8.),
                    at(x + 12. + n as f32 * 9., y + 124.),
                ],
                Stroke::new(2. * s, Color32::from_rgba_unmultiplied(190, 255, 220, 55)),
            );
        }
        text(
            x + 35.,
            y + 70.,
            format!("{}%", numeric(array("tyreWear", i), 0)),
            32.,
            Color32::WHITE,
            65.,
        );
        text(
            outer,
            y + 24.,
            format!("{}°", numeric(array("tyres", i), 0)),
            44.,
            Color32::from_rgb(23, 239, 131),
            112.,
        );
        text(
            outer,
            y + 83.,
            numeric(array("tyrePressures", i), 1),
            40.,
            Color32::from_rgb(255, 228, 82),
            112.,
        );
        text(
            outer,
            y + 114.,
            "PSI".into(),
            20.,
            Color32::LIGHT_GRAY,
            110.,
        );
        text(
            x + 35.,
            if compact {
                y + 153.
            } else if rear {
                y + 165.
            } else {
                y - 24.
            },
            format!("{}°C", numeric(array("brakeTemps", i), 0)),
            28.,
            Color32::from_rgb(103, 239, 249),
            108.,
        );
        text(outer, y - 14., (*name).into(), 16., Color32::GRAY, 100.);
    }
    text(
        if compact { 440. } else { 220. },
        if compact { 190. } else { 235. },
        d.get("tyreCompound")
            .and_then(|v| v.as_str())
            .unwrap_or("—")
            .into(),
        if compact { 18. } else { 26. },
        Color32::WHITE,
        if compact { 180. } else { 320. },
    );
}
