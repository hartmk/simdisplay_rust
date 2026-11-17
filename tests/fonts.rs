use eframe::egui::{self, FontFamily};
use simdisplay::{fonts, widgets};

#[test]
fn missing_system_font_renders_with_fallback_and_retains_selection() {
    let ctx = egui::Context::default();
    widgets::configure_fonts(&ctx);
    let name = "system:SimDisplay-DefinitelyMissing-TestFont";
    for _ in 0..3 {
        let _ = ctx.run(Default::default(), |ctx| {
            assert_eq!(fonts::family(ctx, name), FontFamily::Proportional);
            assert!(fonts::unavailable(ctx, name));
        });
    }
}

#[test]
fn installed_font_loads_on_demand_and_other_widget_fonts_stay_independent() {
    let Some(face) = fonts::installed().iter().find(|face| {
        face.label.contains("Arial")
            || face.label.contains("DejaVuSans")
            || face.label.contains("LiberationSans")
    }) else {
        // Minimal CI images need not have system fonts installed.
        return;
    };
    let ctx = egui::Context::default();
    widgets::configure_fonts(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        assert_eq!(fonts::family(ctx, &face.key), FontFamily::Proportional);
    });
    let output = ctx.run(Default::default(), |ctx| {
        let family = fonts::family(ctx, &face.key);
        assert_eq!(family, FontFamily::Name(face.key.clone().into()));
        assert_eq!(fonts::family(ctx, "sans"), FontFamily::Proportional);
        assert_eq!(fonts::family(ctx, "mono"), FontFamily::Monospace);
        assert!(!fonts::unavailable(ctx, &face.key));
        let layout = simdisplay::model::Layout::default();
        let mut widget = simdisplay::model::Widget::new("lastLap", &layout);
        widget.font = face.key.clone();
        let data = serde_json::json!({"lastLap":90000,"bestLap":89000,"lapTime":30000})
            .as_object()
            .unwrap()
            .clone();
        widgets::paint_widget(
            &ctx.layer_painter(egui::LayerId::background()),
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500., 300.)),
            &widget,
            &data,
            1.,
        );
        ctx.fonts_mut(|fonts| {
            let text = fonts.layout_no_wrap(
                "123 RPM ภาษาไทย".into(),
                egui::FontId::new(24., family),
                egui::Color32::WHITE,
            );
            assert!(text.size().x > 0.);
        });
    });
    let mut text_count = 0;
    for shape in output.shapes {
        if let egui::Shape::Text(text) = shape.shape {
            text_count += 1;
            for section in &text.galley.job.sections {
                assert_eq!(
                    section.format.font_id.family,
                    FontFamily::Name(face.key.clone().into())
                );
            }
        }
    }
    assert!(
        text_count >= 6,
        "lap row labels and values must all use the selected font"
    );
}
