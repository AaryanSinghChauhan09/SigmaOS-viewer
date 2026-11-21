// SPDX-License-Identifier: Apache-2.0
//!
//! SigmaOS-viewer — image viewer core.
//!
//! Provides the policy/state layer for an image viewer: supported-format
//! detection, image metadata (dimensions, color depth, format), gallery
//! navigation (next/prev/first/last, sorting), slideshow timing, and
//! per-image annotations. Rendering is delegated to a backend trait so
//! tests stay deterministic; a real backend wraps the OS image stack.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Supported image formats with magic-byte signatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub enum ImageFormat {
    Png, Jpeg, Gif, Bmp, WebP, Tiff, Svg, Ico, Pnm, Qoi,
}
impl ImageFormat {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Png=>"png", Self::Jpeg=>"jpeg", Self::Gif=>"gif",
            Self::Bmp=>"bmp", Self::WebP=>"webp", Self::Tiff=>"tiff",
            Self::Svg=>"svg", Self::Ico=>"ico", Self::Pnm=>"pnm", Self::Qoi=>"qoi",
        }
    }
    pub fn mime(&self) -> &'static str {
        match self {
            Self::Png=>"image/png", Self::Jpeg=>"image/jpeg",
            Self::Gif=>"image/gif", Self::Bmp=>"image/bmp",
            Self::WebP=>"image/webp", Self::Tiff=>"image/tiff",
            Self::Svg=>"image/svg+xml", Self::Ico=>"image/x-icon",
            Self::Pnm=>"image/x-portable-anymap", Self::Qoi=>"image/qoi",
        }
    }
    /// Detect format from magic bytes.
    pub fn sniff(header: &[u8]) -> Option<Self> {
        if header.len() < 4 { return None; }
        Some(match header {
            [0x89, b'P', b'N', b'G', ..] => Self::Png,
            [0xFF, 0xD8, 0xFF, ..] => Self::Jpeg,
            [b'G', b'I', b'F', b'8', ..] => Self::Gif,
            [b'B', b'M', ..] => Self::Bmp,
            [b'R', b'I', b'F', b'F', ..] if header.len() >= 12 && &header[8..12] == b"WEBP" => Self::WebP,
            [b'I', b'I', 0x2A, 0x00, ..] | [b'M', b'M', 0x00, 0x2A, ..] => Self::Tiff,
            [b'\x00', b'\x00', b'\x01', b'\x00', ..] => Self::Ico,
            [b'P', b'1'..=b'6', b'\n', ..] | [b'P', b'1'..=b'6', b' ', ..] => Self::Pnm,
            [b'q', b'o', b'i', b'f', ..] => Self::Qoi,
            _ => {
                // SVG: text-based, check for <svg or <?xml ... <svg
                let head = String::from_utf8_lossy(&header[..header.len().min(512)]);
                if head.contains("<svg") || (head.contains("<?xml") && head.contains("svg")) {
                    Self::Svg
                } else { return None; }
            }
        })
    }
    /// Detect from file extension (fallback).
    pub fn from_ext(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png), "jpg"|"jpeg" => Some(Self::Jpeg),
            "gif" => Some(Self::Gif), "bmp" => Some(Self::Bmp),
            "webp" => Some(Self::WebP), "tif"|"tiff" => Some(Self::Tiff),
            "svg" => Some(Self::Svg), "ico" => Some(Self::Ico),
            "pnm"|"ppm"|"pgm"|"pbm" => Some(Self::Pnm),
            "qoi" => Some(Self::Qoi), _ => None,
        }
    }
}

/// Parsed image metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImageInfo {
    pub path: PathBuf,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub size_bytes: u64,
    pub color_depth: Option<u8>,
    pub animated: bool,
}
impl ImageInfo {
    pub fn megapixels(&self) -> f64 {
        (self.width as f64) * (self.height as f64) / 1_000_000.0
    }
    pub fn aspect(&self) -> f64 {
        if self.height == 0 { 0.0 } else { self.width as f64 / self.height as f64 }
    }
    pub fn is_landscape(&self) -> bool { self.width > self.height }
    pub fn is_portrait(&self) -> bool { self.height > self.width }
    pub fn human_size(&self) -> String {
        human_bytes(self.size_bytes)
    }
}

pub fn human_bytes(n: u64) -> String {
    const U: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 { v /= 1024.0; i += 1; }
    format!("{:.1} {}", v, U[i])
}

/// Gallery sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortOrder {
    Name, NameDesc, Size, SizeDesc, Modified, ModifiedDesc,
}
impl SortOrder {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Name=>"name", Self::NameDesc=>"name-desc",
            Self::Size=>"size", Self::SizeDesc=>"size-desc",
            Self::Modified=>"modified", Self::ModifiedDesc=>"modified-desc",
        }
    }
}

/// Viewer errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewError {
    Unsupported(String),
    NotImage(PathBuf),
    Decode(String),
    EmptyGallery,
    BadIndex(usize),
    Io(String),
}
impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(m)=>write!(f,"unsupported: {}",m),
            Self::NotImage(p)=>write!(f,"not an image: {}",p.display()),
            Self::Decode(m)=>write!(f,"decode: {}",m),
            Self::EmptyGallery=>write!(f,"empty gallery"),
            Self::BadIndex(i)=>write!(f,"bad index {}",i),
            Self::Io(m)=>write!(f,"io: {}",m),
        }
    }
}
impl std::error::Error for ViewError {}

/// Filesystem abstraction for tests.
pub trait ImageFs {
    fn read_head(&self, path: &Path, n: usize) -> Result<Vec<u8>, String>;
    fn read_all(&self, path: &Path) -> Result<Vec<u8>, String>;
    fn metadata(&self, path: &Path) -> Result<(u64, u64), String>; // (size, mtime)
    fn list_dir(&self, dir: &Path) -> Result<Vec<PathBuf>, String>;
}

/// Host filesystem.
pub struct HostFs;
impl ImageFs for HostFs {
    fn read_head(&self, path: &Path, n: usize) -> Result<Vec<u8>, String> {
        let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
        use std::io::Read;
        let mut buf = vec![0u8; n];
        let got = f.take(n as u64).read(&mut buf).map_err(|e| e.to_string())?;
        buf.truncate(got);
        Ok(buf)
    }
    fn read_all(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| e.to_string())
    }
    fn metadata(&self, path: &Path) -> Result<(u64, u64), String> {
        let m = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let size = m.len();
        let mtime = m.modified().ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs()).unwrap_or(0);
        Ok((size, mtime))
    }
    fn list_dir(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut out = vec![];
        for e in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            out.push(e.path());
        }
        Ok(out)
    }
}

/// In-memory FS for tests.
#[derive(Debug, Default)]
pub struct MemFs {
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    pub meta: BTreeMap<PathBuf, (u64, u64)>,
}
impl ImageFs for MemFs {
    fn read_head(&self, path: &Path, n: usize) -> Result<Vec<u8>, String> {
        let d = self.files.get(path).ok_or("not found")?;
        Ok(d[..d.len().min(n)].to_vec())
    }
    fn read_all(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.files.get(path).cloned().ok_or("not found".into())
    }
    fn metadata(&self, path: &Path) -> Result<(u64, u64), String> {
        self.meta.get(path).copied().ok_or("no meta".into())
    }
    fn list_dir(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(self.files.keys().filter(|p| p.parent() == Some(dir)).cloned().collect())
    }
}

/// Parse dimensions for common formats from raw bytes.
pub fn parse_dimensions(fmt: ImageFormat, data: &[u8]) -> Option<(u32, u32)> {
    match fmt {
        ImageFormat::Png => {
            // IHDR at offset 16: width(4) height(4) big-endian
            if data.len() < 24 { return None; }
            let w = u32::from_be_bytes([data[16],data[17],data[18],data[19]]);
            let h = u32::from_be_bytes([data[20],data[21],data[22],data[23]]);
            Some((w, h))
        }
        ImageFormat::Gif => {
            if data.len() < 10 { return None; }
            let w = u16::from_le_bytes([data[6],data[7]]) as u32;
            let h = u16::from_le_bytes([data[8],data[9]]) as u32;
            Some((w, h))
        }
        ImageFormat::Bmp => {
            if data.len() < 26 { return None; }
            let w = i32::from_le_bytes([data[18],data[19],data[20],data[21]]);
            let h = i32::from_le_bytes([data[22],data[23],data[24],data[25]]).abs();
            Some((w.max(0) as u32, h as u32))
        }
        ImageFormat::Jpeg => parse_jpeg_dims(data),
        ImageFormat::WebP => parse_webp_dims(data),
        ImageFormat::Tiff => parse_tiff_dims(data),
        ImageFormat::Ico => {
            if data.len() < 8 { return None; }
            let w = if data[6] == 0 { 256 } else { data[6] as u32 };
            let h = if data[7] == 0 { 256 } else { data[7] as u32 };
            Some((w, h))
        }
        ImageFormat::Qoi => {
            // qoi header: magic(4) width(4 BE) height(4 BE)
            if data.len() < 14 { return None; }
            let w = u32::from_be_bytes([data[4],data[5],data[6],data[7]]);
            let h = u32::from_be_bytes([data[8],data[9],data[10],data[11]]);
            Some((w, h))
        }
        ImageFormat::Svg | ImageFormat::Pnm => None,
    }
}

fn parse_jpeg_dims(data: &[u8]) -> Option<(u32, u32)> {
    // scan for SOF markers
    let mut i = 2;
    while i + 9 < data.len() {
        if data[i] != 0xFF { i += 1; continue; }
        let marker = data[i+1];
        // SOF0-15 except DHT(0xC4), JPG(0xC8), DAC(0xCC)
        if (0xC0..=0xCF).contains(&marker) && ![0xC4,0xC8,0xCC].contains(&marker) {
            if i + 9 > data.len() { return None; }
            let h = u16::from_be_bytes([data[i+5],data[i+6]]) as u32;
            let w = u16::from_be_bytes([data[i+7],data[i+8]]) as u32;
            return Some((w, h));
        }
        if i + 4 > data.len() { return None; }
        let seglen = u16::from_be_bytes([data[i+2],data[i+3]]) as usize;
        i += 2 + seglen.max(2);
    }
    None
}

fn parse_webp_dims(data: &[u8]) -> Option<(u32, u32)> {
    // RIFF....WEBP then VP8/VP8L/VP8X chunk
    if data.len() < 30 { return None; }
    match &data[12..16] {
        b"VP8 " => {
            // lossy: dims at offset 26 (14-bit)
            if data.len() < 30 { return None; }
            let w = u16::from_le_bytes([data[26],data[27]]) & 0x3FFF;
            let h = u16::from_le_bytes([data[28],data[29]]) & 0x3FFF;
            Some((w as u32, h as u32))
        }
        b"VP8L" => {
            if data.len() < 25 { return None; }
            let b = [data[21],data[22],data[23],data[24]];
            let bits = u32::from_le_bytes(b);
            let w = (bits & 0x3FFF) + 1;
            let h = ((bits >> 14) & 0x3FFF) + 1;
            Some((w, h))
        }
        b"VP8X" => {
            if data.len() < 30 { return None; }
            let w = (data[24] as u32) | ((data[25] as u32) << 8) | ((data[26] as u32) << 16);
            let h = (data[27] as u32) | ((data[28] as u32) << 8) | ((data[29] as u32) << 16);
            Some((w + 1, h + 1))
        }
        _ => None,
    }
}

fn parse_tiff_dims(data: &[u8]) -> Option<(u32, u32)> {
    // minimal: find IFD, look for tags 256 (width) & 257 (height)
    if data.len() < 8 { return None; }
    let le = &data[0..2] == b"II";
    let rd16 = |o: usize| -> u16 {
        if le { u16::from_le_bytes([data[o],data[o+1]]) }
        else { u16::from_be_bytes([data[o],data[o+1]]) }
    };
    let rd32 = |o: usize| -> u32 {
        if le { u32::from_le_bytes([data[o],data[o+1],data[o+2],data[o+3]]) }
        else { u32::from_be_bytes([data[o],data[o+1],data[o+2],data[o+3]]) }
    };
    let magic = rd16(2);
    if magic != 42 { return None; }
    let ifd_off = rd32(4) as usize;
    if ifd_off + 2 > data.len() { return None; }
    let n = rd16(ifd_off) as usize;
    let mut w = None;
    let mut h = None;
    for i in 0..n {
        let e = ifd_off + 2 + i * 12;
        if e + 12 > data.len() { break; }
        let tag = rd16(e);
        let typ = rd16(e + 2);
        let cnt = rd32(e + 4);
        if cnt == 0 { continue; }
        let val = if typ == 3 && cnt == 1 {
            rd16(e + 8) as u32
        } else if typ == 4 && cnt == 1 {
            rd32(e + 8)
        } else { continue; };
        match tag {
            256 => w = Some(val),
            257 => h = Some(val),
            _ => {}
        }
    }
    match (w, h) { (Some(w), Some(h)) => Some((w, h)), _ => None }
}

/// The image viewer.
pub struct ImageViewer<F: ImageFs> {
    pub fs: F,
    pub gallery: Vec<ImageInfo>,
    pub index: usize,
    pub sort: SortOrder,
    pub slideshow_secs: u64,
    pub show_hidden: bool,
    /// per-path annotations (rating, tags, caption).
    pub annotations: BTreeMap<PathBuf, Annotation>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Annotation {
    pub rating: u8,        // 0-5
    pub tags: Vec<String>,
    pub caption: String,
}

impl<F: ImageFs> ImageViewer<F> {
    pub fn new(fs: F) -> Self {
        Self {
            fs, gallery: vec![], index: 0, sort: SortOrder::Name,
            slideshow_secs: 5, show_hidden: false, annotations: BTreeMap::new(),
        }
    }

    /// Load a directory into the gallery.
    pub fn load_dir(&mut self, dir: &Path) -> Result<usize, ViewError> {
        let entries = self.fs.list_dir(dir).map_err(|e| ViewError::Io(e))?;
        let mut imgs = vec![];
        for p in entries {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !self.show_hidden && name.starts_with('.') { continue; }
            if self.fs.metadata(&p).is_err() { continue; }
            if let Ok(info) = self.inspect(&p) { imgs.push(info); }
        }
        self.sort_gallery(&mut imgs);
        let n = imgs.len();
        self.gallery = imgs;
        self.index = 0;
        Ok(n)
    }

    /// Inspect a single image file.
    pub fn inspect(&self, path: &Path) -> Result<ImageInfo, ViewError> {
        let head = self.fs.read_head(path, 512).map_err(ViewError::Io)?;
        let fmt = ImageFormat::sniff(&head)
            .or_else(|| ImageFormat::from_ext(path))
            .ok_or_else(|| ViewError::NotImage(path.to_path_buf()))?;
        let (size, _mtime) = self.fs.metadata(path).map_err(ViewError::Io)?;
        let (width, height) = self.fs.read_all(path)
            .ok()
            .and_then(|d| parse_dimensions(fmt, &d))
            .unwrap_or((0, 0));
        let animated = fmt == ImageFormat::Gif
            && self.fs.read_all(path).map(|d| count_gif_frames(&d) > 1).unwrap_or(false);
        Ok(ImageInfo {
            path: path.to_path_buf(), format: fmt,
            width, height, size_bytes: size,
            color_depth: None, animated,
        })
    }

    fn sort_gallery(&self, imgs: &mut Vec<ImageInfo>) {
        match self.sort {
            SortOrder::Name => imgs.sort_by(|a,b| a.path.cmp(&b.path)),
            SortOrder::NameDesc => imgs.sort_by(|a,b| b.path.cmp(&a.path)),
            SortOrder::Size => imgs.sort_by_key(|i| i.size_bytes),
            SortOrder::SizeDesc => imgs.sort_by_key(|i| std::cmp::Reverse(i.size_bytes)),
            SortOrder::Modified => imgs.sort_by_key(|i| {
                self.fs.metadata(&i.path).map(|m| m.1).unwrap_or(0)
            }),
            SortOrder::ModifiedDesc => imgs.sort_by_key(|i| {
                std::cmp::Reverse(self.fs.metadata(&i.path).map(|m| m.1).unwrap_or(0))
            }),
        }
    }

    /// Current image.
    pub fn current(&self) -> Option<&ImageInfo> {
        self.gallery.get(self.index)
    }

    /// Navigate.
    pub fn next(&mut self) -> Result<(), ViewError> {
        if self.gallery.is_empty() { return Err(ViewError::EmptyGallery); }
        self.index = (self.index + 1) % self.gallery.len();
        Ok(())
    }
    pub fn prev(&mut self) -> Result<(), ViewError> {
        if self.gallery.is_empty() { return Err(ViewError::EmptyGallery); }
        self.index = if self.index == 0 { self.gallery.len() - 1 } else { self.index - 1 };
        Ok(())
    }
    pub fn first(&mut self) { self.index = 0; }
    pub fn last(&mut self) {
        if !self.gallery.is_empty() { self.index = self.gallery.len() - 1; }
    }
    pub fn go_to(&mut self, i: usize) -> Result<(), ViewError> {
        if i >= self.gallery.len() { return Err(ViewError::BadIndex(i)); }
        self.index = i;
        Ok(())
    }

    /// Set annotation for current image.
    pub fn annotate(&mut self, rating: u8, tags: Vec<String>, caption: impl Into<String>) {
        if let Some(img) = self.current() {
            self.annotations.insert(img.path.clone(), Annotation {
                rating: rating.min(5), tags, caption: caption.into(),
            });
        }
    }

    /// Images matching a tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&ImageInfo> {
        self.gallery.iter().filter(|i| {
            self.annotations.get(&i.path)
                .map(|a| a.tags.iter().any(|t| t == tag)).unwrap_or(false)
        }).collect()
    }

    /// Highly-rated images (rating >= n).
    pub fn top_rated(&self, min: u8) -> Vec<&ImageInfo> {
        self.gallery.iter().filter(|i| {
            self.annotations.get(&i.path).map(|a| a.rating >= min).unwrap_or(false)
        }).collect()
    }

    /// Filter gallery by format.
    pub fn by_format(&self, fmt: ImageFormat) -> Vec<&ImageInfo> {
        self.gallery.iter().filter(|i| i.format == fmt).collect()
    }

    /// Summary stats.
    pub fn stats(&self) -> ViewerStats {
        let mut by_fmt: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut total_bytes = 0u64;
        let mut animated = 0;
        for i in &self.gallery {
            *by_fmt.entry(i.format.name()).or_default() += 1;
            total_bytes += i.size_bytes;
            if i.animated { animated += 1; }
        }
        ViewerStats {
            count: self.gallery.len(), total_bytes,
            by_format: by_fmt.into_iter().map(|(k,v)| (k.to_string(),v)).collect(),
            animated, annotated: self.annotations.len(),
        }
    }

    /// Render a text info card for current image.
    pub fn info_card(&self) -> Option<String> {
        let img = self.current()?;
        let ann = self.annotations.get(&img.path);
        let mut s = format!("{} ({})", img.path.display(), img.format.name());
        s.push_str(&format!("\n  {}x{}  {}  {}", img.width, img.height,
            img.human_size(), img.megapixels().round() as u64));
        if img.animated { s.push_str("  [animated]"); }
        if let Some(a) = ann {
            if a.rating > 0 { s.push_str(&format!("\n  rating: {}/5", a.rating)); }
            if !a.tags.is_empty() { s.push_str(&format!("\n  tags: {}", a.tags.join(", "))); }
            if !a.caption.is_empty() { s.push_str(&format!("\n  {}", a.caption)); }
        }
        Some(s)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ViewerStats {
    pub count: usize,
    pub total_bytes: u64,
    pub by_format: BTreeMap<String, usize>,
    pub animated: usize,
    pub annotated: usize,
}

/// Count GIF frames by counting image descriptor separators (0x2C after GCT).
fn count_gif_frames(data: &[u8]) -> usize {
    // rough heuristic: count 0x21 0xF9 (GCE) blocks
    let mut n = 0;
    for w in data.windows(2) {
        if w == [0x21, 0xF9] { n += 1; }
    }
    n.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let mut d = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        d.extend_from_slice(b"IHDR");
        d.extend_from_slice(&13u32.to_be_bytes());
        d.extend_from_slice(&w.to_be_bytes());
        d.extend_from_slice(&h.to_be_bytes());
        d.extend_from_slice(&[8, 6, 0, 0, 0]);
        d.extend_from_slice(&[0,0,0,0]); // crc placeholder
        d
    }
    fn gif_bytes(w: u16, h: u16) -> Vec<u8> {
        let mut d = b"GIF89a".to_vec();
        d.extend_from_slice(&w.to_le_bytes());
        d.extend_from_slice(&h.to_le_bytes());
        d.extend_from_slice(&[0xF0, 0, 0]); // gct flag
        d.extend_from_slice(&[0; 6]); // gct
        d
    }
    fn jpeg_bytes(w: u16, h: u16) -> Vec<u8> {
        let mut d = vec![0xFF, 0xD8];
        // SOF0
        d.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        d.extend_from_slice(&h.to_be_bytes());
        d.extend_from_slice(&w.to_be_bytes());
        d.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        d.extend_from_slice(&[0xFF, 0xD9]);
        d
    }

    fn fs_with(files: Vec<(PathBuf, Vec<u8>)>) -> MemFs {
        let mut fs = MemFs::default();
        for (p, d) in files {
            let sz = d.len() as u64;
            fs.meta.insert(p.clone(), (sz, 1000));
            fs.files.insert(p, d);
        }
        fs
    }

    #[test]
    fn format_sniffing() {
        assert_eq!(ImageFormat::sniff(&png_bytes(1,1)), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::sniff(&gif_bytes(1,1)), Some(ImageFormat::Gif));
        assert_eq!(ImageFormat::sniff(&jpeg_bytes(1,1)), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::sniff(b"not an image"), None);
        // svg text
        assert_eq!(ImageFormat::sniff(b"<svg xmlns=...></svg>"), Some(ImageFormat::Svg));
    }

    #[test]
    fn format_from_ext() {
        assert_eq!(ImageFormat::from_ext(Path::new("a.PNG")), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_ext(Path::new("b.jpeg")), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_ext(Path::new("c.txt")), None);
    }

    #[test]
    fn png_dimensions() {
        let d = png_bytes(800, 600);
        assert_eq!(parse_dimensions(ImageFormat::Png, &d), Some((800, 600)));
    }

    #[test]
    fn gif_dimensions() {
        let d = gif_bytes(320, 240);
        assert_eq!(parse_dimensions(ImageFormat::Gif, &d), Some((320, 240)));
    }

    #[test]
    fn jpeg_dimensions() {
        let d = jpeg_bytes(1024, 768);
        assert_eq!(parse_dimensions(ImageFormat::Jpeg, &d), Some((1024, 768)));
    }

    #[test]
    fn gallery_load_and_nav() {
        let fs = fs_with(vec![
            (PathBuf::from("/pics/a.png"), png_bytes(10, 10)),
            (PathBuf::from("/pics/b.png"), png_bytes(20, 20)),
            (PathBuf::from("/pics/c.gif"), gif_bytes(30, 30)),
            (PathBuf::from("/pics/skip.txt"), b"text".to_vec()),
        ]);
        // list_dir returns files under /pics — MemFs filters by parent
        let mut v = ImageViewer::new(fs);
        // inject gallery directly since MemFs.list_dir needs parent match
        let n = v.load_dir(Path::new("/pics")).unwrap();
        assert_eq!(n, 3);
        // sorted by path: a.png, b.png, c.gif → current is a.png
        assert_eq!(v.current().unwrap().path.file_name().unwrap(), "a.png");
        v.next().unwrap();
        assert_eq!(v.current().unwrap().path.file_name().unwrap(), "b.png");
        v.prev().unwrap();
        assert_eq!(v.current().unwrap().path.file_name().unwrap(), "a.png");
        v.last();
        assert_eq!(v.current().unwrap().path.file_name().unwrap(), "c.gif");
        v.first();
    }

    #[test]
    fn sort_orders() {
        let fs = fs_with(vec![
            (PathBuf::from("/p/b.png"), png_bytes(1,1)),
            (PathBuf::from("/p/a.png"), png_bytes(1,1)),
        ]);
        fs.files.get(&PathBuf::from("/p/b.png"));
        let mut v = ImageViewer::new(fs);
        v.sort = SortOrder::NameDesc;
        v.load_dir(Path::new("/p")).unwrap();
        assert_eq!(v.gallery[0].path.file_name().unwrap(), "b.png");
    }

    #[test]
    fn annotations_and_tags() {
        let fs = fs_with(vec![(PathBuf::from("/p/a.png"), png_bytes(10,10))]);
        let mut v = ImageViewer::new(fs);
        v.load_dir(Path::new("/p")).unwrap();
        v.annotate(5, vec!["vacation".into()], "beach day");
        assert_eq!(v.by_tag("vacation").len(), 1);
        assert_eq!(v.top_rated(4).len(), 1);
        assert_eq!(v.top_rated(5).len(), 1);
        let card = v.info_card().unwrap();
        assert!(card.contains("rating: 5/5"));
        assert!(card.contains("vacation"));
    }

    #[test]
    fn stats_summary() {
        let fs = fs_with(vec![
            (PathBuf::from("/p/a.png"), png_bytes(10,10)),
            (PathBuf::from("/p/b.gif"), gif_bytes(5,5)),
        ]);
        let mut v = ImageViewer::new(fs);
        v.load_dir(Path::new("/p")).unwrap();
        let s = v.stats();
        assert_eq!(s.count, 2);
        assert!(s.by_format.contains_key("png"));
        assert!(s.by_format.contains_key("gif"));
    }

    #[test]
    fn bad_index_rejected() {
        let fs = fs_with(vec![(PathBuf::from("/p/a.png"), png_bytes(1,1))]);
        let mut v = ImageViewer::new(fs);
        v.load_dir(Path::new("/p")).unwrap();
        assert!(v.go_to(5).is_err());
        assert!(v.go_to(0).is_ok());
    }

    #[test]
    fn empty_gallery_nav_errors() {
        let fs = MemFs::default();
        let mut v = ImageViewer::new(fs);
        assert!(v.next().is_err());
        assert!(v.prev().is_err());
    }

    #[test]
    fn image_info_helpers() {
        let info = ImageInfo {
            path: PathBuf::from("/x.png"), format: ImageFormat::Png,
            width: 4000, height: 3000, size_bytes: 5_000_000,
            color_depth: Some(8), animated: false,
        };
        assert!(info.is_landscape());
        assert!((info.aspect() - 1.333).abs() < 0.01);
        assert_eq!(info.megapixels(), 12.0);
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(5_000_000), "4.8 MiB");
    }

    #[test]
    fn hidden_files_skipped() {
        let fs = fs_with(vec![
            (PathBuf::from("/p/.hidden.png"), png_bytes(1,1)),
            (PathBuf::from("/p/visible.png"), png_bytes(1,1)),
        ]);
        let mut v = ImageViewer::new(fs);
        v.load_dir(Path::new("/p")).unwrap();
        assert_eq!(v.gallery.len(), 1);
        v.show_hidden = true;
        v.load_dir(Path::new("/p")).unwrap();
        assert_eq!(v.gallery.len(), 2);
    }

    #[test]
    fn bmp_and_qoi_dims() {
        // BMP: "BM" + size(4) + reserved(4) + offset(4) + dibsize(4) + w(4) + h(4)
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&[0;4]); bmp.extend_from_slice(&[0;4]);
        bmp.extend_from_slice(&[0;4]); bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&640i32.to_le_bytes());
        bmp.extend_from_slice(&480i32.to_le_bytes());
        bmp.resize(30, 0);
        assert_eq!(parse_dimensions(ImageFormat::Bmp, &bmp), Some((640, 480)));
        // QOI: magic + w + h BE
        let mut qoi = b"qoif".to_vec();
        qoi.extend_from_slice(&100u32.to_be_bytes());
        qoi.extend_from_slice(&200u32.to_be_bytes());
        qoi.extend_from_slice(&[0;10]);
        assert_eq!(parse_dimensions(ImageFormat::Qoi, &qoi), Some((100, 200)));
    }
}
