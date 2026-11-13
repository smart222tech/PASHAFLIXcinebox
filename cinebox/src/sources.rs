//! External catalogue source registry for PashaFlix.
//!
//! This module deliberately separates a provider's public catalogue entry point
//! from playback resolution. A public page URL is not assumed to be a direct
//! media URL, and no provider is marked as externally playable until a source
//! adapter has been implemented and its playback flow has been verified.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SourceId {
    Zona,
    KinoBesplatno,
    Kinogo,
    LordFilm,
    Seasonvar,
    Kinokrad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CatalogKind {
    MoviesAndSeries,
    Movies,
    Series,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegrationStatus {
    /// Source URL and catalogue structure have been researched.
    ResearchComplete,
    /// The source adapter still needs implementation and fixture tests.
    AdapterPending,
    /// A working adapter has passed its catalogue tests.
    CatalogReady,
    /// External playback is separately verified and allowed.
    ExternalPlaybackVerified,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceDescriptor {
    pub id: SourceId,
    pub name: &'static str,
    pub homepage: &'static str,
    pub movies_url: Option<&'static str>,
    pub series_url: Option<&'static str>,
    pub catalog_kind: CatalogKind,
    pub status: IntegrationStatus,
}

pub(crate) const SOURCES: [SourceDescriptor; 6] = [
    SourceDescriptor {
        id: SourceId::Zona,
        name: "ZONA",
        homepage: "https://w140.zona.plus/",
        movies_url: Some("https://w140.zona.plus/movies"),
        series_url: Some("https://w140.zona.plus/tvseries"),
        catalog_kind: CatalogKind::MoviesAndSeries,
        status: IntegrationStatus::ResearchComplete,
    },
    SourceDescriptor {
        id: SourceId::KinoBesplatno,
        name: "KinoBesplatno",
        homepage: "https://kinobesplatno.online/",
        movies_url: Some("https://kinobesplatno.online/filmy/"),
        series_url: Some("https://kinobesplatno.online/serialy/"),
        catalog_kind: CatalogKind::MoviesAndSeries,
        status: IntegrationStatus::ResearchComplete,
    },
    SourceDescriptor {
        id: SourceId::Kinogo,
        name: "Kinogo",
        homepage: "https://kinogo.la/",
        movies_url: Some("https://kinogo.la/"),
        series_url: Some("https://kinogo.la/"),
        catalog_kind: CatalogKind::MoviesAndSeries,
        status: IntegrationStatus::AdapterPending,
    },
    SourceDescriptor {
        id: SourceId::LordFilm,
        name: "LordFilm",
        homepage: "https://lordfilm.org/",
        movies_url: Some("https://lordfilm.org/"),
        series_url: Some("https://lordfilm.org/"),
        catalog_kind: CatalogKind::MoviesAndSeries,
        status: IntegrationStatus::AdapterPending,
    },
    SourceDescriptor {
        id: SourceId::Seasonvar,
        name: "Seasonvar",
        homepage: "https://seasonvar.ru/",
        movies_url: None,
        series_url: Some("https://seasonvar.ru/"),
        catalog_kind: CatalogKind::Series,
        status: IntegrationStatus::AdapterPending,
    },
    SourceDescriptor {
        id: SourceId::Kinokrad,
        name: "Kinokrad",
        homepage: "https://kinokrad.cc/",
        movies_url: Some("https://kinokrad.cc/"),
        series_url: Some("https://kinokrad.cc/"),
        catalog_kind: CatalogKind::MoviesAndSeries,
        status: IntegrationStatus::AdapterPending,
    },
];

#[must_use]
pub(crate) fn all_sources() -> &'static [SourceDescriptor] {
    &SOURCES
}

#[must_use]
pub(crate) fn source(id: SourceId) -> &'static SourceDescriptor {
    // The registry is a fixed compile-time list, so this lookup cannot fail.
    SOURCES.iter().find(|item| item.id == id).unwrap_or(&SOURCES[0])
}

/// Identify a known provider from a URL's host without trusting the URL path.
#[must_use]
pub(crate) fn source_for_url(url: &str) -> Option<SourceId> {
    let authority = url
        .split_once("://")?
        .1
        .split(['/', '?', '#'])
        .next()?
        .split('@')
        .next_back()?;
    let host = authority.split(':').next()?.trim_end_matches('.').to_ascii_lowercase();

    match host.as_str() {
        "zona.plus" | "www.zona.plus" | "w140.zona.plus" => Some(SourceId::Zona),
        "kinobesplatno.online" | "www.kinobesplatno.online" => Some(SourceId::KinoBesplatno),
        "kinogo.la" | "www.kinogo.la" => Some(SourceId::Kinogo),
        "lordfilm.org" | "www.lordfilm.org" => Some(SourceId::LordFilm),
        "seasonvar.ru" | "www.seasonvar.ru" => Some(SourceId::Seasonvar),
        "kinokrad.cc" | "www.kinokrad.cc" => Some(SourceId::Kinokrad),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_all_six_requested_sources() {
        assert_eq!(all_sources().len(), 6);
        assert_eq!(all_sources()[0].id, SourceId::Zona);
        assert_eq!(all_sources()[1].id, SourceId::KinoBesplatno);
    }

    #[test]
    fn recognizes_provider_hosts_and_subdomains_explicitly() {
        assert_eq!(source_for_url("https://w140.zona.plus/movies/odisseya-2026"), Some(SourceId::Zona));
        assert_eq!(source_for_url("https://kinobesplatno.online/serialy/example"), Some(SourceId::KinoBesplatno));
        assert_eq!(source_for_url("https://seasonvar.ru/serial/example"), Some(SourceId::Seasonvar));
    }

    #[test]
    fn does_not_match_lookalike_or_unknown_hosts() {
        assert_eq!(source_for_url("https://zona.plus.attacker.example/"), None);
        assert_eq!(source_for_url("https://example.com/?next=zona.plus"), None);
        assert_eq!(source_for_url("not a url"), None);
    }

    #[test]
    fn external_playback_is_not_claimed_before_verification() {
        assert!(all_sources().iter().all(|source| {
            source.status != IntegrationStatus::ExternalPlaybackVerified
        }));
    }
}
