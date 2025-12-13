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

use rspotify::{prelude::*, scopes, AuthCodeSpotify, Config, Credentials, OAuth};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Failed to initialize Spotify client: {0}")]
    ClientConfig(String),
    #[error("Spotify authentication failed: {0}")]
    Spotify(#[from] rspotify::ClientError),
}

/// Initializes and authenticates a Spotify client using the Authorization Code Flow.
///
/// This function:
/// 1. Reads credentials (`RSPOTIFY_CLIENT_ID`, `RSPOTIFY_CLIENT_SECRET`) from the environment.
/// 2. Reads the redirect URI (`RSPOTIFY_REDIRECT_URI`) from the environment.
/// 3. Requests necessary scopes for auditing (library read/write, playlist read).
/// 4. Handles the OAuth2 flow, including token caching and refreshing.
///
/// If a valid token is not cached, it will prompt the user (via stdout) to visit a URL
/// to authorize the application.
pub async fn get_spotify_client() -> Result<AuthCodeSpotify, AuthError> {
    // Load credentials from env. `rspotify` expects RSPOTIFY_CLIENT_ID/SECRET.
    let creds = Credentials::from_env().ok_or_else(|| {
        AuthError::ClientConfig("Missing RSPOTIFY_CLIENT_ID or RSPOTIFY_CLIENT_SECRET".to_string())
    })?;

    // Define scopes required for the audit functionality.
    // - user-library-read: To check Liked Songs.
    // - user-library-modify: To add songs to Liked Songs (sync feature).
    // - playlist-read-private: To read user's private playlists.
    // - playlist-read-collaborative: To read collaborative playlists.
    let scopes = scopes!(
        "user-library-read",
        "user-library-modify",
        "playlist-read-private",
        "playlist-read-collaborative"
    );

    // Load OAuth config (Redirect URI) from env.
    let oauth = OAuth::from_env(scopes)
        .ok_or_else(|| AuthError::ClientConfig("Missing RSPOTIFY_REDIRECT_URI".to_string()))?;

    // Configure the client.
    // `token_cached: true` enables saving the token to a file (default: .spotify_token_cache.json).
    let config = Config {
        token_cached: true,
        token_refreshing: true,
        ..Default::default()
    };

    let spotify = AuthCodeSpotify::with_config(creds, oauth, config);

    // Get the authorization URL.
    let url = spotify.get_authorize_url(false)?;

    // This method from the `cli` feature of rspotify handles the interaction:
    // 1. Tries to open the URL in the default browser.
    // 2. If that fails, prints the URL to stdout.
    // 3. Waits for the redirect URI to be hit (if running a local server) or input.
    // Note: Since we are using a localhost redirect, rspotify usually spins up a tiny server
    // to catch the callback if the port matches the redirect URI.
    spotify.prompt_for_token(&url).await?;

    Ok(spotify)
}
