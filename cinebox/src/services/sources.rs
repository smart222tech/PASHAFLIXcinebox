//! Public-page source adapters. This first slice deliberately parses catalog
//! metadata only; it does not bypass site access controls or extract protected media.

use std::collections::BTreeSet;

/// A supported website family. Hosts are matched exactly or as subdomains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SourceSite {
    Zona,
    KinoBesplatno,
    Kinogo,
    LordFilm,
    Seasonvar,
    Kinokrad,
}

impl SourceSite {
    pub(crate) const ALL: [Self; 6] = [
        Self::Zona,
        Self::KinoBesplatno,
        Self::Kinogo,
        Self::LordFilm,
        Self::Seasonvar,
        Self::Kinokrad,
    ];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Zona => "ZONA",
            Self::KinoBesplatno => "KinoBesplatno",
            Self::Kinogo => "Kinogo",
            Self::LordFilm => "LordFilm",
            Self::Seasonvar => "Seasonvar",
            Self::Kinokrad => "Kinokrad",
        }
    }

    fn hosts(self) -> &'static [&'static str] {
        match self {
            Self::Zona => &["zona.plus", "zona.mobi"],
            Self::KinoBesplatno => &["kinobesplatno.online"],
            Self::Kinogo => &["kinogo.la"],
            Self::LordFilm => &["lordfilm.org"],
            Self::Seasonvar => &["seasonvar.ru"],
            Self::Kinokrad => &["kinokrad.cc"],
        }
    }

    pub(crate) fn from_url(url: &str) -> Option<Self> {
        let authority = url.split_once("://")?.1.split('/').next()?.split(':').next()?;
        let host = authority.trim_start_matches("www.").to_ascii_lowercase();
        Self::ALL.into_iter().find(|site| {
            site.hosts().iter().any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        })
    }
}

/// What kind of Zona catalog page was supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ZonaPageKind {
    Movie,
    Series,
}

impl ZonaPageKind {
    pub(crate) fn from_url(url: &str) -> Option<Self> {
        let path = url.split_once("://")?.1.split_once('/')?.1;
        if path.starts_with("movies/") {
            Some(Self::Movie)
        } else if path.starts_with("tvseries/") {
            Some(Self::Series)
        } else {
            None
        }
    }
}

/// Metadata extracted from the publicly visible page markup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZonaMetadata {
    pub kind: ZonaPageKind,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<u16>,
    pub description: Option<String>,
    pub poster_url: Option<String>,
    pub season_numbers: Vec<u16>,
    pub has_embedded_player: bool,
}

pub(crate) fn parse_zona_page(url: &str, html: &str) -> Option<ZonaMetadata> {
    let kind = ZonaPageKind::from_url(url)?;
    let metas = meta_tags(html);
    let heading = first_element_text(html, "h1");
    let title = heading
        .or_else(|| metas.get("og:title").cloned())
        .or_else(|| first_element_text(html, "title"))?;
    let title = clean_text(&title);
    if title.is_empty() {
        return None;
    }

    let original_title = metas.get("og:site_name").cloned();
    let description = metas.get("og:description").cloned()
        .or_else(|| metas.get("description").cloned())
        .map(|value| clean_text(&value))
        .filter(|value| !value.is_empty());
    let poster_url = metas.get("og:image").cloned();
    let year = find_year(html);
    let season_numbers = if kind == ZonaPageKind::Series {
        find_seasons(html)
    } else {
        Vec::new()
    };

    Some(ZonaMetadata {
        kind,
        title,
        original_title,
        year,
        description,
        poster_url,
        season_numbers,
        has_embedded_player: html.to_ascii_lowercase().contains("<iframe")
            || html.to_ascii_lowercase().contains("<video")
            || html.to_ascii_lowercase().contains("player"),
    })
}

fn meta_tags(html: &str) -> std::collections::HashMap<String, String> {
    let mut values = std::collections::HashMap::new();
    let lower = html.to_ascii_lowercase();
    let mut offset = 0;
    while let Some(start) = lower[offset..].find("<meta") {
        let start = offset + start;
        let Some(end_rel) = lower[start..].find('>') else { break };
        let end = start + end_rel;
        let tag = &html[start..=end];
        let attrs = attributes(tag);
        let key = attrs.get("property").or_else(|| attrs.get("name"));
        let value = attrs.get("content");
        if let (Some(key), Some(value)) = (key, value) {
            values.insert(key.to_ascii_lowercase(), clean_text(value));
        }
        offset = end + 1;
    }
    values
}

fn attributes(tag: &str) -> std::collections::HashMap<String, String> {
    let bytes = tag.as_bytes();
    let mut attrs = std::collections::HashMap::new();
    let mut i = 5.min(bytes.len());
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/' || bytes[i] == b'>') { i += 1; }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'-' | b':' | b'_')) { i += 1; }
        if start == i { i += 1; continue; }
        let key = tag[start..i].to_ascii_lowercase();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
        if i >= bytes.len() || bytes[i] != b'=' { continue; }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() { i += 1; }
        if i >= bytes.len() { break; }
        let (value_start, value_end) = if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let value_start = i;
            while i < bytes.len() && bytes[i] != quote { i += 1; }
            let value_end = i;
            if i < bytes.len() { i += 1; }
            (value_start, value_end)
        } else {
            let value_start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'>' { i += 1; }
            (value_start, i)
        };
        attrs.insert(key, tag[value_start..value_end].to_owned());
    }
    attrs
}

fn first_element_text(html: &str, element: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = format!("<{element}");
    let start = lower.find(&open)?;
    let content_start = start + lower[start..].find('>')? + 1;
    let close = format!("</{element}");
    let end = content_start + lower[content_start..].find(&close)?;
    Some(clean_text(&html[content_start..end]))
}

fn clean_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_year(html: &str) -> Option<u16> {
    let bytes = html.as_bytes();
    bytes.windows(4).find_map(|window| {
        if window.iter().all(u8::is_ascii_digit) {
            let year = std::str::from_utf8(window).ok()?.parse::<u16>().ok()?;
            (1900..=2099).contains(&year).then_some(year)
        } else {
            None
        }
    })
}

fn find_seasons(html: &str) -> Vec<u16> {
    let lower = html.to_ascii_lowercase();
    let mut seasons = BTreeSet::new();
    let mut offset = 0;
    while let Some(found) = lower[offset..].find("data-season") {
        let start = offset + found;
        let tail = &html[start..html.len().min(start + 80)];
        if let Some((_, value)) = tail.split_once('=') {
            let digits: String = value.trim_start_matches([' ', '"', '\''])
                .chars().take_while(char::is_ascii_digit).collect();
            if let Ok(number) = digits.parse::<u16>() {
                if number > 0 { seasons.insert(number); }
            }
        }
        offset = start + "data-season".len();
    }
    seasons.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_the_six_requested_site_families() {
        assert_eq!(SourceSite::from_url("https://w140.zona.plus/movies/x"), Some(SourceSite::Zona));
        assert_eq!(SourceSite::from_url("https://www.kinogo.la/film"), Some(SourceSite::Kinogo));
        assert_eq!(SourceSite::from_url("https://evilzona.plus.example/"), None);
        assert_eq!(SourceSite::from_url("https://example.com/"), None);
    }

    #[test]
    fn parses_zona_movie_metadata_from_public_page_markup() {
        let html = r#"<html><head><meta property="og:image" content="https://img.example/poster.jpg"><meta name="description" content="A&amp;B"></head><body><h1>Одиссея</h1><div>2026</div></body></html>"#;
        let parsed = parse_zona_page("https://w140.zona.plus/movies/odisseya-2026", html).expect("fixture parses");
        assert_eq!(parsed.kind, ZonaPageKind::Movie);
        assert_eq!(parsed.title, "Одиссея");
        assert_eq!(parsed.year, Some(2026));
        assert_eq!(parsed.description.as_deref(), Some("A&B"));
        assert_eq!(parsed.poster_url.as_deref(), Some("https://img.example/poster.jpg"));
        assert!(parsed.season_numbers.is_empty());
    }

    #[test]
    fn parses_zona_series_seasons_and_player_presence() {
        let html = r#"<h1>Укрытие</h1><span>2023</span><div data-season="1"></div><div data-season='3'></div><iframe src="player"></iframe>"#;
        let parsed = parse_zona_page("https://w140.zona.plus/tvseries/ukrytie-2023", html).expect("fixture parses");
        assert_eq!(parsed.kind, ZonaPageKind::Series);
        assert_eq!(parsed.year, Some(2023));
        assert_eq!(parsed.season_numbers, vec![1, 3]);
        assert!(parsed.has_embedded_player);
    }

    #[test]
    fn rejects_unknown_zona_routes_and_missing_titles() {
        assert!(parse_zona_page("https://w140.zona.plus/", "<h1>Home</h1>").is_none());
        assert!(parse_zona_page("https://w140.zona.plus/movies/x", "<p>no title</p>").is_none());
    }
}
