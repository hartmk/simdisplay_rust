use crate::{
    editor::Editor,
    inspector::{color_edit, widget_inspector},
    model::{CATALOG, Layout},
    monitors::{self, Monitor, Placement},
    storage::{Saver, Store, Workspace, atomic_write},
    telemetry::{self, Data, Receiver},
    widgets::{self, color, paint_widget},
};
use eframe::egui::{
    self, Align2, Color32, FontId, Key, Rect, Sense, Stroke, StrokeKind, Vec2, ViewportCommand,
    pos2, vec2,
};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
struct Drag {
    id: String,
    start: [f64; 4],
    resize: bool,
}
enum FileResult {
    Import(Result<Layout, String>),
    Export(Result<(), String>),
}
pub struct DisplayApp {
    pub editor: Editor,
    workspace: Workspace,
    saver: Saver,
    save_status: String,
    storage_blocked: bool,
    source: String,
    receiver: Option<Receiver>,
    connection_error: String,
    start: Instant,
    search: String,
    grid: bool,
    preview: bool,
    drag: Option<Drag>,
    resize_handle_focus: bool,
    rpm_error: String,
    style_clipboard: Option<crate::model::Widget>,
    canvas_width: u32,
    canvas_height: u32,
    last_canvas_size: (u32, u32),
    monitors: Vec<Monitor>,
    monitor_id: Option<u32>,
    last_monitors: Instant,
    presenting: bool,
    pending_fullscreen: Option<Instant>,
    restore: Option<Placement>,
    pending_restore: Option<Instant>,
    file_tx: mpsc::Sender<FileResult>,
    file_rx: mpsc::Receiver<FileResult>,
    file_busy: bool,
    message: String,
    last_saved: Workspace,
    dirty_since: Option<Instant>,
    retry_after: Instant,
}
impl DisplayApp {
    pub fn new(ctx: &egui::Context, path: PathBuf) -> Self {
        widgets::configure_fonts(ctx);
        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals(egui::Visuals::dark());
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = vec2(8., 8.);
        style.visuals.panel_fill = Color32::from_rgb(17, 23, 29);
        style.visuals.window_fill = Color32::from_rgb(17, 23, 29);
        style.visuals.selection.bg_fill = Color32::from_rgb(67, 91, 39);
        style.visuals.selection.stroke = Stroke::new(1_f32, Color32::from_rgb(196, 245, 98));
        ctx.set_style(style);
        let (store, workspace) = Store::load(path);
        let error = store.error.clone();
        let editor = Editor::new(workspace.active.clone());
        let (file_tx, file_rx) = mpsc::channel();
        let monitors = monitors::enumerate().unwrap_or_default();
        let monitor_id = monitors
            .iter()
            .find(|m| m.primary)
            .or(monitors.first())
            .map(|m| m.id);
        Self {
            canvas_width: editor.layout.width,
            canvas_height: editor.layout.height,
            last_canvas_size: (editor.layout.width, editor.layout.height),
            editor,
            last_saved: workspace.clone(),
            workspace,
            saver: Saver::new(store),
            save_status: if error.is_some() {
                "หยุดบันทึก: ไฟล์เดิมมีปัญหา".into()
            } else {
                "บันทึกอัตโนมัติ".into()
            },
            storage_blocked: error.is_some(),
            source: "demo".into(),
            receiver: None,
            connection_error: String::new(),
            start: Instant::now(),
            search: String::new(),
            grid: true,
            preview: false,
            drag: None,
            resize_handle_focus: false,
            rpm_error: String::new(),
            style_clipboard: None,
            monitors,
            monitor_id,
            last_monitors: Instant::now(),
            presenting: false,
            pending_fullscreen: None,
            restore: None,
            pending_restore: None,
            file_tx,
            file_rx,
            file_busy: false,
            message: error.unwrap_or_default(),
            dirty_since: None,
            retry_after: Instant::now(),
        }
    }
    fn set_source(&mut self, source: String) {
        self.receiver = None;
        self.connection_error.clear();
        self.source = source;
        if self.source == "f1" {
            match Receiver::bind("127.0.0.1:20777") {
                Ok(r) => self.receiver = Some(r),
                Err(e) => self.connection_error = e,
            }
        }
    }
    fn telemetry(&mut self) -> (Data, &'static str) {
        if self.source == "demo" {
            return (telemetry::demo(self.start.elapsed().as_secs_f64()), "DEMO");
        }
        if self.source != "f1" {
            return (Data::new(), "NOT READY");
        }
        if let Some(r) = &self.receiver {
            let s = r.snapshot();
            if let Some(e) = s.error {
                self.connection_error = e;
            }
            return (
                s.data,
                if !self.connection_error.is_empty() {
                    "ERROR"
                } else if s.live {
                    "LIVE"
                } else {
                    "WAITING"
                },
            );
        }
        (
            Data::new(),
            if self.connection_error.is_empty() {
                "WAITING"
            } else {
                "ERROR"
            },
        )
    }
    fn import(&mut self) {
        if self.file_busy {
            return;
        }
        self.file_busy = true;
        let tx = self.file_tx.clone();
        std::thread::spawn(move || {
            let result = if let Some(p) = rfd::FileDialog::new()
                .add_filter("Layout JSON", &["json"])
                .pick_file()
            {
                (|| {
                    use std::io::Read;
                    let f = std::fs::File::open(p).map_err(|e| e.to_string())?;
                    let mut s = String::new();
                    f.take(4_000_001)
                        .read_to_string(&mut s)
                        .map_err(|e| e.to_string())?;
                    Layout::parse(&s)
                })()
            } else {
                Err("ยกเลิกการนำเข้า".into())
            };
            let _ = tx.send(FileResult::Import(result));
        });
    }
    fn export(&mut self) {
        if self.file_busy {
            return;
        }
        self.file_busy = true;
        let tx = self.file_tx.clone();
        let l = self.editor.layout.clone();
        std::thread::spawn(move || {
            let result = if let Some(p) = rfd::FileDialog::new()
                .add_filter("Layout JSON", &["json"])
                .set_file_name("dashboard.json")
                .save_file()
            {
                (|| {
                    l.validate()?;
                    let b = serde_json::to_vec_pretty(&l).map_err(|e| e.to_string())?;
                    atomic_write(&p, &b)
                })()
            } else {
                Err("ยกเลิกการส่งออก".into())
            };
            let _ = tx.send(FileResult::Export(result));
        });
    }
    fn launch(&mut self, ctx: &egui::Context) {
        if let Some(m) = self.monitors.iter().find(|m| Some(m.id) == self.monitor_id) {
            self.editor.end_edit();
            // Context::input holds egui's context lock. Read scale only after
            // releasing it: pixels_per_point/zoom_factor also acquire that lock.
            let bounds = ctx.input(|i| i.viewport().outer_rect.zip(i.viewport().inner_rect));
            self.restore = bounds.map(|(outer, inner)| {
                Placement::capture(
                    outer,
                    inner.size(),
                    monitors::desktop_units_per_point(ctx.pixels_per_point(), ctx.zoom_factor()),
                )
            });
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(
                m.position(ctx.pixels_per_point(), ctx.zoom_factor()),
            ));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(
                m.window_size(ctx.pixels_per_point(), ctx.zoom_factor()),
            ));
            self.presenting = true;
            self.pending_fullscreen = Some(Instant::now());
        }
    }
    fn exit_display(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        self.presenting = false;
        self.pending_fullscreen = None;
        self.pending_restore = Some(Instant::now());
        self.preview = false;
    }
    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.file_rx.try_recv() {
            self.file_busy = false;
            match result {
                FileResult::Import(Ok(l)) => {
                    self.canvas_width = l.width;
                    self.canvas_height = l.height;
                    self.editor.replace(l);
                    self.message = "นำเข้าเลย์เอาต์แล้ว".into();
                }
                FileResult::Import(Err(e)) | FileResult::Export(Err(e)) => self.message = e,
                FileResult::Export(Ok(())) => self.message = "ส่งออกเลย์เอาต์แล้ว".into(),
            }
        }
        while let Ok(result) = self.saver.results.try_recv() {
            match result.outcome {
                Ok(()) => {
                    self.last_saved = result.workspace;
                    if result.reset {
                        self.storage_blocked = false;
                    }
                    self.save_status = "บันทึกแล้ว".into();
                }
                Err(e) => {
                    self.save_status = "บันทึกไม่สำเร็จ · จะลองใหม่".into();
                    self.message = e;
                    self.retry_after = Instant::now() + Duration::from_secs(2);
                }
            }
        }
        if self.last_monitors.elapsed() > Duration::from_secs(2) {
            if let Ok(ms) = monitors::enumerate() {
                let missing = self
                    .monitor_id
                    .is_some_and(|id| !ms.iter().any(|m| m.id == id));
                self.monitors = ms;
                if missing {
                    self.monitor_id = self
                        .monitors
                        .iter()
                        .find(|m| m.primary)
                        .or(self.monitors.first())
                        .map(|m| m.id);
                    if self.presenting {
                        self.exit_display(ctx);
                        self.message = "จอถูกถอด: กลับมาที่จอหลัก".into();
                    }
                }
            }
            self.last_monitors = Instant::now();
        }
        if self
            .pending_restore
            .is_some_and(|t| t.elapsed() > Duration::from_millis(300))
            && !ctx.input(|i| i.viewport().fullscreen.unwrap_or(false))
        {
            if let Ok(ms) = monitors::enumerate() {
                self.monitors = ms;
            }
            if let Some(p) = self.restore.take() {
                let (pos, size) = p.commands(
                    &self.monitors,
                    monitors::desktop_units_per_point(ctx.pixels_per_point(), ctx.zoom_factor()),
                );
                ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
            }
            self.pending_restore = None;
        }
        if self
            .pending_fullscreen
            .is_some_and(|t| t.elapsed() > Duration::from_millis(250))
        {
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
            self.pending_fullscreen = None;
        }
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            if self.presenting {
                self.exit_display(ctx);
            } else {
                self.preview = false;
            }
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.presenting || self.preview {
            return;
        }
        let text_focus = ctx.wants_keyboard_input();
        if !text_focus {
            let (command, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
            if command && ctx.input(|i| i.key_pressed(Key::Z)) {
                if shift {
                    self.editor.redo();
                } else {
                    self.editor.undo();
                }
            }
            if command && ctx.input(|i| i.key_pressed(Key::D)) {
                self.editor.duplicate();
            }
            if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                self.editor.delete();
            }
            let step = if shift { 10. } else { 1. };
            for (key, x, y) in [
                (Key::ArrowLeft, -step, 0.),
                (Key::ArrowRight, step, 0.),
                (Key::ArrowUp, 0., -step),
                (Key::ArrowDown, 0., step),
            ] {
                if ctx.input(|i| i.key_pressed(key)) {
                    if self.resize_handle_focus {
                        self.editor.nudge_size(x, y);
                    } else {
                        self.editor.nudge(x, y);
                    }
                }
            }
        }
    }
    fn header(&mut self, ctx: &egui::Context, status: &str) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("A / APEX DISPLAY")
                        .size(22.)
                        .strong()
                        .color(Color32::from_rgb(196, 245, 98)),
                );
                ui.label("RUST STUDIO");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("↗ เปิดบนจอแข่ง").clicked() {
                        self.launch(ctx);
                    }
                    if ui.selectable_label(self.preview, "Preview").clicked() {
                        self.editor.end_edit();
                        self.preview = !self.preview;
                    }
                    ui.label(&self.save_status);
                });
            });
            ui.add_space(6.);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(status)
                        .strong()
                        .color(if status == "LIVE" {
                            Color32::LIGHT_GREEN
                        } else {
                            Color32::from_rgb(255, 202, 106)
                        }),
                );
                let previous = self.source.clone();
                egui::ComboBox::from_id_salt("source")
                    .selected_text(&self.source)
                    .show_ui(ui, |ui| {
                        for (id, label) in [
                            ("demo", "Demo / ข้อมูลจำลอง"),
                            ("f1", "F1 25 · UDP"),
                            ("ac", "Assetto Corsa · NOT READY"),
                            ("acc", "ACC · NOT READY"),
                            ("lmu", "Le Mans Ultimate · NOT READY"),
                            ("iracing", "iRacing · NOT READY"),
                        ] {
                            ui.selectable_value(&mut self.source, id.into(), label);
                        }
                    });
                if previous != self.source {
                    self.set_source(self.source.clone());
                }
                if self.source == "f1" {
                    ui.small("127.0.0.1:20777 · format 2025");
                    if ui.button("เชื่อมต่อใหม่").clicked() {
                        self.set_source("f1".into());
                    }
                }
                egui::ComboBox::from_id_salt("monitor")
                    .selected_text(
                        self.monitors
                            .iter()
                            .find(|m| Some(m.id) == self.monitor_id)
                            .map(|m| m.label.as_str())
                            .unwrap_or("ไม่พบจอ"),
                    )
                    .show_ui(ui, |ui| {
                        for m in &self.monitors {
                            ui.selectable_value(&mut self.monitor_id, Some(m.id), &m.label);
                        }
                    });
            });
            if !self.connection_error.is_empty() {
                ui.colored_label(Color32::LIGHT_RED, &self.connection_error);
            }
            ui.add_space(5.);
        });
    }
    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(4.);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.file_busy, egui::Button::new("Import JSON"))
                    .clicked()
                {
                    self.import();
                }
                if ui
                    .add_enabled(!self.file_busy, egui::Button::new("Export JSON"))
                    .clicked()
                {
                    self.export();
                }
                if ui.button("+ เก็บเลย์เอาต์").clicked() {
                    self.editor.end_edit();
                    self.workspace
                        .library
                        .insert(self.editor.layout.name.clone(), self.editor.layout.clone());
                    self.message = "เก็บเลย์เอาต์แล้ว (ชื่อซ้ำจะอัปเดต)".into();
                }
                let mut load = None;
                egui::ComboBox::from_id_salt("library")
                    .selected_text("เลย์เอาต์ที่บันทึก")
                    .show_ui(ui, |ui| {
                        for (name, l) in &self.workspace.library {
                            if ui.selectable_label(false, name).clicked() {
                                load = Some(l.clone());
                            }
                        }
                    });
                if let Some(l) = load {
                    self.canvas_width = l.width;
                    self.canvas_height = l.height;
                    self.editor.replace(l);
                }
                ui.small(format!(
                    "{} widgets · {} × {}",
                    self.editor.layout.widgets.len(),
                    self.editor.layout.width,
                    self.editor.layout.height
                ));
            });
            if !self.message.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&self.message);
                    if ui.small_button("ปิดข้อความ").clicked() {
                        self.message.clear();
                    }
                });
            }
            if self.storage_blocked {
                ui.colored_label(Color32::LIGHT_RED, "ไฟล์บันทึกมีปัญหา: ยังไม่เขียนทับข้อมูลเดิม");
                if ui.button("สำรองไฟล์เดิม แล้วเริ่มบันทึกใหม่").clicked()
                {
                    self.saver.reset_and_save(self.workspace.clone());
                }
            }
            ui.add_space(4.);
        });
    }
    fn library_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("widgets").exact_width(180.).show(ctx,|ui|{ui.add_space(10.);ui.heading("Widgets");ui.text_edit_singleline(&mut self.search);ui.separator();egui::ScrollArea::vertical().show(ui,|ui|{for(kind,label,_)in CATALOG{if label.to_lowercase().contains(&self.search.to_lowercase())&&ui.add_enabled(self.editor.layout.widgets.len()<100,egui::Button::new(label).min_size(vec2(158.,36.))).clicked(){self.editor.add(kind);}}ui.add_space(15.);ui.small("ลากชื่อเพื่อย้าย\nลากมุมเพื่อปรับขนาด\nลูกศร: 1 px\nShift + ลูกศร: 10 px\nCtrl/Cmd + Z: Undo");});});
    }
    fn inspector(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("inspector")
            .exact_width(265.)
            .show(ctx, |ui| {
                ui.add_space(10.);
                ui.heading("Inspector");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.editor.selected.is_some() {
                        ui.horizontal(|ui| {
                            if ui.button("ทำสำเนา").clicked() {
                                self.editor.duplicate();
                            }
                            if ui.button("ลบ").clicked() {
                                self.editor.delete();
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("ชั้นล่าง ↓").clicked() {
                                self.editor.layer(false);
                            }
                            if ui.button("ชั้นบน ↑").clicked() {
                                self.editor.layer(true);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.button("คัดลอกสไตล์").clicked() {
                                self.style_clipboard = self.editor.selected_mut().cloned();
                            }
                            if ui
                                .add_enabled(
                                    self.style_clipboard.is_some(),
                                    egui::Button::new("วางสไตล์"),
                                )
                                .on_hover_text("คัดลอกสี ฟอนต์ และการตกแต่ง โดยคงขนาดตัวเลขและข้อมูลเดิม")
                                .clicked()
                                && let Some(source) = self.style_clipboard.clone()
                            {
                                self.editor.paste_style(&source);
                            }
                        });
                        let (width, height) = (self.editor.layout.width, self.editor.layout.height);
                        self.editor.begin_edit();
                        if let Some(w) = self.editor.selected_mut() {
                            widget_inspector(ui, w, width, height, &mut self.rpm_error);
                        }
                        if !ctx.wants_keyboard_input() && !ctx.input(|i| i.pointer.primary_down()) {
                            self.editor.end_edit();
                        }
                    } else {
                        ui.label("เลือกวิดเจ็ตบน canvas เพื่อแก้ไข");
                    }
                    ui.separator();
                    ui.heading("Canvas");
                    self.editor.begin_edit();
                    color_edit(ui, "พื้นหลังจอ", &mut self.editor.layout.background);

                    egui::ComboBox::from_id_salt("canvas-preset")
                        .selected_text(format!("{} × {}", self.canvas_width, self.canvas_height))
                        .show_ui(ui, |ui| {
                            for (label, w, h) in [
                                ("Full HD", 1920, 1080),
                                ("11.3 inch · Landscape", 1920, 480),
                                ("11.3 inch · Portrait", 480, 1920),
                                ("QHD", 2560, 1440),
                            ] {
                                if ui
                                    .selectable_label(
                                        self.canvas_width == w && self.canvas_height == h,
                                        label,
                                    )
                                    .clicked()
                                {
                                    self.canvas_width = w;
                                    self.canvas_height = h;
                                }
                            }
                        });
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.canvas_width).range(320..=7680));
                        ui.label("×");
                        ui.add(egui::DragValue::new(&mut self.canvas_height).range(240..=4320));
                    });
                    if ui.button("ใช้ขนาดนี้").clicked()
                        && let Err(e) = self.editor.resize(self.canvas_width, self.canvas_height)
                    {
                        self.message = e;
                    }
                });
            });
    }
    fn canvas(&mut self, ui: &mut egui::Ui, data: &Data, status: &str) {
        if !self.presenting {
            ui.horizontal(|ui| {
                if !self.preview {
                    let before = self.editor.layout.clone();
                    self.editor.begin_edit();
                    ui.text_edit_singleline(&mut self.editor.layout.name);
                    if self.editor.layout.name.encode_utf16().count() > 80 {
                        self.editor.layout = before;
                    }
                    if !ui.ctx().wants_keyboard_input() && !ui.ctx().input(|i| i.pointer.any_down())
                    {
                        self.editor.end_edit();
                    }
                    if ui
                        .add_enabled(self.editor.history.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.editor.undo();
                    }
                    if ui
                        .add_enabled(self.editor.history.can_redo(), egui::Button::new("Redo"))
                        .clicked()
                    {
                        self.editor.redo();
                    }
                    ui.checkbox(&mut self.grid, "Grid");
                } else {
                    ui.label(&self.editor.layout.name);
                }
                ui.label(if self.preview { "PREVIEW" } else { "EDIT MODE" });
            });
        }
        let area = ui.available_rect_before_wrap();
        let pad = if self.presenting { 0. } else { 16. };
        let size = area.size() - Vec2::splat(pad * 2.);
        let scale = (size.x / self.editor.layout.width as f32)
            .min(size.y / self.editor.layout.height as f32)
            .max(0.001);
        let canvas_size = vec2(
            self.editor.layout.width as f32,
            self.editor.layout.height as f32,
        ) * scale;
        let canvas = Rect::from_center_size(area.center(), canvas_size);
        let response = ui.allocate_rect(area, Sense::click());
        if response.clicked() {
            self.editor.end_edit();
            self.editor.selected = None;
            ui.ctx().memory_mut(|m| m.stop_text_input());
        }
        let p = ui.painter().with_clip_rect(canvas);
        p.rect_filled(canvas, 0., color(&self.editor.layout.background));
        if self.grid && !self.preview && !self.presenting {
            for x in (0..self.editor.layout.width).step_by(80) {
                p.line_segment(
                    [
                        canvas.min + vec2(x as f32 * scale, 0.),
                        pos2(canvas.left() + x as f32 * scale, canvas.bottom()),
                    ],
                    Stroke::new(0.5_f32, Color32::from_gray(31)),
                );
            }
            for y in (0..self.editor.layout.height).step_by(80) {
                p.line_segment(
                    [
                        canvas.min + vec2(0., y as f32 * scale),
                        pos2(canvas.right(), canvas.top() + y as f32 * scale),
                    ],
                    Stroke::new(0.5_f32, Color32::from_gray(31)),
                );
            }
        }
        let widgets = self.editor.layout.widgets.clone();
        for w in &widgets {
            let r = Rect::from_min_size(
                canvas.min + vec2(w.x as f32, w.y as f32) * scale,
                vec2(w.w as f32, w.h as f32) * scale,
            );
            paint_widget(&p, r, w, data, scale);
            if !self.preview && !self.presenting {
                let selected = self.editor.selected.as_ref() == Some(&w.id);
                if selected {
                    p.rect_stroke(
                        r,
                        4.,
                        Stroke::new(1.5_f32, Color32::from_rgb(196, 245, 98)),
                        StrokeKind::Inside,
                    );
                }
                let click = ui.interact(r, ui.id().with((&w.id, "select")), Sense::click());
                if click.clicked() {
                    self.resize_handle_focus = false;
                    self.editor.end_edit();
                    self.editor.selected = Some(w.id.clone());
                    ui.ctx().memory_mut(|m| m.stop_text_input());
                }
                let title = Rect::from_min_size(
                    r.min,
                    vec2(r.width(), (30. * scale).max(14.).min(r.height())),
                );
                let handle = Rect::from_min_size(r.max - Vec2::splat(12.), Vec2::splat(12.));
                for (resize, hit) in [(false, title), (true, handle)] {
                    let resp = ui.interact(
                        hit,
                        ui.id().with((&w.id, resize)),
                        if w.locked {
                            Sense::click()
                        } else {
                            Sense::click_and_drag()
                        },
                    );
                    if resize && selected {
                        p.rect_filled(handle, 2., Color32::from_rgb(196, 245, 98));
                    }
                    if resp.clicked() {
                        self.editor.end_edit();
                        self.editor.selected = Some(w.id.clone());
                        self.resize_handle_focus = resize;
                        ui.ctx().memory_mut(|m| m.stop_text_input());
                    }
                    if resp.drag_started() && !w.locked {
                        self.editor.end_edit();
                        self.editor.selected = Some(w.id.clone());
                        self.editor.begin_edit();
                        self.drag = Some(Drag {
                            id: w.id.clone(),
                            start: [w.x, w.y, w.w, w.h],
                            resize,
                        });
                    }
                    if resp.dragged()
                        && let Some(drag) = &self.drag
                        && drag.id == w.id
                        && drag.resize == resize
                    {
                        let delta = resp.total_drag_delta().unwrap_or_default() / scale;
                        let (width, height) = (self.editor.layout.width, self.editor.layout.height);
                        if let Some(target) = self.editor.selected_mut() {
                            let snap = |v: f64| {
                                if self.grid {
                                    (v / 8.).round() * 8.
                                } else {
                                    v.round()
                                }
                            };
                            if resize {
                                target.w = snap(drag.start[2] + delta.x as f64);
                                target.h = snap(drag.start[3] + delta.y as f64);
                            } else {
                                target.x = snap(drag.start[0] + delta.x as f64);
                                target.y = snap(drag.start[1] + delta.y as f64);
                            }
                            target.constrain(width, height);
                        }
                    }
                    if resp.drag_stopped() {
                        self.resize_handle_focus = resize;
                        ui.ctx().memory_mut(|m| m.stop_text_input());
                        self.drag = None;
                        self.editor.end_edit();
                    }
                }
            }
        }
        if self.presenting {
            let badge = Rect::from_min_size(canvas.min + vec2(8., 8.), vec2(120., 25.));
            p.rect_filled(badge, 3., Color32::from_black_alpha(210));
            p.text(
                badge.center(),
                Align2::CENTER_CENTER,
                status,
                FontId::proportional(14.),
                Color32::from_rgb(255, 202, 106),
            );
        }
    }
    fn persist(&mut self) {
        self.workspace.active = self.editor.layout.clone();
        if self.workspace != self.last_saved {
            self.dirty_since.get_or_insert_with(Instant::now);
        }
        if !self.storage_blocked
            && Instant::now() >= self.retry_after
            && self
                .dirty_since
                .is_some_and(|t| t.elapsed() > Duration::from_millis(400))
        {
            self.saver.submit(self.workspace.clone());

            self.dirty_since = None;
            self.save_status = "กำลังบันทึก…".into();
        }
    }
}
impl DisplayApp {
    pub fn show(&mut self, ctx: &egui::Context) {
        let previous_size = self.last_canvas_size;
        self.poll(ctx);
        self.shortcuts(ctx);
        let (data, status) = self.telemetry();
        if !self.presenting {
            self.header(ctx, status);
            self.footer(ctx);
            if !self.preview {
                self.library_panel(ctx);
                self.inspector(ctx);
            }
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(10, 15, 20))
                    .inner_margin(if self.presenting { 0. } else { 10. }),
            )
            .show(ctx, |ui| self.canvas(ui, &data, status));
        if previous_size != (self.editor.layout.width, self.editor.layout.height) {
            self.canvas_width = self.editor.layout.width;
            self.canvas_height = self.editor.layout.height;
            self.last_canvas_size = (self.editor.layout.width, self.editor.layout.height);
        }
        self.persist();
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
impl eframe::App for DisplayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show(ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.workspace.active = self.editor.layout.clone();
        if !self.storage_blocked {
            self.saver.submit(self.workspace.clone());
        }
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::*;

    #[test]
    fn launch_with_native_window_bounds_does_not_deadlock() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let temp = tempfile::tempdir().unwrap();
            let ctx = egui::Context::default();
            let mut app = DisplayApp::new(&ctx, temp.path().join("workspace.json"));
            app.monitors = vec![Monitor {
                id: 1,
                label: "Test display".into(),
                x: 0.,
                y: 0.,
                width: 1920.,
                height: 1080.,
                scale: 1.,
                primary: true,
            }];
            app.monitor_id = Some(1);
            let mut input = egui::RawInput::default();
            let viewport = input.viewports.get_mut(&egui::ViewportId::ROOT).unwrap();
            viewport.outer_rect =
                Some(Rect::from_min_size(egui::pos2(80., 60.), vec2(1440., 932.)));
            viewport.inner_rect =
                Some(Rect::from_min_size(egui::pos2(80., 92.), vec2(1440., 900.)));
            let output = ctx.run(input, |ctx| app.launch(ctx));
            assert!(app.restore.is_some());
            assert!(app.presenting);
            assert!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::OuterPosition(_)))
            );
            app.pending_fullscreen = Some(Instant::now() - Duration::from_millis(300));
            let output = ctx.run(egui::RawInput::default(), |ctx| app.poll(ctx));
            assert!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::Fullscreen(true)))
            );
            let input = egui::RawInput {
                events: vec![egui::Event::Key {
                    key: Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
                ..Default::default()
            };
            let output = ctx.run(input, |ctx| app.poll(ctx));
            assert!(!app.presenting);
            assert!(
                output.viewport_output[&egui::ViewportId::ROOT]
                    .commands
                    .iter()
                    .any(|c| matches!(c, ViewportCommand::Fullscreen(false)))
            );
            tx.send(()).unwrap();
        });
        rx.recv_timeout(Duration::from_secs(3))
            .expect("launch/fullscreen/Escape must return without deadlocking the egui context");
    }
}
