mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::{embed_cover, png_bytes, write_png, write_tagged_wav, Meta};
use goymusic_core::events::EventBus;
use goymusic_core::AlbumId;
use goymusic_library::art::{album_art, ArtSize};
use goymusic_library::db::Db;
use goymusic_library::{folders, queries, scanner};

fn album_in(dir: &Path) -> PathBuf {
    let path = dir.join("album/01.wav");
    write_tagged_wav(
        &path,
        0.2,
        &Meta {
            album: Some("Covers"),
            track: Some(1),
            ..Meta::new("One", "Ana")
        },
    );
    path
}

fn scanned(dir: &Path) -> (Db, AlbumId) {
    let mut db = Db::open_in_memory().unwrap();
    folders::add(&db, dir).unwrap();
    scanner::scan(&mut db, &EventBus::new()).unwrap();
    let album = queries::albums(&db).unwrap()[0].id;
    (db, album)
}

fn dimensions(path: &Path) -> (u32, u32) {
    let reader = image::ImageReader::open(path).unwrap().with_guessed_format().unwrap();
    assert_eq!(reader.format(), Some(image::ImageFormat::WebP));
    let img = reader.decode().unwrap();
    (img.width(), img.height())
}

#[test]
fn embedded_cover_becomes_a_webp_thumbnail() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    embed_cover(&album_in(music.path()), png_bytes(400, 300));
    let (db, album) = scanned(music.path());
    let thumb = album_art(&db, album, ArtSize::Small, cache.path()).unwrap().unwrap();
    assert!(thumb.starts_with(cache.path()));
    assert_eq!(dimensions(&thumb), (200, 150));
}

#[test]
fn folder_cover_is_used_without_embedded_art() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    album_in(music.path());
    write_png(&music.path().join("album/Cover.JPG.png"), 10, 10); // not a cover name: ignored
    image::RgbImage::from_pixel(1200, 800, image::Rgb([1, 2, 3]))
        .save_with_format(music.path().join("album/Cover.JPG"), image::ImageFormat::Jpeg)
        .unwrap();
    let (db, album) = scanned(music.path());
    let thumb = album_art(&db, album, ArtSize::Large, cache.path()).unwrap().unwrap();
    assert_eq!(dimensions(&thumb), (600, 400));
}

#[test]
fn small_images_are_not_upscaled() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    album_in(music.path());
    write_png(&music.path().join("album/folder.png"), 100, 100);
    let (db, album) = scanned(music.path());
    let thumb = album_art(&db, album, ArtSize::Large, cache.path()).unwrap().unwrap();
    assert_eq!(dimensions(&thumb), (100, 100));
}

#[test]
fn album_without_art_has_none() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    album_in(music.path());
    let (db, album) = scanned(music.path());
    assert_eq!(album_art(&db, album, ArtSize::Small, cache.path()).unwrap(), None);
}

#[test]
fn unreadable_cover_is_ignored() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    album_in(music.path());
    std::fs::write(music.path().join("album/cover.png"), b"not an image").unwrap();
    let (db, album) = scanned(music.path());
    assert_eq!(album_art(&db, album, ArtSize::Small, cache.path()).unwrap(), None);
}

#[test]
fn cache_is_reused_then_refreshed_when_the_source_changes() {
    let music = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    album_in(music.path());
    let cover = music.path().join("album/cover.png");
    write_png(&cover, 300, 300);
    let (db, album) = scanned(music.path());

    let thumb = album_art(&db, album, ArtSize::Small, cache.path()).unwrap().unwrap();
    let first = std::fs::metadata(&thumb).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    album_art(&db, album, ArtSize::Small, cache.path()).unwrap().unwrap();
    assert_eq!(
        std::fs::metadata(&thumb).unwrap().modified().unwrap(),
        first,
        "cache reused"
    );

    write_png(&cover, 300, 150);
    let later = SystemTime::now() + Duration::from_secs(5);
    std::fs::File::options()
        .write(true)
        .open(&cover)
        .unwrap()
        .set_modified(later)
        .unwrap();
    album_art(&db, album, ArtSize::Small, cache.path()).unwrap().unwrap();
    assert_eq!(dimensions(&thumb), (200, 100), "regenerated from the new cover");
}

#[test]
fn sizes_map_to_pixels() {
    assert_eq!(ArtSize::Small.px(), 200);
    assert_eq!(ArtSize::Large.px(), 600);
    assert_eq!(ArtSize::from_px(600), Some(ArtSize::Large));
    assert_eq!(ArtSize::from_px(300), None);
}
