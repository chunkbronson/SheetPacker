use std::path::{Path, PathBuf};

use eframe::egui;
use egui::{Color32, CornerRadius, Margin};

use crate::packer::{self, FitMode, GridConfig};

/// Accent + surface colors for the modern dark theme.
const ACCENT: Color32 = Color32::from_rgb(0x6e, 0xa8, 0xfe);
const PANEL_BG: Color32 = Color32::from_rgb(0x1a, 0x1c, 0x21);
const SURFACE: Color32 = Color32::from_rgb(0x24, 0x27, 0x2e);
const SURFACE_HI: Color32 = Color32::from_rgb(0x2e, 0x32, 0x3b);
const BORDER: Color32 = Color32::from_rgb(0x36, 0x3a, 0x44);
const TEXT: Color32 = Color32::from_rgb(0xe7, 0xe8, 0xec);
const MUTED: Color32 = Color32::from_rgb(0x9a, 0xa1, 0xad);

pub struct SheetPackerApp {
    files: Vec<PathBuf>,
    cols: usize,
    rows: usize,
    sheet_w: u32,
    sheet_h: u32,
    margin: u32,
    spacing: u32,
    fit_mode: FitMode,
    labels: bool,
    status: String,
    last_output: Option<PathBuf>,
    preview_tex: Option<egui::TextureHandle>,
    dirty: bool,
    styled: bool,
    // playback
    playing: bool,
    play_pos: f64,
    play_index: usize,
    frame_tex: Option<(usize, egui::TextureHandle)>,
    fps: f32,
}

impl Default for SheetPackerApp {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            cols: 4,
            rows: 4,
            sheet_w: 2048,
            sheet_h: 2048,
            margin: 0,
            spacing: 0,
            fit_mode: FitMode::Exact,
            labels: false,
            status: "Drop PNG frames here, or use “Add files…” / “Import folder…”."
                .to_owned(),
            last_output: None,
            preview_tex: None,
            dirty: true,
            styled: false,
            playing: false,
            play_pos: 0.0,
            play_index: 0,
            frame_tex: None,
            fps: 12.0,
        }
    }
}

impl SheetPackerApp {
    /// Create the app with frames preloaded (e.g. from a CLI argument).
    pub fn with_files(files: Vec<PathBuf>) -> Self {
        let mut app = Self::default();
        app.files = files;
        app.sort_files();
        app.dirty = true;
        app.status = format!("{} frame(s) loaded", app.files.len());
        app
    }

    fn grid_config(&self) -> GridConfig {
        GridConfig {
            cols: self.cols as u32,
            rows: self.rows as u32,
            sheet_w: self.sheet_w,
            sheet_h: self.sheet_h,
            margin: self.margin,
            spacing: self.spacing,
            fit_mode: self.fit_mode,
            labels: self.labels && self.fit_mode != FitMode::Letterbox,
        }
    }

    fn sort_files(&mut self) {
        packer::natural_sort(&mut self.files);
    }

    fn add_file(&mut self, path: PathBuf) {
        self.files.push(path);
        self.sort_files();
        self.dirty = true;
    }

    fn add_files_dialog(&mut self) {
        if let Some(paths) = rfd::FileDialog::new()
            .set_title("Add image frames")
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_files()
        {
            let n = paths.len();
            for p in paths {
                self.files.push(p);
            }
            self.sort_files();
            self.dirty = true;
            self.status = format!("Added {n} frame(s)");
        }
    }

    fn import_folder(&mut self) {
        let Some(dir) = rfd::FileDialog::new()
            .set_title("Import a folder of image frames")
            .pick_folder()
        else {
            return;
        };
        let allowed = ["png", "jpg", "jpeg", "webp", "bmp"];
        let Ok(entries) = std::fs::read_dir(&dir) else {
            self.status = format!("Could not read {}", dir.display());
            return;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .map(|ext| allowed.iter().any(|a| a.eq_ignore_ascii_case(ext)))
                        .unwrap_or(false)
            })
            .collect();
        packer::natural_sort(&mut paths);
        let n = paths.len();
        self.files.extend(paths);
        self.sort_files();
        self.dirty = true;
        self.status = format!("Imported {n} frame(s) from {}", dir.display());
    }

    fn export(&mut self) {
        let cfg = self.grid_config();
        let Some(out) = rfd::FileDialog::new()
            .set_file_name("sheet.png")
            .add_filter("PNG image", &["png"])
            .save_file()
        else {
            return;
        };
        match packer::build_sheet(&self.files, &cfg, &out) {
            Ok(saved) => {
                self.status = format!("Saved {}", saved.display());
                self.last_output = Some(saved);
            }
            Err(err) => {
                self.status = format!("Error: {err:#}");
            }
        }
    }

    /// Smallest square-ish grid that fits every frame.
    fn autofit_grid(&mut self) {
        let n = self.files.len().max(1);
        let side = (n as f64).sqrt().ceil() as usize;
        self.cols = side.clamp(1, 64);
        self.rows = n.div_ceil(self.cols).clamp(1, 64);
        self.dirty = true;
        self.status = format!("Auto-fit grid: {}×{}", self.cols, self.rows);
    }

    /// Move a frame from index `from` to index `to`.
    fn move_file(&mut self, from: usize, to: usize) {
        if from >= self.files.len() || from == to {
            return;
        }
        let item = self.files.remove(from);
        let at = to.min(self.files.len());
        self.files.insert(at, item);
        self.dirty = true;
        self.status = format!("Moved frame {} → {}", from + 1, at + 1);
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.all_styles_mut(|style| {
            let mut v = egui::Visuals::dark();

            v.dark_mode = true;
            v.panel_fill = PANEL_BG;
            v.window_fill = SURFACE;
            v.extreme_bg_color = Color32::from_rgb(0x12, 0x13, 0x17);
            v.faint_bg_color = SURFACE;
            v.override_text_color = Some(TEXT);
            v.weak_text_color = Some(MUTED);
            v.hyperlink_color = ACCENT;
            v.warn_fg_color = Color32::from_rgb(0xf5, 0xc4, 0x51);
            v.error_fg_color = Color32::from_rgb(0xff, 0x6b, 0x6b);
            v.selection.bg_fill = Color32::from_rgb(0x2b, 0x40, 0x6b);
            v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
            v.window_corner_radius = CornerRadius::same(12);
            v.menu_corner_radius = CornerRadius::same(10);

            let mk = |bg: Color32, weak: Color32, stroke: Color32, fg: Color32, r: u8| {
                egui::style::WidgetVisuals {
                    bg_fill: bg,
                    weak_bg_fill: weak,
                    bg_stroke: egui::Stroke::new(1.0, stroke),
                    corner_radius: CornerRadius::same(r),
                    fg_stroke: egui::Stroke::new(1.0, fg),
                    expansion: 0.0,
                }
            };

            v.widgets.noninteractive = mk(Color32::TRANSPARENT, SURFACE, BORDER, MUTED, 8);
            v.widgets.inactive = mk(SURFACE, SURFACE, BORDER, TEXT, 8);
            v.widgets.hovered = mk(SURFACE_HI, SURFACE_HI, ACCENT, TEXT, 8);
            v.widgets.active = mk(ACCENT, ACCENT, ACCENT, Color32::WHITE, 8);
            v.widgets.open = mk(SURFACE, SURFACE, BORDER, TEXT, 8);

            style.visuals = v;
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 7.0);
            style.spacing.interact_size = egui::vec2(40.0, 26.0);
            style.spacing.slider_width = 150.0;
        });
    }
}

impl eframe::App for SheetPackerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.styled {
            self.apply_theme(ui.ctx());
            self.styled = true;
        }

        // --- Drag & drop files -------------------------------------------
        let dropped = ui.ctx().input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for file in dropped {
                self.add_file(file.path().to_path_buf());
            }
        }

        let snapshot = (
            self.cols,
            self.rows,
            self.sheet_w,
            self.sheet_h,
            self.margin,
            self.spacing,
            self.fit_mode,
            self.labels,
        );

        // --- Top bar -----------------------------------------------------
        egui::Panel::top("top")
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(Margin::symmetric(18, 14)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("SheetPacker")
                            .size(30.0)
                            .strong()
                            .color(ACCENT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Clear").clicked() {
                            self.files.clear();
                            self.dirty = true;
                            self.last_output = None;
                            self.status = "Cleared".to_owned();
                        }
                        if ui.button("Import folder…").clicked() {
                            self.import_folder();
                        }
                        if ui
                            .add(egui::Button::new("Add files…").fill(ACCENT))
                            .clicked()
                        {
                            self.add_files_dialog();
                        }
                    });
                });
            });

        // --- Frames (left) ----------------------------------------------
        egui::Panel::left("frames")
            .resizable(true)
            .min_size(240.0)
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(Margin::symmetric(12, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("FRAMES").size(11.0).strong().color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("{}", self.files.len()))
                                .size(11.0)
                                .color(MUTED),
                        );
                    });
                });
                ui.add_space(6.0);

                if self.files.is_empty() {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("No frames yet.\nDrop images here or add them above.")
                            .size(12.0)
                            .color(MUTED),
                    );
                } else {
                    let mut remove: Option<usize> = None;
                    let mut rects: Vec<egui::Rect> = Vec::with_capacity(self.files.len());

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (i, path) in self.files.iter().enumerate() {
                                let name = path
                                    .file_name()
                                    .map(|s| s.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let id = egui::Id::new(("frame_row", i));
                                let resp = ui
                                    .dnd_drag_source(id, i, |ui| {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .small_button("\u{00d7}")
                                                .on_hover_text("Remove")
                                                .clicked()
                                            {
                                                remove = Some(i);
                                            }
                                            ui.label(
                                                egui::RichText::new(format!("{:>3}", i + 1))
                                                    .size(11.0)
                                                    .monospace()
                                                    .color(MUTED),
                                            );
                                            ui.label(name.clone())
                                                .on_hover_text(path.display().to_string());
                                        });
                                    })
                                    .response;
                                rects.push(resp.rect);
                            }
                        });

                    // Reorder via drag & drop.
                    let pointer = ui.ctx().input(|i| i.pointer.interact_pos());
                    if let Some(dragged) = egui::DragAndDrop::payload::<usize>(ui.ctx()) {
                        if let Some(pos) = pointer {
                            let target = rects.iter().position(|r| r.contains(pos));
                            if let Some(t) = target {
                                let r = rects[t];
                                ui.painter().line_segment(
                                    [egui::pos2(r.left(), r.top()), egui::pos2(r.right(), r.top())],
                                    egui::Stroke::new(2.5, ACCENT),
                                );
                                if ui.ctx().input(|i| i.pointer.any_released()) && t != *dragged {
                                    self.move_file(*dragged, t);
                                }
                            }
                        }
                    }

                    if let Some(i) = remove {
                        self.files.remove(i);
                        self.dirty = true;
                    }
                }
            });

        // --- Settings (right) -------------------------------------------
        egui::Panel::right("settings")
            .min_size(300.0)
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(Margin::symmetric(14, 12)),
            )
            .show(ui, |ui| {
                ui.label(egui::RichText::new("LAYOUT").size(11.0).strong().color(MUTED));
                ui.add_space(6.0);
                ui.add(egui::Slider::new(&mut self.cols, 1..=32).text("Columns"));
                ui.add(egui::Slider::new(&mut self.rows, 1..=32).text("Rows"));

                ui.add_space(14.0);
                ui.label(egui::RichText::new("SHEET").size(11.0).strong().color(MUTED));
                ui.add_space(6.0);
                egui::Grid::new("sheet_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Width");
                        ui.add(egui::DragValue::new(&mut self.sheet_w).range(16..=16384).suffix(" px"));
                        ui.end_row();
                        ui.label("Height");
                        ui.add(egui::DragValue::new(&mut self.sheet_h).range(16..=16384).suffix(" px"));
                        ui.end_row();
                        ui.label("Margin");
                        ui.add(egui::DragValue::new(&mut self.margin).range(0..=256).suffix(" px"));
                        ui.end_row();
                        ui.label("Spacing");
                        ui.add(egui::DragValue::new(&mut self.spacing).range(0..=256).suffix(" px"));
                        ui.end_row();
                    });

                ui.add_space(14.0);
                ui.label(egui::RichText::new("FIT").size(11.0).strong().color(MUTED));
                ui.add_space(6.0);
                egui::ComboBox::from_id_salt("fit_mode")
                    .selected_text(fit_label(self.fit_mode))
                    .width(ui.available_width() - 4.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fit_mode, FitMode::Letterbox, "Letterbox (keep aspect)");
                        ui.selectable_value(&mut self.fit_mode, FitMode::Stretch, "Stretch to cell");
                        ui.selectable_value(&mut self.fit_mode, FitMode::Exact, "Exact (no scaling)");
                    });

                ui.add_space(10.0);
                let labels_ok = self.fit_mode != FitMode::Letterbox;
                ui.add_enabled(
                    labels_ok,
                    egui::Checkbox::new(&mut self.labels, "File-name labels"),
                )
                .on_disabled_hover_text("Labels are for grid layouts — use Exact or Stretch.");
                if labels_ok && self.labels {
                    ui.label(
                        egui::RichText::new("good for contact sheets")
                            .size(11.0)
                            .color(MUTED),
                    );
                }

                ui.add_space(14.0);
                let capacity = self.cols * self.rows;
                if self.files.len() > capacity {
                    ui.label(
                        egui::RichText::new(format!(
                            "⚠  {} of {} frames won't fit",
                            self.files.len() - capacity,
                            self.files.len()
                        ))
                        .size(12.0)
                        .color(egui::Color32::from_rgb(0xf5, 0xc4, 0x51)),
                    );
                    ui.add_space(4.0);
                    if ui.button("Auto-fit grid").clicked() {
                        self.autofit_grid();
                    }
                    ui.add_space(10.0);
                }

                ui.separator();
                ui.add_space(6.0);
                let enabled = !self.files.is_empty();
                let export = egui::Button::new(egui::RichText::new("Export sheet…").strong())
                    .fill(ACCENT)
                    .min_size(egui::vec2(ui.available_width(), 36.0));
                if ui.add_enabled(enabled, export).clicked() {
                    self.export();
                }
                if let Some(out) = &self.last_output {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Saved: {}", out.display()))
                            .size(11.0)
                            .color(MUTED),
                    );
                }
            });

        // --- Status bar (bottom) ----------------------------------------
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(Margin::symmetric(16, 7)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status).size(12.0).color(MUTED));
                    if let Some(tex) = &self.preview_tex {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let s = tex.size();
                            ui.label(
                                egui::RichText::new(format!("{} × {} px", s[0], s[1]))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                        });
                    }
                });
            });

        // --- Playback tick ----------------------------------------------
        if self.playing && !self.files.is_empty() {
            let dt = ui.input(|i| i.stable_dt) as f64;
            self.play_pos += dt;
            let n = self.files.len();
            let idx = ((self.play_pos * self.fps as f64) as usize) % n;
            if idx != self.play_index || self.frame_tex.as_ref().map(|(i, _)| *i) != Some(idx) {
                self.play_index = idx;
                if let Some(path) = self.files.get(idx) {
                    if let Some(tex) = load_frame_texture(ui.ctx(), path) {
                        self.frame_tex = Some((idx, tex));
                    }
                }
            }
            ui.ctx().request_repaint();
        }

        // --- Rebuild sheet preview when anything changed ----------------
        if self.dirty {
            if self.files.is_empty() {
                self.preview_tex = None;
            } else {
                let cfg = self.grid_config();
                match packer::build_sheet_image(&self.files, &cfg) {
                    Ok(img) => {
                        let size = [img.width() as usize, img.height() as usize];
                        let buffer = img.into_raw();
                        let color = egui::ColorImage::from_rgba_unmultiplied(size, &buffer);
                        let tex = ui
                            .ctx()
                            .load_texture("preview", color, egui::TextureOptions::LINEAR);
                        self.preview_tex = Some(tex);
                    }
                    Err(err) => {
                        self.status = format!("Preview error: {err}");
                        self.preview_tex = None;
                    }
                }
            }
            self.dirty = false;
        }

        // --- Preview (central, must be last) ----------------------------
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .inner_margin(Margin::same(14)),
            )
            .show(ui, |ui| {
                // playback toolbar
                ui.horizontal(|ui| {
                    let many = self.files.len() > 1;
                    if ui
                        .add_enabled(many, egui::Button::new(if self.playing { "Pause" } else { "Play" }))
                        .clicked()
                    {
                        self.playing = !self.playing;
                        self.play_pos = 0.0;
                        self.play_index = 0;
                    }
                    ui.add_enabled(
                        many,
                        egui::DragValue::new(&mut self.fps).range(1.0..=60.0).suffix(" fps"),
                    );
                    if self.playing {
                        ui.label(
                            egui::RichText::new(format!(
                                "frame {}/{}",
                                self.play_index + 1,
                                self.files.len()
                            ))
                            .size(12.0)
                            .color(MUTED),
                        );
                    }
                });
                ui.add_space(6.0);

                egui::Frame::NONE
                    .fill(SURFACE)
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());

                        let playback_tex = if self.playing {
                            self.frame_tex.as_ref().map(|(_, t)| t)
                        } else {
                            None
                        };

                        if let Some(tex) = playback_tex.or(self.preview_tex.as_ref()) {
                            ui.centered_and_justified(|ui| {
                                ui.add(
                                    egui::Image::new(tex)
                                        .max_size(ui.available_size())
                                        .maintain_aspect_ratio(true),
                                );
                            });
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new("Drop PNG frames to begin")
                                        .size(18.0)
                                        .color(MUTED),
                                );
                            });
                        }
                    });
            });

        // --- Detect setting changes -> mark dirty -----------------------
        let now = (
            self.cols,
            self.rows,
            self.sheet_w,
            self.sheet_h,
            self.margin,
            self.spacing,
            self.fit_mode,
            self.labels,
        );
        if now != snapshot {
            self.dirty = true;
        }
    }
}

fn fit_label(mode: FitMode) -> &'static str {
    match mode {
        FitMode::Letterbox => "Letterbox (keep aspect)",
        FitMode::Stretch => "Stretch to cell",
        FitMode::Exact => "Exact (no scaling)",
    }
}

/// Load one frame from disk into an egui texture (used by playback).
fn load_frame_texture(ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let buffer = img.into_raw();
    let color = egui::ColorImage::from_rgba_unmultiplied(size, &buffer);
    Some(ctx.load_texture("playback_frame", color, egui::TextureOptions::LINEAR))
}
