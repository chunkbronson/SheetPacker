use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::{DynamicImage, Rgba, RgbaImage};

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
            fit_mode: FitMode::Letterbox,
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

    for (index, path) in files.iter().take(capacity).enumerate() {
        let img_val = image::open(path).with_context(|| format!("opening {}", path.display()))?;
        let thumb = match cfg.fit_mode {
            FitMode::Letterbox => fit(&img_val, cell_w, cell_h),
            FitMode::Stretch => {
                img_val
                    .resize_exact(cell_w, cell_h, image::imageops::FilterType::Lanczos3)
                    .to_rgba8()
            }
            FitMode::Exact => img_val.to_rgba8(),
        };

        let col = index as u32 % cfg.cols;
        let row = index as u32 / cfg.cols;
        let x = cfg.margin
            + col * (cell_w + cfg.spacing)
            + (cell_w - thumb.width()) / 2;
        let y = cfg.margin
            + row * (cell_h + cfg.spacing)
            + (cell_h - thumb.height()) / 2;

        image::imageops::overlay(&mut canvas, &thumb, x as i64, y as i64);
    }
    Ok(canvas)
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
        };
        build_sheet(&files, &cfg, &out).unwrap();

        let sheet = image::open(&out).unwrap();
        assert_eq!((sheet.width(), sheet.height()), (128, 128));

        std::fs::remove_dir_all(&dir).ok();
    }
}
