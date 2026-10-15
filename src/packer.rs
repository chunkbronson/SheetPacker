use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ab_glyph::{FontVec, PxScale};
use anyhow::{Context, Result};
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;

/// Bundled font used to draw file-name labels into the sheet.
static LABEL_FONT: OnceLock<FontVec> = OnceLock::new();

fn label_font() -> &'static FontVec {
    LABEL_FONT.get_or_init(|| {
        let bytes = include_bytes!("../assets/DejaVuSans.ttf");
        FontVec::try_from_vec(bytes.to_vec()).expect("embedded label font is valid")
    })
}

/// How the sheet should be laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    Letterbox,
    Stretch,
    Exact,
}

#[derive(Debug, Clone, Copy)]
pub struct GridConfig {
    pub cols: u32,
    pub rows: u32,
    pub sheet_w: u32,
    pub sheet_h: u32,
    pub margin: u32,
    pub spacing: u32,
    pub fit_mode: FitMode,
    /// Draw each frame's file name.
    pub labels: bool,
    /// `true` = label beneath the frame (reserves space); `false` = label
    /// overlaid on the image (keeps edge-to-edge tiling).
    pub label_below: bool,
}

impl Default for GridConfig {
    fn default() -> Self {
        Self {
            cols: 4,
            rows: 4,
            sheet_w: 2048,
            sheet_h: 2048,
            margin: 0,
            spacing: 0,
            fit_mode: FitMode::Exact,
            labels: false,
            label_below: true,
        }
    }
}

/// Compose `files` into a single grid sheet and write it to `out`.
///
/// Frames are placed left-to-right, top-to-bottom, letterboxed (aspect
/// preserved) into each cell. Extra frames beyond `cols * rows` are ignored.
///
/// TODO(scaffold): enhancements are listed in PLAN.md.
pub fn build_sheet(files: &[PathBuf], cfg: &GridConfig, out: &Path) -> Result<PathBuf> {
    let img = build_sheet_image(files, cfg)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    DynamicImage::ImageRgba8(img).save(out).with_context(|| format!("saving {}", out.display()))?;
    Ok(out.to_path_buf())
}

/// Build the sheet image without saving to disk.
pub fn build_sheet_image(files: &[PathBuf], cfg: &GridConfig) -> Result<RgbaImage> {
    if files.is_empty() {
        anyhow::bail!("no input frames");
    }
    if cfg.cols == 0 || cfg.rows == 0 {
        anyhow::bail!("columns and rows must be at least 1");
    }
    if cfg.sheet_w == 0 || cfg.sheet_h == 0 {
        anyhow::bail!("sheet size must be at least 1 px");
    }

    let usable_w = cfg.sheet_w.saturating_sub(cfg.margin * 2);
    let usable_h = cfg.sheet_h.saturating_sub(cfg.margin * 2);
    let cell_w = usable_w.saturating_sub(cfg.spacing * (cfg.cols - 1)) / cfg.cols;
    let cell_h = usable_h.saturating_sub(cfg.spacing * (cfg.rows - 1)) / cfg.rows;
    if cell_w == 0 || cell_h == 0 {
        anyhow::bail!("sheet is too small for the requested grid");
    }

    let mut canvas = RgbaImage::from_pixel(cfg.sheet_w, cfg.sheet_h, Rgba([0, 0, 0, 0]));
    let capacity = (cfg.cols * cfg.rows) as usize;

    // Labels *on* the image overlay the frame (keeps the sheet a seamless
    // edge-to-edge tiling). Labels *beneath* reserve a strip at the bottom of
    // each cell — which, by design, is NOT edge-to-edge (e.g. contact sheets).
    let label_h = if cfg.labels {
        (cell_h / 8).clamp(14, 56)
    } else {
        0
    };
    let img_h = if cfg.labels && cfg.label_below {
        cell_h.saturating_sub(label_h).max(1)
    } else {
        cell_h
    };

    for (index, path) in files.iter().take(capacity).enumerate() {
        let img_val = image::open(path).with_context(|| format!("opening {}", path.display()))?;
        let thumb = match cfg.fit_mode {
            FitMode::Letterbox => fit(&img_val, cell_w, img_h),
            FitMode::Stretch => img_val
                .resize_exact(cell_w, img_h, image::imageops::FilterType::Lanczos3)
                .to_rgba8(),
            FitMode::Exact => img_val.to_rgba8(),
        };

        let col = index as u32 % cfg.cols;
        let row = index as u32 / cfg.cols;
        let cell_x = cfg.margin + col * (cell_w + cfg.spacing);
        let cell_y = cfg.margin + row * (cell_h + cfg.spacing);
        let x = cell_x + cell_w.saturating_sub(thumb.width()) / 2;
        let y = cell_y + img_h.saturating_sub(thumb.height()) / 2;

        image::imageops::overlay(&mut canvas, &thumb, x as i64, y as i64);

        if label_h > 0 {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            draw_label(&mut canvas, cell_x, cell_y + cell_h - label_h, cell_w, label_h, &name);
        }
    }
    Ok(canvas)
}

/// Draw a centred file-name label with a dark pill behind it for legibility.
fn draw_label(canvas: &mut RgbaImage, cell_x: u32, label_y: u32, cell_w: u32, label_h: u32, text: &str) {
    let font = label_font();
    let avail = cell_w.saturating_sub(8) as f32;
    let base = (label_h as f32 * 0.6).max(9.0);

    let mut scale = PxScale::from(base);
    let (mut tw, mut th) = text_size(scale, font, text);
    // Shrink to fit, then truncate if still too wide.
    if tw as f32 > avail {
        let f = (avail / tw as f32).max(0.35);
        scale = PxScale::from((base * f).max(7.0));
        let s = text_size(scale, font, text);
        tw = s.0;
        th = s.1;
    }
    let mut shown = text.to_string();
    while tw as f32 > avail && shown.chars().count() > 1 {
        shown.pop();
        let s = text_size(scale, font, &shown);
        tw = s.0;
        th = s.1;
    }

    let bx = cell_x + cell_w.saturating_sub(tw) / 2;
    let by = label_y + label_h.saturating_sub(th) / 2;

    let pad_x = 5i32;
    let pad_y = 2i32;
    let rect = Rect::at(bx as i32 - pad_x, by as i32 - pad_y)
        .of_size(tw + 2 * pad_x as u32, th + 2 * pad_y as u32);
    draw_filled_rect_mut(canvas, rect, Rgba([10, 12, 16, 180]));
    draw_text_mut(
        canvas,
        Rgba([236, 238, 242, 255]),
        bx as i32,
        by as i32,
        scale,
        font,
        &shown,
    );
}

/// Scale `img` down/up to fit inside `max_w` x `max_h`, preserving aspect ratio.
fn fit(img: &DynamicImage, max_w: u32, max_h: u32) -> RgbaImage {
    let w = img.width().max(1);
    let h = img.height().max(1);
    let scale = f64::min(max_w as f64 / w as f64, max_h as f64 / h as f64);

    let new_w = ((w as f64 * scale).round() as u32).clamp(1, max_w);
    let new_h = ((h as f64 * scale).round() as u32).clamp(1, max_h);

    img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3)
        .to_rgba8()
}

/// Sort frame paths in natural order, so `frame_2.png` comes before
/// `frame_10.png`. Both the GUI and the headless CLI use this.
pub fn natural_sort(files: &mut [PathBuf]) {
    files.sort_by(|a, b| natural_key(a).cmp(&natural_key(b)));
}

fn natural_key(path: &Path) -> (String, u64) {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut prefix = String::new();
    let mut number = 0u64;
    for ch in name.chars() {
        if ch.is_ascii_digit() {
            number = number
                .saturating_mul(10)
                .saturating_add(ch.to_digit(10).unwrap_or(0) as u64);
        } else if number == 0 {
            prefix.push(ch);
        }
    }
    (prefix, number)
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn packs_a_grid() {
        let dir = std::env::temp_dir().join(format!("sheetpacker_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut files = Vec::new();
        for i in 0..4u8 {
            let path = dir.join(format!("frame_{i}.png"));
            let img = RgbaImage::from_pixel(32, 32, Rgba([i.wrapping_mul(60), 30, 200, 255]));
            DynamicImage::ImageRgba8(img).save(&path).unwrap();
            files.push(path);
        }

        let out = dir.join("sheet.png");
        let cfg = GridConfig {
            cols: 2,
            rows: 2,
            sheet_w: 128,
            sheet_h: 128,
            margin: 0,
            spacing: 0,
            fit_mode: FitMode::Letterbox,
            labels: false,
            label_below: false,
        };
        build_sheet(&files, &cfg, &out).unwrap();

        let sheet = image::open(&out).unwrap();
        assert_eq!((sheet.width(), sheet.height()), (128, 128));

        std::fs::remove_dir_all(&dir).ok();
    }
}
