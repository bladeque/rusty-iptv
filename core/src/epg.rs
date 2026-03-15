use crate::models::EPGEntry;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::BufRead;
use thiserror::Error;
use chrono::NaiveDateTime;

#[derive(Error, Debug)]
pub enum EPGError {
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type EPGResult<T> = std::result::Result<T, EPGError>;

/// Parse XMLTV EPG data from a reader, streaming batches to avoid memory buildup.
pub fn parse_xmltv_streaming<R: BufRead, F>(
    reader: R,
    batch_size: usize,
    mut on_batch: F,
) -> EPGResult<u64>
where
    F: FnMut(Vec<EPGEntry>),
{
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut batch: Vec<EPGEntry> = Vec::with_capacity(batch_size);
    let mut total = 0u64;

    // State for current <programme>
    let mut current_channel: Option<String> = None;
    let mut current_start: Option<i64> = None;
    let mut current_stop: Option<i64> = None;
    let mut current_title: Option<String> = None;
    let mut current_desc: Option<String> = None;
    let mut in_title = false;
    let mut in_desc = false;

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"programme" => {
                        current_channel = None;
                        current_start = None;
                        current_stop = None;
                        current_title = None;
                        current_desc = None;

                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"channel" => {
                                    current_channel = Some(String::from_utf8_lossy(&attr.value).to_string());
                                }
                                b"start" => {
                                    current_start = parse_xmltv_time(&String::from_utf8_lossy(&attr.value));
                                }
                                b"stop" => {
                                    current_stop = parse_xmltv_time(&String::from_utf8_lossy(&attr.value));
                                }
                                _ => {}
                            }
                        }
                    }
                    b"title" => in_title = true,
                    b"desc" => in_desc = true,
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if in_title { current_title = Some(text); }
                else if in_desc { current_desc = Some(text); }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"title" => in_title = false,
                    b"desc" => in_desc = false,
                    b"programme" => {
                        if let (Some(ch), Some(start), Some(stop), Some(title)) =
                            (current_channel.take(), current_start, current_stop, current_title.take())
                        {
                            batch.push(EPGEntry {
                                id: 0,
                                channel_tvg_id: ch,
                                title,
                                start_ts: start,
                                end_ts: stop,
                                description: current_desc.take(),
                            });
                            total += 1;

                            if batch.len() >= batch_size {
                                on_batch(std::mem::replace(&mut batch, Vec::with_capacity(batch_size)));
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EPGError::Xml(e)),
            _ => {}
        }
        buf.clear();
    }

    if !batch.is_empty() {
        on_batch(batch);
    }

    Ok(total)
}

/// Parse XMLTV timestamp format: "20240315143000 +0000" or "20240315143000"
fn parse_xmltv_time(s: &str) -> Option<i64> {
    let s = s.trim();
    // Try with timezone offset
    if s.len() >= 14 {
        let dt_str = &s[..14]; // YYYYMMDDHHmmss
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt_str, "%Y%m%d%H%M%S") {
            let offset_secs = if s.len() > 15 {
                parse_tz_offset(&s[15..])
            } else {
                0
            };
            let ts = ndt.and_utc().timestamp() - offset_secs;
            return Some(ts);
        }
    }
    None
}

fn parse_tz_offset(s: &str) -> i64 {
    let s = s.trim();
    if s.len() < 5 { return 0; }
    let sign = if s.starts_with('-') { -1i64 } else { 1i64 };
    let digits = &s[1..];
    let hours: i64 = digits[..2].parse().unwrap_or(0);
    let mins: i64 = digits[2..4].parse().unwrap_or(0);
    sign * (hours * 3600 + mins * 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_EPG: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<tv>
    <programme start="20240315140000 +0000" stop="20240315150000 +0000" channel="bbc1.uk">
        <title lang="en">BBC News at One</title>
        <desc lang="en">The latest news from the BBC.</desc>
    </programme>
    <programme start="20240315150000 +0000" stop="20240315160000 +0000" channel="bbc1.uk">
        <title lang="en">Afternoon Live</title>
    </programme>
    <programme start="20240315140000 +0000" stop="20240315150000 +0000" channel="cnn.us">
        <title lang="en">CNN Newsroom</title>
    </programme>
</tv>
"#;

    #[test]
    fn parse_basic_xmltv() {
        let mut entries = Vec::new();
        let count = parse_xmltv_streaming(
            SAMPLE_EPG.as_bytes(),
            500,
            |batch| entries.extend(batch),
        ).unwrap();

        assert_eq!(count, 3);
        assert_eq!(entries[0].channel_tvg_id, "bbc1.uk");
        assert_eq!(entries[0].title, "BBC News at One");
        assert!(entries[0].description.is_some());
        assert_eq!(entries[1].title, "Afternoon Live");
        assert!(entries[1].description.is_none());
    }

    #[test]
    fn timestamps_parsed_correctly() {
        let ts = parse_xmltv_time("20240315140000 +0000");
        assert!(ts.is_some());
        let ts2 = parse_xmltv_time("20240315150000 +0000");
        assert!(ts2.unwrap() > ts.unwrap());
    }

    #[test]
    fn batching_works() {
        let header = r#"<?xml version="1.0"?><tv>"#;
        let programmes: String = (0..200).map(|i| format!(
            r#"<programme start="20240315{:02}0000 +0000" stop="20240315{:02}3000 +0000" channel="ch{}.tv"><title>Show {}</title></programme>"#,
            (i / 6) % 24, (i / 6) % 24, i % 5, i
        )).collect();
        let footer = "</tv>";
        let input = format!("{}{}{}", header, programmes, footer);

        let mut batch_count = 0;
        let count = parse_xmltv_streaming(
            input.as_bytes(), 50,
            |batch| { assert!(batch.len() <= 50); batch_count += 1; }
        ).unwrap();
        assert_eq!(count, 200);
        assert_eq!(batch_count, 4);
    }
}
