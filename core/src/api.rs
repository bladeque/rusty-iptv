use crate::models::{
    FilterOptions as ModelsFilterOptions,
    Provider, ProviderType,
    ChannelSummary as ModelChannelSummary,
    ChannelPage as ModelChannelPage,
    EPGEntry as ModelEPGEntry,
};
use crate::storage::Database;
use std::sync::Mutex;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Storage: {0}")]
    Storage(String),
    #[error("Parse: {0}")]
    Parse(String),
    #[error("Network: {0}")]
    Network(String),
    #[error("Not found")]
    NotFound,
}

impl From<crate::storage::StorageError> for CoreError {
    fn from(e: crate::storage::StorageError) -> Self {
        CoreError::Storage(e.to_string())
    }
}

pub struct ProviderInput {
    pub name: String,
    pub provider_type: String,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub struct FilterOptions {
    pub preset_id: Option<i64>,
    pub search_query: Option<String>,
    pub show_hidden: bool,
}

impl From<FilterOptions> for ModelsFilterOptions {
    fn from(f: FilterOptions) -> Self {
        ModelsFilterOptions {
            preset_id: f.preset_id,
            search_query: f.search_query,
            show_hidden: f.show_hidden,
        }
    }
}

// Re-export model types needed by callers
pub use crate::models::ChannelSummary;
pub use crate::models::ChannelPage;
pub use crate::models::EPGEntry;
pub use crate::models::VodItem;

pub struct RustyCore {
    db: Mutex<Database>,
}

impl RustyCore {
    pub fn new(db_path: String) -> Result<Self, CoreError> {
        let db = Database::open(Path::new(&db_path))
            .map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(RustyCore { db: Mutex::new(db) })
    }

    pub fn add_provider(&self, provider: ProviderInput) -> Result<i64, CoreError> {
        let p = Provider {
            id: None,
            name: provider.name,
            provider_type: if provider.provider_type == "xtream" {
                ProviderType::XtreamCodes
            } else {
                ProviderType::M3U
            },
            url: provider.url,
            username: provider.username,
            password: provider.password,
        };
        let db = self.db.lock().unwrap();
        Ok(db.insert_provider(&p)?)
    }

    pub fn import_m3u_from_url(&self, provider_id: i64, url: String) -> Result<(), CoreError> {
        let body = reqwest::blocking::get(&url)
            .map_err(|e| CoreError::Network(e.to_string()))?
            .bytes()
            .map_err(|e| CoreError::Network(e.to_string()))?;

        let db = self.db.lock().unwrap();
        crate::m3u::parse_m3u_streaming(body.as_ref(), provider_id, 500, |batch| {
            let _ = db.insert_channels_batch(&batch);
        }).map_err(|e| CoreError::Parse(e.to_string()))?;

        db.rebuild_channels_fts().map_err(|e| CoreError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn get_channels(&self, page: u32, opts: FilterOptions) -> Result<ModelChannelPage, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.get_channels_page(page, 50, &opts.into())?)
    }

    pub fn search_channels(&self, query: String, page: u32) -> Result<Vec<ModelChannelSummary>, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.search_channels(&query, page, 50)?)
    }

    pub fn get_stream_url(&self, channel_id: i64) -> Result<String, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.get_stream_url(channel_id)?)
    }

    pub fn toggle_favorite(&self, channel_id: i64) -> Result<bool, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.toggle_favorite(channel_id)?)
    }

    pub fn toggle_hidden(&self, channel_id: i64) -> Result<bool, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.toggle_hidden(channel_id)?)
    }

    pub fn get_epg(&self, tvg_id: String, from_ts: i64, to_ts: i64) -> Result<Vec<ModelEPGEntry>, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.get_epg(&tvg_id, from_ts, to_ts)?)
    }

    pub fn get_groups(&self, provider_id: i64) -> Result<Vec<String>, CoreError> {
        let db = self.db.lock().unwrap();
        Ok(db.get_groups(provider_id)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_core() -> (RustyCore, NamedTempFile) {
        let f = NamedTempFile::new().unwrap();
        let core = RustyCore::new(f.path().to_str().unwrap().to_string()).unwrap();
        (core, f)
    }

    #[test]
    fn add_provider_returns_id() {
        let (core, _f) = make_core();
        let id = core.add_provider(ProviderInput {
            name: "Test".to_string(),
            provider_type: "m3u".to_string(),
            url: "http://example.com/list.m3u".to_string(),
            username: None,
            password: None,
        }).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn get_channels_empty_returns_page() {
        let (core, _f) = make_core();
        // Need a provider first to have a valid DB state
        core.add_provider(ProviderInput {
            name: "P".to_string(),
            provider_type: "m3u".to_string(),
            url: "http://x.com".to_string(),
            username: None,
            password: None,
        }).unwrap();

        let page = core.get_channels(0, FilterOptions {
            preset_id: None,
            search_query: None,
            show_hidden: false,
        }).unwrap();
        assert_eq!(page.total, 0);
        assert_eq!(page.channels.len(), 0);
    }

    #[test]
    fn get_groups_returns_distinct_groups() {
        use crate::models::Channel;
        let f = NamedTempFile::new().unwrap();
        let core = RustyCore::new(f.path().to_str().unwrap().to_string()).unwrap();

        let provider_id = core.add_provider(ProviderInput {
            name: "P".to_string(),
            provider_type: "m3u".to_string(),
            url: "http://x.com".to_string(),
            username: None,
            password: None,
        }).unwrap();

        // Insert channels directly via db
        let db = core.db.lock().unwrap();
        let channels = vec![
            Channel { id: 0, provider_id, name: "Ch1".to_string(), group_title: Some("Sports".to_string()), logo_url: None, stream_url: "http://x.com/1".to_string(), tvg_id: None, hidden: false, sort_order: 0 },
            Channel { id: 0, provider_id, name: "Ch2".to_string(), group_title: Some("News".to_string()), logo_url: None, stream_url: "http://x.com/2".to_string(), tvg_id: None, hidden: false, sort_order: 1 },
            Channel { id: 0, provider_id, name: "Ch3".to_string(), group_title: Some("Sports".to_string()), logo_url: None, stream_url: "http://x.com/3".to_string(), tvg_id: None, hidden: false, sort_order: 2 },
        ];
        db.insert_channels_batch(&channels).unwrap();
        drop(db);

        let groups = core.get_groups(provider_id).unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.contains(&"Sports".to_string()));
        assert!(groups.contains(&"News".to_string()));
    }
}
