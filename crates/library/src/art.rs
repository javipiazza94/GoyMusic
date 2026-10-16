//! Album covers: the picture embedded in the album's first track, or a `cover.*` / `folder.*`
//! image beside it, resized to a WebP thumbnail cached on disk.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use goymusic_core::AlbumId;
use image::codecs::webp::WebPEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType};
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;

use crate::db::Db;
use crate::error::Result;
use crate::queries;

const COVER_STEMS: &[&str] = &["cover", "folder"];
const COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtSize {
    Small,
    Large,
}

impl ArtSize {
    pub fn px(self) -> u32 {
        match self {
            ArtSize::Small => 200,
            ArtSize::Large => 600,
        }
    }

    pub fn from_px(px: u32) -> Option<ArtSize> {
        [ArtSize::Small, ArtSize::Large].into_iter().find(|s| s.px() == px)
    }
}

enum Source {
    Embedded { audio: PathBuf, data: Vec<u8> },
    File(PathBuf),
}

impl Source {
    /// The file whose modification invalidates the cached thumbnail.
    fn origin(&self) -> &Path {
        match self {
            Source::Embedded { audio, .. } => audio,
            Source::File(path) => path,
        }
    }

    fn decode(&self) -> Option<DynamicImage> {
        match self {
            Source::Embedded { data, .. } => image::load_from_memory(data).ok(),
            Source::File(path) => image::ImageReader::open(path)
                .ok()?
                .with_guessed_format()
                .ok()?
                .decode()
                .ok(),
        }
    }
}

/// Path of the album's cached thumbnail, creating or refreshing it when needed. `None` when
/// the album has no art or its image cannot be decoded.
pub fn album_art(db: &Db, album: AlbumId, size: ArtSize, cache_dir: &Path) -> Result<Option<PathBuf>> {
    let Some(first) = queries::album_tracks(db, album)?.into_iter().next() else {
        return Ok(None);
    };
    let Some(source) = find_source(Path::new(&first.path)) else {
        return Ok(None);
    };
    let target = cache_dir.join(format!("{}-{}.webp", album.0, size.px()));
    if is_fresh(&target, source.origin()) {
        return Ok(Some(target));
    }
    let Some(image) = source.decode() else {
        return Ok(None);
    };
    std::fs::create_dir_all(cache_dir)?;
    write_thumbnail(&image, size.px(), &target)?;
    Ok(Some(target))
}

fn find_source(track: &Path) -> Option<Source> {
    if let Some(data) = embedded_picture(track) {
        return Some(Source::Embedded {
            audio: track.to_owned(),
            data,
        });
    }
    folder_cover(track.parent()?).map(Source::File)
}

fn embedded_picture(track: &Path) -> Option<Vec<u8>> {
    let tagged = lofty::read_from_path(track).ok()?;
    let pictures: Vec<_> = tagged.tags().iter().flat_map(|t| t.pictures()).collect();
    pictures
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first())
        .map(|p| p.data().to_vec())
}

/// `cover.*` before `folder.*`, extension and case ignored.
fn folder_cover(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<(usize, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?.to_lowercase();
            let ext = path.extension()?.to_str()?.to_lowercase();
            let rank = COVER_STEMS.iter().position(|s| *s == stem)?;
            COVER_EXTENSIONS.contains(&ext.as_str()).then_some((rank, path))
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next().map(|(_, path)| path)
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn is_fresh(target: &Path, origin: &Path) -> bool {
    match (modified(target), modified(origin)) {
        (Some(cached), Some(source)) => cached >= source,
        (Some(_), None) => true,
        _ => false,
    }
}

/// Fits the image within `px`×`px` (never enlarging it) and writes lossless WebP atomically.
fn write_thumbnail(image: &DynamicImage, px: u32, target: &Path) -> Result<()> {
    let resized = if image.width() > px || image.height() > px {
        image.resize(px, px, FilterType::Lanczos3)
    } else {
        image.clone()
    };
    let rgba = resized.to_rgba8();
    let mut bytes = Vec::new();
    WebPEncoder::new_lossless(&mut bytes)
        .encode(rgba.as_raw(), rgba.width(), rgba.height(), ExtendedColorType::Rgba8)
        .map_err(std::io::Error::other)?;
    let tmp = target.with_extension("webp.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, target)?;
    Ok(())
}
