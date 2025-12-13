use crate::models::{
    AuditSummary, PlaylistSummary, ProblematicTrack, SyncBatchLog, SyncReport, TrackInspection,
};
use futures::stream::TryStreamExt;
use log::{debug, info};
use rspotify::{
    model::{FullTrack, Market, PlaylistId, TrackId},
    prelude::*,
    AuthCodeSpotify,
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuditError {
    #[error("Spotify API error: {0}")]
    Spotify(#[from] rspotify::ClientError),
    #[error("Invalid Playlist ID: {0}")]
    InvalidId(String),
    #[error("Invalid Track ID: {0}")]
    InvalidTrackId(String),
}

pub struct Auditor {
    spotify: Arc<AuthCodeSpotify>,
}

impl Auditor {
    pub fn new(spotify: AuthCodeSpotify) -> Self {
        Self {
            spotify: Arc::new(spotify),
        }
    }

    /// Scans the user's "Liked Songs" (Saved Tracks) for unplayable items.
    pub async fn scan_liked_songs(&self) -> Result<AuditSummary, AuditError> {
        let mut summary = AuditSummary::new();
        let mut stream = self.spotify.current_user_saved_tracks(None);

        while let Some(item) = stream.try_next().await? {
            summary.total_tracks_scanned += 1;
            if let Some(problem) = self.analyze_track(&item.track) {
                summary.add_problem(problem);
            }
        }

        Ok(summary)
    }

    /// Scans a specific Playlist for unplayable items.
    pub async fn scan_playlist(&self, playlist_id_str: &str) -> Result<AuditSummary, AuditError> {
        let mut summary = AuditSummary::new();

        let playlist_id = PlaylistId::from_id(playlist_id_str)
            .map_err(|_| AuditError::InvalidId(playlist_id_str.to_string()))?;

        let mut stream = self
            .spotify
            .playlist_items(playlist_id, None, Some(Market::FromToken));

        while let Some(item) = stream.try_next().await? {
            if let Some(rspotify::model::PlayableItem::Track(track)) = item.track {
                summary.total_tracks_scanned += 1;
                if let Some(problem) = self.analyze_track(&track) {
                    summary.add_problem(problem);
                }
            }
        }

        Ok(summary)
    }

    pub async fn inspect_track(&self, track_id_str: &str) -> Result<TrackInspection, AuditError> {
        let track_id = TrackId::from_id(track_id_str)
            .map_err(|_| AuditError::InvalidTrackId(track_id_str.to_string()))?;

        let track = self.spotify.track(track_id, None).await?;

        Ok(TrackInspection {
            id: track.id.map(|id| id.to_string()).unwrap_or_default(),
            name: track.name,
            artists: track.artists.iter().map(|a| a.name.clone()).collect(),
            album: track.album.name,
            release_date: track.album.release_date.unwrap_or_default(),
            duration_ms: track.duration.num_milliseconds() as u32,
            popularity: track.popularity,
            is_playable: track.is_playable,
            available_markets: track
                .available_markets
                .iter()
                .map(|m| m.as_str().to_string())
                .collect(),
            external_ids: track.external_ids,
            external_urls: track.external_urls,
            disc_number: track.disc_number,
            track_number: track.track_number,
            is_local: track.is_local,
        })
    }

    pub async fn list_playlists(&self) -> Result<Vec<PlaylistSummary>, AuditError> {
        let mut playlists = Vec::new();
        let mut stream = self.spotify.current_user_playlists();

        while let Some(pl) = stream.try_next().await? {
            let owner_name = pl.owner.display_name.unwrap_or(pl.owner.id.to_string());

            playlists.push(PlaylistSummary {
                id: pl.id.to_string(),
                name: pl.name,
                total_tracks: pl.tracks.total,
                is_public: pl.public.unwrap_or(false),
                is_collaborative: pl.collaborative,
                owner_name,
            });
        }

        Ok(playlists)
    }

    async fn get_liked_songs_count(&self) -> Result<u32, AuditError> {
        let page = self
            .spotify
            .current_user_saved_tracks_manual(None, Some(1), Some(0))
            .await?;
        Ok(page.total)
    }

    pub async fn sync_playlist_to_liked(
        &self,
        playlist_id_str: &str,
    ) -> Result<SyncReport, AuditError> {
        let initial_liked_count = self.get_liked_songs_count().await?;

        let mut report = SyncReport {
            initial_liked_count,
            ..Default::default()
        };

        let playlist_id = PlaylistId::from_id(playlist_id_str)
            .map_err(|_| AuditError::InvalidId(playlist_id_str.to_string()))?;

        let mut stream = self
            .spotify
            .playlist_items(playlist_id, None, Some(Market::FromToken));
        let mut track_ids: Vec<TrackId> = Vec::new();

        while let Some(item) = stream.try_next().await? {
            if let Some(rspotify::model::PlayableItem::Track(track)) = item.track {
                if let Some(id) = track.id {
                    track_ids.push(id);
                }
            }
            report.total_tracks_in_playlist += 1;
        }

        report.tracks_processed = track_ids.len() as u32;

        if track_ids.is_empty() {
            report.final_liked_count = report.initial_liked_count;
            return Ok(report);
        }

        for (i, chunk) in track_ids.chunks(50).enumerate() {
            let batch_ids: Vec<String> = chunk.iter().map(|id| id.to_string()).collect();

            match self
                .spotify
                .current_user_saved_tracks_add(chunk.iter().cloned())
                .await
            {
                Ok(_) => {
                    report.batch_logs.push(SyncBatchLog {
                        batch_index: i,
                        tracks_count: chunk.len(),
                        track_ids: batch_ids,
                        status: "Success".to_string(),
                    });
                }
                Err(e) => {
                    report.batch_logs.push(SyncBatchLog {
                        batch_index: i,
                        tracks_count: chunk.len(),
                        track_ids: batch_ids,
                        status: format!("Error: {}", e),
                    });
                }
            }
        }

        report.final_liked_count = self.get_liked_songs_count().await?;

        if report.final_liked_count >= report.initial_liked_count {
            report.estimated_added = report.final_liked_count - report.initial_liked_count;
        }

        Ok(report)
    }

    /// Deduplicates 'Liked Songs' by removing dead tracks that share an ISRC with a living track.
    pub async fn deduplicate_liked_songs(&self) -> Result<Vec<String>, AuditError> {
        let mut stream = self.spotify.current_user_saved_tracks(None);
        let mut by_isrc: HashMap<String, Vec<FullTrack>> = HashMap::new();

        while let Some(item) = stream.try_next().await? {
            let track = item.track;
            if let Some(isrc) = track.external_ids.get("isrc") {
                by_isrc.entry(isrc.clone()).or_default().push(track);
            }
        }

        let mut tracks_to_remove: Vec<TrackId> = Vec::new();
        let mut removed_names: Vec<String> = Vec::new();

        for (isrc, tracks) in by_isrc {
            if tracks.len() > 1 {
                debug!("Checking ISRC {} with {} duplicates", isrc, tracks.len());

                // Sort by markets count (descending), so the best one is first.
                let mut sorted_tracks = tracks.clone();
                sorted_tracks.sort_by_key(|t| std::cmp::Reverse(t.available_markets.len()));

                let best_track = &sorted_tracks[0];
                let best_markets = best_track.available_markets.len();

                // If the best track has markets (is alive), we can safely remove the others if they have NO markets or FEWER markets.
                // Actually, if we have duplicate ISRCs in "Liked Songs", they are redundant regardless.
                // We should keep the "best" one and remove all others.

                if best_markets > 0 {
                    for duplicate in sorted_tracks.iter().skip(1) {
                        let dup_markets = duplicate.available_markets.len();

                        // Heuristic: Delete if it has 0 markets OR significantly fewer (e.g. strict subset logic is hard, but 0 is safe).
                        // Or if IDs are different?
                        if let Some(dup_id) = &duplicate.id {
                            if Some(dup_id) != best_track.id.as_ref() {
                                debug!("  -> Marking for removal: {} ({} markets) vs Keeper ({} markets)", duplicate.name, dup_markets, best_markets);
                                tracks_to_remove.push(dup_id.clone());
                                removed_names
                                    .push(format!("{} (Markets: {})", duplicate.name, dup_markets));
                            }
                        }
                    }
                }
            }
        }

        if !tracks_to_remove.is_empty() {
            info!(
                "Removing {} duplicate/dead tracks...",
                tracks_to_remove.len()
            );
            for chunk in tracks_to_remove.chunks(50) {
                self.spotify
                    .current_user_saved_tracks_delete(chunk.iter().cloned())
                    .await?;
            }
        }

        Ok(removed_names)
    }

    fn analyze_track(&self, track: &FullTrack) -> Option<ProblematicTrack> {
        let is_playable = track.is_playable.unwrap_or(true);

        if !is_playable {
            return Some(
                self.create_problem_report(track, "Track marked as unplayable by Spotify"),
            );
        }
        None
    }

    fn create_problem_report(&self, track: &FullTrack, reason: &str) -> ProblematicTrack {
        let artists = track
            .artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        let available_markets_count = track.available_markets.len();

        ProblematicTrack {
            id: track
                .id
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            name: track.name.clone(),
            artists,
            album: track.album.name.clone(),
            reason: reason.to_string(),
            external_url: track
                .external_urls
                .get("spotify")
                .cloned()
                .unwrap_or_default(),
            available_markets_count,
        }
    }
}
