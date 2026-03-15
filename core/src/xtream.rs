#[allow(unused_imports)]
use crate::models::{Channel, VodItem, Series, Episode, EPGEntry};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum XtreamError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

pub type XtreamResult<T> = std::result::Result<T, XtreamError>;

#[derive(Clone)]
pub struct XtreamClient {
    client: Client,
    base_url: String,
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct XtreamUserInfo {
    auth: i32,
}

#[derive(Deserialize)]
struct XtreamAuthResponse {
    user_info: XtreamUserInfo,
}

#[derive(Deserialize)]
pub struct XtreamCategory {
    pub category_id: String,
    pub category_name: String,
}

#[derive(Deserialize)]
struct XtreamStream {
    stream_id: Option<serde_json::Value>,
    name: Option<String>,
    stream_icon: Option<String>,
    epg_channel_id: Option<String>,
    category_name: Option<String>,
}

#[derive(Deserialize)]
struct XtreamVod {
    stream_id: Option<serde_json::Value>,
    name: Option<String>,
    stream_icon: Option<String>,
    category_name: Option<String>,
    rating: Option<String>,
    year: Option<String>,
}

#[derive(Deserialize)]
struct XtreamSeries {
    series_id: Option<serde_json::Value>,
    name: Option<String>,
    cover: Option<String>,
    category_name: Option<String>,
}

impl XtreamClient {
    pub fn new(base_url: String, username: String, password: String) -> Self {
        XtreamClient {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            username,
            password,
        }
    }

    fn api_url(&self, action: &str) -> String {
        format!(
            "{}/player_api.php?username={}&password={}&action={}",
            self.base_url, self.username, self.password, action
        )
    }

    pub async fn authenticate(&self) -> XtreamResult<()> {
        let url = format!(
            "{}/player_api.php?username={}&password={}",
            self.base_url, self.username, self.password
        );
        let resp: XtreamAuthResponse = self.client.get(&url).send().await?.json().await?;
        if resp.user_info.auth != 1 {
            return Err(XtreamError::AuthFailed);
        }
        Ok(())
    }

    pub async fn get_live_categories(&self) -> XtreamResult<Vec<XtreamCategory>> {
        let url = self.api_url("get_live_categories");
        Ok(self.client.get(&url).send().await?.json().await?)
    }

    /// Fetch live streams for a category, call on_batch for each batch of channels
    pub async fn get_live_streams_batched<F>(
        &self,
        provider_id: i64,
        category_id: Option<&str>,
        batch_size: usize,
        mut on_batch: F,
    ) -> XtreamResult<u64>
    where
        F: FnMut(Vec<Channel>),
    {
        let action = if let Some(cat) = category_id {
            format!("get_live_streams&category_id={}", cat)
        } else {
            "get_live_streams".to_string()
        };
        let url = self.api_url(&action);
        let streams: Vec<XtreamStream> = self.client.get(&url).send().await?.json().await?;

        let mut total = 0u64;
        let mut batch = Vec::with_capacity(batch_size);

        for (sort_order, s) in streams.into_iter().enumerate() {
            let stream_id = match &s.stream_id {
                Some(v) => v.to_string().trim_matches('"').to_string(),
                None => continue,
            };
            let stream_url = format!(
                "{}/{}/{}/{}.ts",
                self.base_url, self.username, self.password, stream_id
            );
            batch.push(Channel {
                id: 0,
                provider_id,
                name: s.name.unwrap_or_default(),
                group_title: s.category_name,
                logo_url: s.stream_icon.filter(|s| !s.is_empty()),
                stream_url,
                tvg_id: s.epg_channel_id.filter(|s| !s.is_empty()),
                hidden: false,
                sort_order: sort_order as i64,
            });
            total += 1;

            if batch.len() >= batch_size {
                on_batch(std::mem::replace(&mut batch, Vec::with_capacity(batch_size)));
            }
        }
        if !batch.is_empty() {
            on_batch(batch);
        }
        Ok(total)
    }

    pub async fn get_vod_batched<F>(
        &self,
        provider_id: i64,
        batch_size: usize,
        mut on_batch: F,
    ) -> XtreamResult<u64>
    where
        F: FnMut(Vec<VodItem>),
    {
        let url = self.api_url("get_vod_streams");
        let items: Vec<XtreamVod> = self.client.get(&url).send().await?.json().await?;
        let mut total = 0u64;
        let mut batch = Vec::with_capacity(batch_size);

        for v in items {
            let stream_id = match &v.stream_id {
                Some(id) => id.to_string().trim_matches('"').to_string(),
                None => continue,
            };
            let stream_url = format!(
                "{}/movie/{}/{}/{}.mp4",
                self.base_url, self.username, self.password, stream_id
            );
            batch.push(VodItem {
                id: 0,
                provider_id,
                name: v.name.unwrap_or_default(),
                cover_url: v.stream_icon.filter(|s| !s.is_empty()),
                stream_url,
                genre: v.category_name,
                rating: v.rating,
                year: v.year,
            });
            total += 1;
            if batch.len() >= batch_size {
                on_batch(std::mem::replace(&mut batch, Vec::with_capacity(batch_size)));
            }
        }
        if !batch.is_empty() {
            on_batch(batch);
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_url_format() {
        let client = XtreamClient::new(
            "http://example.com:8080".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        let url = client.api_url("get_live_categories");
        assert!(url.contains("player_api.php"));
        assert!(url.contains("username=user"));
        assert!(url.contains("password=pass"));
        assert!(url.contains("action=get_live_categories"));
    }

    #[test]
    fn stream_url_construction() {
        let client = XtreamClient::new(
            "http://example.com".to_string(),
            "user".to_string(),
            "pass".to_string(),
        );
        // Verify URL format for streams
        let url = format!("{}/{}/{}/12345.ts", client.base_url, client.username, client.password);
        assert_eq!(url, "http://example.com/user/pass/12345.ts");
    }
}
