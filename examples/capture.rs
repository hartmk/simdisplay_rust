//! Render this app's framebuffer to PNG for visual regression checks.
use eframe::egui;
use simdisplay::app::DisplayApp;
struct Capture {
    app: DisplayApp,
    frames: usize,
    path: String,
}
impl eframe::App for Capture {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.app.show(ctx);
        self.frames += 1;
        if self.frames == 30 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        for e in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = e {
                image::save_buffer(
                    &self.path,
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}
fn main() -> eframe::Result<()> {
    let path = std::env::args().nth(1).expect("PNG output path");
    let dimensions: Vec<u32> = std::env::args()
        .skip(2)
        .map(|s| s.parse().expect("canvas dimension"))
        .collect();
    let data = tempfile::tempdir().unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1440., 900.]),
        ..Default::default()
    };
    eframe::run_native(
        "Apex Display capture",
        options,
        Box::new(move |cc| {
            let mut app = DisplayApp::new(&cc.egui_ctx, data.path().join("state.json"));
            if dimensions.len() == 2 {
                app.editor
                    .resize(dimensions[0], dimensions[1])
                    .expect("valid canvas size");
            }
            if let Ok(preset) = std::env::var("SIMDISPLAY_CAPTURE_STYLE") {
                for w in &mut app.editor.layout.widgets {
                    w.apply_style_preset(&preset);
                }
                app.editor.selected = app
                    .editor
                    .layout
                    .widgets
                    .iter()
                    .find(|w| w.kind == "speed")
                    .map(|w| w.id.clone());
            }
            Ok(Box::new(Capture {
                app,
                frames: 0,
                path,
            }))
        }),
    )
}
