// Don't show a console window alongside the GUI on Windows.
#![windows_subsystem = "windows"]

mod app;
mod packer;

use std::path::{Path, PathBuf};

use eframe::egui;
use packer::{FitMode, GridConfig};

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut files: Vec<PathBuf> = Vec::new();
    let mut out: Option<PathBuf> = None;
    let mut cfg = GridConfig {
        cols: 4,
        rows: 4,
        sheet_w: 2048,
        sheet_h: 2048,
        margin: 0,
        spacing: 0,
        fit_mode: FitMode::Exact,
        labels: false,
        label_below: true,
    };

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let take = |i: usize| args.get(i + 1).map(String::as_str);
        match a {
            "--out" | "-o" => {
                if let Some(v) = take(i) {
                    out = Some(PathBuf::from(v));
                }
                i += 1;
            }
            "--cols" => {
                if let Some(v) = take(i).and_then(|v| v.parse().ok()) {
                    cfg.cols = v;
                }
                i += 1;
            }
            "--rows" => {
                if let Some(v) = take(i).and_then(|v| v.parse().ok()) {
                    cfg.rows = v;
                }
                i += 1;
            }
            "--size" => {
                if let Some((w, h)) = take(i).and_then(|v| v.split_once('x')) {
                    if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                        cfg.sheet_w = w;
                        cfg.sheet_h = h;
                    }
                }
                i += 1;
            }
            "--margin" => {
                if let Some(v) = take(i).and_then(|v| v.parse().ok()) {
                    cfg.margin = v;
                }
                i += 1;
            }
            "--spacing" => {
                if let Some(v) = take(i).and_then(|v| v.parse().ok()) {
                    cfg.spacing = v;
                }
                i += 1;
            }
            "--fit" => {
                if let Some(v) = take(i) {
                    cfg.fit_mode = match v {
                        "letterbox" => FitMode::Letterbox,
                        "stretch" => FitMode::Stretch,
                        _ => FitMode::Exact,
                    };
                }
                i += 1;
            }
            "--labels" => cfg.labels = true,
            "--labels-on-image" => {
                cfg.labels = true;
                cfg.label_below = false;
            }
            other => collect_path(Path::new(other), &mut files),
        }
        i += 1;
    }

    // Stable, filename-ordered packing for both modes.
    packer::natural_sort(&mut files);

    // --- Headless export mode: `sheetpacker <inputs> --out sheet.png` -----
    if let Some(out_path) = out {
        if files.is_empty() {
            eprintln!("error: no input images given");
            std::process::exit(1);
        }
        match packer::build_sheet(&files, &cfg, &out_path) {
            Ok(p) => {
                println!("wrote {} ({}×{})", p.display(), cfg.sheet_w, cfg.sheet_h);
                return Ok(());
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                std::process::exit(1);
            }
        }
    }

    // --- GUI mode --------------------------------------------------------
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 740.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "SheetPacker",
        options,
        Box::new(move |_cc| {
            let app = if files.is_empty() {
                app::SheetPackerApp::default()
            } else {
                app::SheetPackerApp::with_files(files, cfg)
            };
            Ok(Box::new(app))
        }),
    )
}

/// Window / taskbar icon (also embedded as a Windows resource for the .exe).
fn load_icon() -> egui::IconData {
    let bytes = include_bytes!("../assets/icon_256.png");
    let img = image::load_from_memory(bytes)
        .expect("embedded icon is a valid PNG")
        .to_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}

/// Add an image file, or every image inside a directory, to `out`.
fn collect_path(path: &Path, out: &mut Vec<PathBuf>) {
    const ALLOWED: [&str; 5] = ["png", "jpg", "jpeg", "webp", "bmp"];
    let is_image = |p: &Path| {
        p.extension()
            .and_then(|s| s.to_str())
            .map(|ext| ALLOWED.iter().any(|a| a.eq_ignore_ascii_case(ext)))
            .unwrap_or(false)
    };

    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && is_image(&p) {
                    out.push(p);
                }
            }
        }
    } else if path.is_file() {
        out.push(path.to_path_buf());
    }
}
