use crate::models::*;
use rusqlite::{Connection, params};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> StorageResult<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn };
        db.run_migrations(true)?;
        Ok(db)
    }

    pub fn open_in_memory() -> StorageResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Database { conn };
        db.run_migrations(false)?;
        Ok(db)
    }

    fn run_migrations(&self, use_wal: bool) -> StorageResult<()> {
        if use_wal {
            self.conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        }
        self.conn.execute_batch("
            PRAGMA foreign_keys=ON;

            CREATE TABLE IF NOT EXISTS providers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                provider_type TEXT NOT NULL,
                url TEXT NOT NULL,
                username TEXT,
                password TEXT
            );

            CREATE TABLE IF NOT EXISTS channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                group_title TEXT,
                logo_url TEXT,
                stream_url TEXT NOT NULL,
                tvg_id TEXT,
                hidden INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_channels_provider ON channels(provider_id);
            CREATE INDEX IF NOT EXISTS idx_channels_group ON channels(group_title);
            CREATE INDEX IF NOT EXISTS idx_channels_hidden ON channels(hidden);

            CREATE VIRTUAL TABLE IF NOT EXISTS channels_fts USING fts5(
                name, group_title,
                content='channels', content_rowid='id',
                tokenize='unicode61'
            );

            CREATE TABLE IF NOT EXISTS favorites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id INTEGER NOT NULL UNIQUE REFERENCES channels(id) ON DELETE CASCADE,
                added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );

            CREATE TABLE IF NOT EXISTS filters (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                group_filter TEXT,
                name_filter TEXT,
                show_hidden INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS epg (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_tvg_id TEXT NOT NULL,
                title TEXT NOT NULL,
                start_ts INTEGER NOT NULL,
                end_ts INTEGER NOT NULL,
                description TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_epg_channel ON epg(channel_tvg_id, start_ts);

            CREATE TABLE IF NOT EXISTS vod (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                cover_url TEXT,
                stream_url TEXT NOT NULL,
                genre TEXT,
                rating TEXT,
                year TEXT
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS vod_fts USING fts5(
                name, genre,
                content='vod', content_rowid='id',
                tokenize='unicode61'
            );

            CREATE TABLE IF NOT EXISTS series (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                cover_url TEXT,
                genre TEXT
            );

            CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                series_id INTEGER NOT NULL REFERENCES series(id) ON DELETE CASCADE,
                season INTEGER NOT NULL,
                episode INTEGER NOT NULL,
                name TEXT NOT NULL,
                stream_url TEXT NOT NULL
            );
        ")?;
        Ok(())
    }

    pub fn insert_provider(&self, p: &Provider) -> StorageResult<i64> {
        let type_str = match p.provider_type {
            ProviderType::XtreamCodes => "xtream",
            ProviderType::M3U => "m3u",
        };
        self.conn.execute(
            "INSERT INTO providers (name, provider_type, url, username, password) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![p.name, type_str, p.url, p.username, p.password],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Batch insert channels. Returns count inserted.
    pub fn insert_channels_batch(&self, channels: &[Channel]) -> StorageResult<usize> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO channels (provider_id, name, group_title, logo_url, stream_url, tvg_id, hidden, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;
        let mut count = 0;
        for ch in channels {
            stmt.execute(params![
                ch.provider_id, ch.name, ch.group_title, ch.logo_url,
                ch.stream_url, ch.tvg_id, ch.hidden as i32, ch.sort_order
            ])?;
            count += 1;
        }
        Ok(count)
    }

    pub fn rebuild_channels_fts(&self) -> StorageResult<()> {
        self.conn.execute_batch("
            INSERT INTO channels_fts(channels_fts) VALUES ('rebuild');
        ")?;
        Ok(())
    }

    pub fn get_channels_page(
        &self,
        page: u32,
        page_size: u32,
        opts: &FilterOptions,
    ) -> StorageResult<ChannelPage> {
        let offset = page * page_size;

        // Build WHERE conditions
        let mut conditions = Vec::new();
        let mut bind_params: Vec<String> = Vec::new();

        if !opts.show_hidden {
            conditions.push("c.hidden = 0".to_string());
        }

        if let Some(q) = &opts.search_query {
            if !q.is_empty() {
                let idx = bind_params.len() + 1;
                conditions.push(format!("(c.name LIKE ?{} OR c.group_title LIKE ?{})", idx, idx));
                bind_params.push(format!("%{}%", q));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Count query
        let count_sql = format!(
            "SELECT COUNT(*) FROM channels c LEFT JOIN favorites f ON f.channel_id = c.id {}",
            where_clause
        );

        let total: u32 = {
            let mut stmt = self.conn.prepare(&count_sql)?;
            stmt.query_row(
                rusqlite::params_from_iter(bind_params.iter()),
                |r| r.get(0)
            )?
        };

        // Data query
        let page_idx = bind_params.len() + 1;
        let offset_idx = bind_params.len() + 2;
        let data_sql = format!(
            "SELECT c.id, c.name, c.group_title, c.logo_url, c.hidden,
                    (CASE WHEN f.id IS NOT NULL THEN 1 ELSE 0 END) as is_fav
             FROM channels c
             LEFT JOIN favorites f ON f.channel_id = c.id
             {}
             ORDER BY c.sort_order, c.name
             LIMIT ?{} OFFSET ?{}",
            where_clause, page_idx, offset_idx
        );

        let mut all_params: Vec<Box<dyn rusqlite::ToSql>> = bind_params
            .iter()
            .map(|s| Box::new(s.clone()) as Box<dyn rusqlite::ToSql>)
            .collect();
        all_params.push(Box::new(page_size as i64));
        all_params.push(Box::new(offset as i64));

        let mut stmt = self.conn.prepare(&data_sql)?;
        let channels: Vec<ChannelSummary> = stmt.query_map(
            rusqlite::params_from_iter(all_params.iter().map(|p| p.as_ref())),
            |row| Ok(ChannelSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                group_title: row.get(2)?,
                logo_url: row.get(3)?,
                hidden: row.get::<_, i32>(4)? != 0,
                is_favorite: row.get::<_, i32>(5)? != 0,
            })
        )?.filter_map(|r| r.ok()).collect();

        Ok(ChannelPage { channels, total, page, page_size })
    }

    pub fn search_channels(&self, query: &str, page: u32, page_size: u32) -> StorageResult<Vec<ChannelSummary>> {
        let offset = page * page_size;
        let fts_query = format!("{}*", query.replace('"', ""));
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, c.group_title, c.logo_url, c.hidden,
                    (CASE WHEN f.id IS NOT NULL THEN 1 ELSE 0 END) as is_fav
             FROM channels_fts fts
             JOIN channels c ON c.id = fts.rowid
             LEFT JOIN favorites f ON f.channel_id = c.id
             WHERE channels_fts MATCH ?1 AND c.hidden = 0
             ORDER BY rank
             LIMIT ?2 OFFSET ?3"
        )?;
        let results = stmt.query_map(params![fts_query, page_size as i64, offset as i64], |row| {
            Ok(ChannelSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                group_title: row.get(2)?,
                logo_url: row.get(3)?,
                hidden: row.get::<_, i32>(4)? != 0,
                is_favorite: row.get::<_, i32>(5)? != 0,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(results)
    }

    pub fn toggle_favorite(&self, channel_id: i64) -> StorageResult<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) FROM favorites WHERE channel_id = ?1",
            params![channel_id],
            |r| r.get::<_, i32>(0)
        ).map(|c| c > 0)?;

        if exists {
            self.conn.execute("DELETE FROM favorites WHERE channel_id = ?1", params![channel_id])?;
            Ok(false)
        } else {
            self.conn.execute("INSERT INTO favorites (channel_id) VALUES (?1)", params![channel_id])?;
            Ok(true)
        }
    }

    pub fn toggle_hidden(&self, channel_id: i64) -> StorageResult<bool> {
        let hidden: bool = self.conn.query_row(
            "SELECT hidden FROM channels WHERE id = ?1",
            params![channel_id],
            |r| r.get::<_, i32>(0)
        ).map(|v| v != 0)?;
        let new_hidden = !hidden;
        self.conn.execute(
            "UPDATE channels SET hidden = ?1 WHERE id = ?2",
            params![new_hidden as i32, channel_id]
        )?;
        Ok(new_hidden)
    }

    pub fn get_stream_url(&self, channel_id: i64) -> StorageResult<String> {
        Ok(self.conn.query_row(
            "SELECT stream_url FROM channels WHERE id = ?1",
            params![channel_id],
            |r| r.get(0)
        )?)
    }

    pub fn get_groups(&self, provider_id: i64) -> StorageResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT group_title FROM channels WHERE provider_id = ?1 AND group_title IS NOT NULL ORDER BY group_title"
        )?;
        let groups = stmt.query_map(params![provider_id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(groups)
    }

    pub fn insert_epg_batch(&self, entries: &[EPGEntry]) -> StorageResult<usize> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO epg (channel_tvg_id, title, start_ts, end_ts, description)
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;
        let mut count = 0;
        for e in entries {
            stmt.execute(params![e.channel_tvg_id, e.title, e.start_ts, e.end_ts, e.description])?;
            count += 1;
        }
        Ok(count)
    }

    pub fn get_epg(&self, tvg_id: &str, from_ts: i64, to_ts: i64) -> StorageResult<Vec<EPGEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, channel_tvg_id, title, start_ts, end_ts, description
             FROM epg WHERE channel_tvg_id = ?1 AND end_ts >= ?2 AND start_ts <= ?3
             ORDER BY start_ts"
        )?;
        let entries = stmt.query_map(params![tvg_id, from_ts, to_ts], |r| {
            Ok(EPGEntry {
                id: r.get(0)?,
                channel_tvg_id: r.get(1)?,
                title: r.get(2)?,
                start_ts: r.get(3)?,
                end_ts: r.get(4)?,
                description: r.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn sample_provider() -> Provider {
        Provider {
            id: None,
            name: "Test Provider".to_string(),
            provider_type: ProviderType::M3U,
            url: "http://example.com/playlist.m3u".to_string(),
            username: None,
            password: None,
        }
    }

    #[test]
    fn insert_and_retrieve_provider() {
        let db = test_db();
        let id = db.insert_provider(&sample_provider()).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn batch_insert_channels_and_paginate() {
        let db = test_db();
        let pid = db.insert_provider(&sample_provider()).unwrap();

        let channels: Vec<Channel> = (0..150).map(|i| Channel {
            id: 0,
            provider_id: pid,
            name: format!("Channel {}", i),
            group_title: Some("Sports".to_string()),
            logo_url: None,
            stream_url: format!("http://example.com/stream/{}", i),
            tvg_id: Some(format!("ch{}", i)),
            hidden: false,
            sort_order: i,
        }).collect();

        let count = db.insert_channels_batch(&channels).unwrap();
        assert_eq!(count, 150);

        let page = db.get_channels_page(0, 50, &FilterOptions::default()).unwrap();
        assert_eq!(page.channels.len(), 50);
        assert_eq!(page.total, 150);
        assert_eq!(page.page, 0);
    }

    #[test]
    fn hidden_channels_excluded_by_default() {
        let db = test_db();
        let pid = db.insert_provider(&sample_provider()).unwrap();

        let channels = vec![
            Channel { id: 0, provider_id: pid, name: "Visible".to_string(), group_title: None, logo_url: None, stream_url: "http://x.com/1".to_string(), tvg_id: None, hidden: false, sort_order: 0 },
            Channel { id: 0, provider_id: pid, name: "Hidden".to_string(), group_title: None, logo_url: None, stream_url: "http://x.com/2".to_string(), tvg_id: None, hidden: true, sort_order: 1 },
        ];
        db.insert_channels_batch(&channels).unwrap();

        let page = db.get_channels_page(0, 50, &FilterOptions::default()).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.channels[0].name, "Visible");
    }

    #[test]
    fn toggle_favorite_adds_and_removes() {
        let db = test_db();
        let pid = db.insert_provider(&sample_provider()).unwrap();
        let channels = vec![Channel { id: 0, provider_id: pid, name: "CNN".to_string(), group_title: None, logo_url: None, stream_url: "http://x.com/1".to_string(), tvg_id: None, hidden: false, sort_order: 0 }];
        db.insert_channels_batch(&channels).unwrap();

        let page = db.get_channels_page(0, 50, &FilterOptions::default()).unwrap();
        let ch_id = page.channels[0].id;

        let is_fav = db.toggle_favorite(ch_id).unwrap();
        assert!(is_fav);
        let is_fav = db.toggle_favorite(ch_id).unwrap();
        assert!(!is_fav);
    }

    #[test]
    fn toggle_hidden_flips_value() {
        let db = test_db();
        let pid = db.insert_provider(&sample_provider()).unwrap();
        let channels = vec![Channel { id: 0, provider_id: pid, name: "BBC".to_string(), group_title: None, logo_url: None, stream_url: "http://x.com/1".to_string(), tvg_id: None, hidden: false, sort_order: 0 }];
        db.insert_channels_batch(&channels).unwrap();
        let page = db.get_channels_page(0, 50, &FilterOptions { show_hidden: true, ..Default::default() }).unwrap();
        let ch_id = page.channels[0].id;

        let now_hidden = db.toggle_hidden(ch_id).unwrap();
        assert!(now_hidden);
        let now_hidden = db.toggle_hidden(ch_id).unwrap();
        assert!(!now_hidden);
    }

    #[test]
    fn fts5_search_returns_matches() {
        let db = test_db();
        let pid = db.insert_provider(&sample_provider()).unwrap();
        let channels: Vec<Channel> = vec![
            ("BBC News", "UK"), ("BBC One", "UK"), ("CNN International", "US"), ("Fox Sports", "US")
        ].into_iter().enumerate().map(|(i, (name, group))| Channel {
            id: 0, provider_id: pid, name: name.to_string(),
            group_title: Some(group.to_string()), logo_url: None,
            stream_url: format!("http://x.com/{}", i), tvg_id: None, hidden: false, sort_order: i as i64,
        }).collect();
        db.insert_channels_batch(&channels).unwrap();
        db.rebuild_channels_fts().unwrap();

        let results = db.search_channels("BBC", 0, 50).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.contains("BBC")));
    }
}
