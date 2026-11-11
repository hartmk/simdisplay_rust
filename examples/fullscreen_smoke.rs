//! Native regression check: click the race-display button, then Escape.
//! Uses an isolated workspace and injects events only into this app.
use eframe::egui::{self, Event, Key, PointerButton, pos2};
use simdisplay::app::DisplayApp;
use std::time::{Duration, Instant};
struct Smoke {
    app: DisplayApp,
    start: Instant,
    pressed: bool,
    released: bool,
    entered: bool,
    escaped: bool,
}
impl eframe::App for Smoke {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if self.start.elapsed() > Duration::from_millis(500) && !self.released {
            let p = pos2(input.screen_rect.map_or(1440., |r| r.width()) - 55., 23.);
            input.events.push(Event::PointerMoved(p));
            input.events.push(Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed: !self.pressed,
                modifiers: Default::default(),
            });
            if self.pressed {
                self.released = true;
            }
            self.pressed = true;
        }
        if self.entered && self.start.elapsed() > Duration::from_secs(3) && !self.escaped {
            input.events.push(Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            });
            self.escaped = true;
        }
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.app.show(ctx);
        let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
        if fullscreen && !self.entered {
            self.entered = true;
            println!("entered native fullscreen");
        }
        if self.escaped && !fullscreen && self.start.elapsed() > Duration::from_secs(5) {
            println!("Escape restored window; UI remained responsive");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
fn main() -> eframe::Result<()> {
    // Also catches a blocked native event loop, unlike an in-loop timeout.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(12));
        eprintln!("native fullscreen smoke check timed out");
        std::process::exit(1);
    });
    let data = tempfile::tempdir().unwrap();
    eframe::run_native(
        "Fullscreen regression check",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1440., 900.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            Ok(Box::new(Smoke {
                app: DisplayApp::new(&cc.egui_ctx, data.path().join("workspace.json")),
                start: Instant::now(),
                pressed: false,
                released: false,
                entered: false,
                escaped: false,
            }))
        }),
    )
}
