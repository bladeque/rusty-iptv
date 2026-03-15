#[derive(Debug, Clone, PartialEq)]
pub enum ProviderType {
    XtreamCodes,
    M3U,
}

#[derive(Debug, Clone)]
pub struct Provider {
    pub id: Option<i64>,
    pub name: String,
    pub provider_type: ProviderType,
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub id: i64,
    pub provider_id: i64,
    pub name: String,
    pub group_title: Option<String>,
    pub logo_url: Option<String>,
    pub stream_url: String,
    pub tvg_id: Option<String>,
    pub hidden: bool,
    pub sort_order: i64,
}

#[derive(Debug, Clone)]
pub struct ChannelSummary {
    pub id: i64,
    pub name: String,
    pub group_title: Option<String>,
    pub logo_url: Option<String>,
    pub hidden: bool,
    pub is_favorite: bool,
}

#[derive(Debug, Clone)]
pub struct EPGEntry {
    pub id: i64,
    pub channel_tvg_id: String,
    pub title: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VodItem {
    pub id: i64,
    pub provider_id: i64,
    pub name: String,
    pub cover_url: Option<String>,
    pub stream_url: String,
    pub genre: Option<String>,
    pub rating: Option<String>,
    pub year: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub id: i64,
    pub provider_id: i64,
    pub name: String,
    pub cover_url: Option<String>,
    pub genre: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub id: i64,
    pub series_id: i64,
    pub season: i32,
    pub episode: i32,
    pub name: String,
    pub stream_url: String,
}

#[derive(Debug, Clone)]
pub struct FilterPreset {
    pub id: Option<i64>,
    pub name: String,
    pub group_filter: Option<String>,
    pub name_filter: Option<String>,
    pub show_hidden: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    pub preset_id: Option<i64>,
    pub search_query: Option<String>,
    pub show_hidden: bool,
}

#[derive(Debug, Clone)]
pub struct ChannelPage {
    pub channels: Vec<ChannelSummary>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_summary_has_required_fields() {
        let ch = ChannelSummary {
            id: 1,
            name: "BBC One".to_string(),
            group_title: Some("UK".to_string()),
            logo_url: None,
            hidden: false,
            is_favorite: false,
        };
        assert_eq!(ch.name, "BBC One");
        assert!(!ch.hidden);
    }

    #[test]
    fn filter_options_default_is_no_filter() {
        let opts = FilterOptions::default();
        assert!(opts.preset_id.is_none());
        assert!(opts.search_query.is_none());
        assert!(!opts.show_hidden);
    }
}
