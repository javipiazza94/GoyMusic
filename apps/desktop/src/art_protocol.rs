//! `art://localhost/album/{id}/{200|600}`: album cover thumbnails for the UI, served from
//! the cache without base64 (spec §6).

use std::sync::Arc;

use goymusic_core::AlbumId;
use goymusic_library::art::ArtSize;
use goymusic_services::Services;
use tauri::http::{header, Response, StatusCode};
use tauri::UriSchemeResponder;

/// `/album/{id}/{px}` → album and size; anything else is not ours.
pub fn parse(path: &str) -> Option<(AlbumId, ArtSize)> {
    let mut parts = path.strip_prefix("/album/")?.split('/');
    let id = parts.next()?.parse().ok()?;
    let size = ArtSize::from_px(parts.next()?.parse().ok()?)?;
    parts.next().is_none().then_some((AlbumId(id), size))
}

/// Answers on its own thread: generating a thumbnail can take a while and must never block
/// the main (UI) thread.
pub fn respond(services: Arc<Services>, path: String, responder: UriSchemeResponder) {
    std::thread::spawn(move || {
        let response = match parse(&path).map(|(id, size)| services.album_art(id, size)) {
            Some(Ok(Some(file))) => match std::fs::read(&file) {
                Ok(bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "image/webp")
                    .header(header::CACHE_CONTROL, "max-age=86400")
                    .body(bytes),
                Err(e) => error(e.to_string()),
            },
            Some(Ok(None)) | None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
            Some(Err(e)) => error(e.to_string()),
        };
        responder.respond(response.expect("valid art response"));
    });
}

fn error(message: String) -> tauri::http::Result<Response<Vec<u8>>> {
    tracing::warn!(%message, "album art failed");
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn album_paths_are_parsed() {
        assert_eq!(parse("/album/12/200"), Some((AlbumId(12), ArtSize::Small)));
        assert_eq!(parse("/album/12/600"), Some((AlbumId(12), ArtSize::Large)));
    }

    #[test]
    fn other_paths_are_rejected() {
        for path in [
            "/album/x/200",
            "/album/12/300",
            "/track/1/200",
            "",
            "/album/12",
            "/album/12/200/extra",
        ] {
            assert_eq!(parse(path), None, "{path}");
        }
    }
}
