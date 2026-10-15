use serde::{Deserialize, Serialize};

macro_rules! id_types {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
            #[serde(transparent)]
            pub struct $name(pub i64);
        )*
    };
}

id_types!(TrackId, AlbumId, ArtistId, PlaylistId, PlaylistFolderId, FolderId);
