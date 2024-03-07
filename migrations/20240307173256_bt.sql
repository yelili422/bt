DROP TABLE IF EXISTS rss;

CREATE TABLE rss
(
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    url      TEXT NOT NULL,
    title    TEXT,
    rss_type TEXT NOT NULL
);

DROP TABLE IF EXISTS download_task;

CREATE TABLE download_task
(
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    torrent_hash TEXT NOT NULL,
    torrent_url  TEXT,
    start_time   TEXT NOT NULL,
    end_time     TEXT,
    status       TEXT NOT NULL,
    show_name    TEXT NOT NULL,
    episode_name TEXT,
    display_name TEXT,
    season       INTEGER,
    episode      INTEGER,
    category     TEXT
);

