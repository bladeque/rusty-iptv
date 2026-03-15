use crate::models::Channel;
use std::io::{BufRead, BufReader, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum M3UError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not a valid M3U file (missing #EXTM3U header)")]
    InvalidFormat,
}

pub type M3UResult<T> = std::result::Result<T, M3UError>;

/// Parse M3U from a reader, yielding Channel records in batches.
/// `provider_id` is the DB provider ID.
/// `on_batch` is called with each batch of parsed channels (batch_size items).
/// Never allocates more than batch_size channels at once.
pub fn parse_m3u_streaming<R: Read, F>(
    reader: R,
    provider_id: i64,
    batch_size: usize,
    mut on_batch: F,
) -> M3UResult<u64>
where
    F: FnMut(Vec<Channel>),
{
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    // Validate header
    let first = lines.next()
        .ok_or(M3UError::InvalidFormat)??;
    if !first.trim_start().starts_with("#EXTM3U") {
        return Err(M3UError::InvalidFormat);
    }

    let mut batch: Vec<Channel> = Vec::with_capacity(batch_size);
    let mut pending_meta: Option<ChannelMeta> = None;
    let mut total = 0u64;
    let mut sort_order = 0i64;

    for line in lines {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with("#EXTINF:") {
            pending_meta = Some(parse_extinf(line));
        } else if !line.starts_with('#') {
            // This is a stream URL
            let meta = pending_meta.take().unwrap_or_default();
            batch.push(Channel {
                id: 0,
                provider_id,
                name: meta.name,
                group_title: meta.group_title,
                logo_url: meta.logo_url,
                stream_url: line.to_string(),
                tvg_id: meta.tvg_id,
                hidden: false,
                sort_order,
            });
            sort_order += 1;
            total += 1;

            if batch.len() >= batch_size {
                on_batch(std::mem::replace(&mut batch, Vec::with_capacity(batch_size)));
            }
        }
    }

    if !batch.is_empty() {
        on_batch(batch);
    }

    Ok(total)
}

#[derive(Default)]
struct ChannelMeta {
    name: String,
    group_title: Option<String>,
    logo_url: Option<String>,
    tvg_id: Option<String>,
}

fn parse_extinf(line: &str) -> ChannelMeta {
    let mut meta = ChannelMeta::default();

    // Extract display name (after last comma)
    if let Some(comma_pos) = line.rfind(',') {
        meta.name = line[comma_pos + 1..].trim().to_string();
    }

    // Extract attributes: key="value" pairs
    if let Some(attrs_part) = line.strip_prefix("#EXTINF:") {
        let attrs_part = attrs_part.split(',').next().unwrap_or("");
        // Skip the duration number
        let attrs_part = attrs_part.trim_start_matches(|c: char| c.is_numeric() || c == '-');

        meta.tvg_id = extract_attr(attrs_part, "tvg-id");
        meta.logo_url = extract_attr(attrs_part, "tvg-logo");
        meta.group_title = extract_attr(attrs_part, "group-title");

        if meta.name.is_empty() {
            meta.name = extract_attr(attrs_part, "tvg-name").unwrap_or_default();
        }
    }

    if meta.name.is_empty() {
        meta.name = "Unknown Channel".to_string();
    }

    meta
}

fn extract_attr(s: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=\"", key);
    let start = s.find(&pattern)? + pattern.len();
    let end = s[start..].find('"')? + start;
    let value = s[start..end].trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_M3U: &str = r#"#EXTM3U
#EXTINF:-1 tvg-id="bbc1.uk" tvg-logo="http://logos.com/bbc1.png" group-title="UK",BBC One
http://stream.example.com/bbc1
#EXTINF:-1 tvg-id="bbc2.uk" group-title="UK",BBC Two
http://stream.example.com/bbc2
#EXTINF:-1 group-title="US",CNN
http://stream.example.com/cnn
"#;

    #[test]
    fn parse_basic_m3u() {
        let mut channels = Vec::new();
        let count = parse_m3u_streaming(
            SAMPLE_M3U.as_bytes(), 1, 500,
            |batch| channels.extend(batch)
        ).unwrap();

        assert_eq!(count, 3);
        assert_eq!(channels.len(), 3);
        assert_eq!(channels[0].name, "BBC One");
        assert_eq!(channels[0].tvg_id.as_deref(), Some("bbc1.uk"));
        assert_eq!(channels[0].group_title.as_deref(), Some("UK"));
        assert!(channels[0].logo_url.is_some());
        assert_eq!(channels[2].name, "CNN");
        assert_eq!(channels[2].group_title.as_deref(), Some("US"));
    }

    #[test]
    fn batching_works_for_large_input() {
        // Generate 1000 channels
        let header = "#EXTM3U\n";
        let entries: String = (0..1000).map(|i| {
            format!("#EXTINF:-1 group-title=\"Group{}\",Channel {}\nhttp://x.com/{}\n", i % 10, i, i)
        }).collect();
        let input = format!("{}{}", header, entries);

        let mut batch_count = 0;
        let mut total_channels = 0u64;

        let count = parse_m3u_streaming(
            input.as_bytes(), 1, 100,
            |batch| {
                assert!(batch.len() <= 100, "batch too large: {}", batch.len());
                total_channels += batch.len() as u64;
                batch_count += 1;
            }
        ).unwrap();

        assert_eq!(count, 1000);
        assert_eq!(total_channels, 1000);
        assert_eq!(batch_count, 10); // 1000 / 100
    }

    #[test]
    fn invalid_m3u_returns_error() {
        let result = parse_m3u_streaming(b"not a m3u file".as_ref(), 1, 100, |_| {});
        assert!(matches!(result, Err(M3UError::InvalidFormat)));
    }

    #[test]
    fn sort_order_is_sequential() {
        let mut channels = Vec::new();
        parse_m3u_streaming(SAMPLE_M3U.as_bytes(), 1, 500, |b| channels.extend(b)).unwrap();
        for (i, ch) in channels.iter().enumerate() {
            assert_eq!(ch.sort_order, i as i64);
        }
    }
}
