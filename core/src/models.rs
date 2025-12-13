/*
    spotify-audit-rs | Rust CLI tool to audit playlists and sync Liked Songs.
    Copyright (C) 2025  Israel Alberto Roldan Vega

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a track that is found to be problematic (grey/unplayable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblematicTrack {
    pub id: String,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub reason: String, // Technical reason (e.g. "Track marked as unplayable")
    pub external_url: String,
    pub available_markets_count: usize, // How many markets have this track?
}

impl fmt::Display for ProblematicTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.available_markets_count == 0 {
            "🔴 REMOVED GLOBALLY".to_string()
        } else {
            format!(
                "🌍 GEO-LOCKED (Available in {} markets)",
                self.available_markets_count
            )
        };

        write!(
            f,
            "[{}] {} - {} (Album: {}) -> {} | {}",
            self.id, self.name, self.artists, self.album, self.reason, status
        )
    }
}

/// Summary of a library scan.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_tracks_scanned: u32,
    pub problematic_tracks: Vec<ProblematicTrack>,
}

impl AuditSummary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_problem(&mut self, track: ProblematicTrack) {
        self.problematic_tracks.push(track);
    }
}

/// Detailed log for a sync operation batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncBatchLog {
    pub batch_index: usize,
    pub tracks_count: usize,
    pub track_ids: Vec<String>,
    pub status: String, // "Success" or error message
}

/// Report for the sync operation.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncReport {
    pub initial_liked_count: u32,
    pub final_liked_count: u32,
    pub total_tracks_in_playlist: u32,
    pub tracks_processed: u32,
    pub estimated_added: u32, // final - initial
    pub batch_logs: Vec<SyncBatchLog>,
}

/// Summary of a playlist for listing purposes.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: String,
    pub name: String,
    pub total_tracks: u32,
    pub is_public: bool,
    pub is_collaborative: bool,
    pub owner_name: String,
}

/// Detailed forensic information about a single track.
#[derive(Debug, Serialize, Deserialize)]
pub struct TrackInspection {
    pub id: String,
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub release_date: String,
    pub duration_ms: u32,
    pub popularity: u32,
    pub is_playable: Option<bool>,
    pub available_markets: Vec<String>,
    pub external_ids: std::collections::HashMap<String, String>, // ISRC, EAN, UPC
    pub external_urls: std::collections::HashMap<String, String>,
    pub disc_number: i32,
    pub track_number: u32,
    pub is_local: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_problematic_track_display_global_removed() {
        let track = ProblematicTrack {
            id: "123".to_string(),
            name: "Ghost Track".to_string(),
            artists: "Unknown Artist".to_string(),
            album: "Lost Album".to_string(),
            reason: "Unplayable".to_string(),
            external_url: "http://...".to_string(),
            available_markets_count: 0,
        };

        let display = format!("{}", track);
        assert!(display.contains("🔴 REMOVED GLOBALLY"));
        assert!(display.contains("Ghost Track"));
    }

    #[test]
    fn test_problematic_track_display_geo_locked() {
        let track = ProblematicTrack {
            id: "456".to_string(),
            name: "Locked Song".to_string(),
            artists: "Famous Singer".to_string(),
            album: "Region Album".to_string(),
            reason: "Unplayable".to_string(),
            external_url: "http://...".to_string(),
            available_markets_count: 5,
        };

        let display = format!("{}", track);
        assert!(display.contains("🌍 GEO-LOCKED"));
        assert!(display.contains("Available in 5 markets"));
    }

    #[test]
    fn test_audit_summary_aggregation() {
        let mut summary = AuditSummary::new();
        assert_eq!(summary.total_tracks_scanned, 0);
        assert!(summary.problematic_tracks.is_empty());

        let track = ProblematicTrack {
            id: "1".to_string(),
            name: "A".to_string(),
            artists: "B".to_string(),
            album: "C".to_string(),
            reason: "D".to_string(),
            external_url: "E".to_string(),
            available_markets_count: 0,
        };

        summary.add_problem(track);
        summary.total_tracks_scanned += 1;

        assert_eq!(summary.total_tracks_scanned, 1);
        assert_eq!(summary.problematic_tracks.len(), 1);
        assert_eq!(summary.problematic_tracks[0].name, "A");
    }
}
