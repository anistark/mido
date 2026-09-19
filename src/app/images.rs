use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageReader, Limits};
use ratatui::layout::Size;
use ratatui_image::Resize;
use ratatui_image::picker::Picker;
use ratatui_image::sliced::SlicedProtocol;

use crate::render::layout::ImageSizes;

const MAX_REMOTE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_DIMENSION: u32 = 8192;
const MAX_ALLOC: u64 = 256 * 1024 * 1024;

#[derive(Default)]
pub struct Images {
    picker: Option<Picker>,
    pub enabled: bool,
    pub remote: bool,
    loaded: HashMap<String, Option<DynamicImage>>,
    protocols: HashMap<(String, u16, u16), SlicedProtocol>,
    pub notice: Option<String>,
}

impl Images {
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    pub fn set_picker(&mut self, picker: Picker) {
        self.picker = Some(picker);
        self.protocols.clear();
    }

    pub fn available(&self) -> bool {
        self.picker.is_some()
    }

    pub fn active(&self) -> bool {
        self.enabled && self.picker.is_some()
    }

    pub fn protocol_name(&self) -> String {
        self.picker
            .as_ref()
            .map(|p| format!("{:?}", p.protocol_type()).to_lowercase())
            .unwrap_or_else(|| "none".to_string())
    }

    pub fn sizes(&mut self, urls: &[String], base: &Path) -> Option<ImageSizes> {
        if !self.active() {
            return None;
        }
        let font = self.picker.as_ref()?.font_size();
        let mut sizes = HashMap::new();
        for url in urls {
            if let Some(img) = self.load(url, base) {
                sizes.insert(url.clone(), (img.width(), img.height()));
            }
        }
        Some(ImageSizes {
            font: (font.width, font.height),
            sizes,
        })
    }

    fn key(url: &str, base: &Path) -> String {
        if is_remote(url) {
            url.to_string()
        } else {
            base.join(url).display().to_string()
        }
    }

    fn load(&mut self, url: &str, base: &Path) -> Option<&DynamicImage> {
        let key = Self::key(url, base);
        if !self.loaded.contains_key(&key) {
            let result = if is_remote(url) {
                if self.remote {
                    fetch(url)
                } else {
                    Err("remote images are off, start with --remote-images".to_string())
                }
            } else {
                open(&base.join(url))
            };
            let image = match result {
                Ok(image) => Some(image),
                Err(err) => {
                    self.notice = Some(format!("image {url}: {err}"));
                    None
                }
            };
            self.loaded.insert(key.clone(), image);
        }
        self.loaded.get(&key)?.as_ref()
    }

    pub fn protocol(
        &mut self,
        url: &str,
        base: &Path,
        cols: u16,
        rows: u16,
    ) -> Option<&SlicedProtocol> {
        let key = Self::key(url, base);
        let cache_key = (key.clone(), cols, rows);
        if !self.protocols.contains_key(&cache_key) {
            let picker = self.picker.as_ref()?;
            let image = self.loaded.get(&key)?.as_ref()?.clone();
            let protocol = SlicedProtocol::new_with_resize(
                picker,
                image,
                Size::new(cols, rows),
                Resize::Fit(None),
            )
            .ok()?;
            self.protocols.insert(cache_key.clone(), protocol);
        }
        self.protocols.get(&cache_key)
    }
}

fn is_remote(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOC);
    limits
}

fn open(path: &Path) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::open(path)
        .and_then(ImageReader::with_guessed_format)
        .map_err(|err| err.to_string())?;
    reader.limits(limits());
    reader.decode().map_err(|err| err.to_string())
}

fn decode(bytes: Vec<u8>) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| err.to_string())?;
    reader.limits(limits());
    reader.decode().map_err(|err| err.to_string())
}

fn cache_path(url: &str) -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("", "", "mido")?;
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    Some(
        dirs.cache_dir()
            .join("images")
            .join(format!("{:016x}", hasher.finish())),
    )
}

fn fetch(url: &str) -> Result<DynamicImage, String> {
    let cache = cache_path(url);
    if let Some(path) = &cache
        && let Ok(bytes) = std::fs::read(path)
    {
        return decode(bytes);
    }
    let mut response = ureq::get(url).call().map_err(|err| err.to_string())?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_REMOTE_BYTES)
        .read_to_vec()
        .map_err(|err| err.to_string())?;
    if let Some(path) = &cache
        && let Some(dir) = path.parent()
        && std::fs::create_dir_all(dir).is_ok()
    {
        let _ = std::fs::write(path, &bytes);
    }
    decode(bytes)
}
