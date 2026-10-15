CREATE TABLE folders (
    id   INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE
);

CREATE TABLE artists (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE albums (
    id            INTEGER PRIMARY KEY,
    title         TEXT NOT NULL,
    artist_id     INTEGER REFERENCES artists(id) ON DELETE SET NULL,
    mb_release_id TEXT,
    year          INTEGER,
    group_key     TEXT NOT NULL UNIQUE
);

CREATE TABLE tracks (
    id              INTEGER PRIMARY KEY,
    path            TEXT NOT NULL UNIQUE,
    folder_id       INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    size            INTEGER NOT NULL,
    mtime           INTEGER NOT NULL,
    title           TEXT NOT NULL,
    album_id        INTEGER REFERENCES albums(id) ON DELETE SET NULL,
    album_artist    TEXT,
    track_no        INTEGER,
    disc_no         INTEGER,
    year            INTEGER,
    genre           TEXT,
    duration_ms     INTEGER NOT NULL,
    mb_recording_id TEXT
);
CREATE INDEX tracks_album ON tracks(album_id);
CREATE INDEX tracks_folder ON tracks(folder_id);

CREATE TABLE track_artists (
    track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    PRIMARY KEY (track_id, position)
);
CREATE INDEX track_artists_artist ON track_artists(artist_id);

CREATE TABLE playlist_folders (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL,
    parent_id INTEGER REFERENCES playlist_folders(id) ON DELETE CASCADE
);

CREATE TABLE playlists (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    folder_id  INTEGER REFERENCES playlist_folders(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE playlist_items (
    id          INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    pos         INTEGER NOT NULL,
    track_id    INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE
);
CREATE INDEX playlist_items_order ON playlist_items(playlist_id, pos);

CREATE TABLE likes (
    id       INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL UNIQUE REFERENCES tracks(id) ON DELETE CASCADE,
    liked_at INTEGER NOT NULL
);

CREATE TABLE play_history (
    id        INTEGER PRIMARY KEY,
    track_id  INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    played_at INTEGER NOT NULL
);

CREATE TABLE queue_state (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    snapshot    TEXT NOT NULL,
    position_ms INTEGER NOT NULL
);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE VIRTUAL TABLE tracks_fts USING fts5(
    title, artists, album,
    tokenize = 'unicode61 remove_diacritics 2'
);
