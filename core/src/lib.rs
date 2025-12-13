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

pub mod audit;
pub mod auth;
pub mod models;

// Re-export key items for convenience
pub use audit::Auditor;
pub use auth::get_spotify_client;
pub use models::{AuditSummary, ProblematicTrack};
