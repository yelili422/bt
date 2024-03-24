DROP TABLE IF EXISTS rss;

CREATE TABLE rss
(
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    url      TEXT              NOT NULL,
    title    TEXT,
    rss_type TEXT              NOT NULL,
    enabled  INTEGER DEFAULT 1 NOT NULL,
    season   INTEGER
);

-- INSERT INTO rss (url, title, rss_type, enabled)
-- VALUES ('https://mikanani.me/RSS/Bangumi?bangumiId=3141&subgroupid=615', 'My Bangumi', 'mikan', 1),
--        ('https://mikanani.me/RSS/Bangumi?bangumiId=3231&subgroupid=583', '魔都精兵的奴隶', 'mikan', 1),
--        ('https://mikanani.me/RSS/Bangumi?bangumiId=3240&subgroupid=370', '迷宫饭', 'mikan', 1);

DROP TABLE IF EXISTS download_task;

CREATE TABLE download_task
(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    torrent_hash  TEXT              NOT NULL,
    torrent_url   TEXT,
    start_time    TEXT              NOT NULL,
    status        TEXT              NOT NULL,
    show_name     TEXT              NOT NULL,
    episode_name  TEXT,
    display_name  TEXT,
    season        INTEGER,
    episode       INTEGER,
    category      TEXT,
    download_path TEXT,
    renamed       INTEGER DEFAULT 0 NOT NULL
);
