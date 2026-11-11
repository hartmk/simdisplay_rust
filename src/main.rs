#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use simdisplay::{app::DisplayApp, model::Layout};
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|s| s == "--export-default")
        && let Some(path) = args.get(i + 1)
    {
        simdisplay::storage::atomic_write(
            std::path::Path::new(path),
            &serde_json::to_vec_pretty(&Layout::default()).unwrap(),
        )
        .expect("export default layout");
        return Ok(());
    }
    let path = if let Some(i) = args.iter().position(|s| s == "--data-dir") {
        std::path::PathBuf::from(args.get(i + 1).expect("--data-dir needs a directory"))
            .join("workspace.json")
    } else {
        directories::ProjectDirs::from("local", "Apex", "SimdisplayRust")
            .expect("application data directory")
            .data_dir()
            .join("workspace.json")
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Apex Display · Rust")
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .expect("bundled icon"),
            )
            .with_inner_size([1440., 900.])
            .with_min_inner_size([1000., 680.]),
        ..Default::default()
    };
    eframe::run_native(
        "Apex Display · Rust",
        options,
        Box::new(move |cc| Ok(Box::new(DisplayApp::new(&cc.egui_ctx, path)))),
    )
}
