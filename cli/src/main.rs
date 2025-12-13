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

use audit_core::{get_spotify_client, Auditor};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use std::fs::File;
use std::io::Write;
use std::process;

#[derive(Parser)]
#[command(name = "spotify-audit")]
#[command(about = "A tool to audit and manage your Spotify library", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scans for problematic (unplayable) tracks. By default scans 'Liked Songs'.
    Scan {
        /// Output the report to a JSON file (e.g., --json=report.json)
        #[arg(long)]
        json: Option<String>,

        /// Optional: Scan a specific Playlist ID instead of 'Liked Songs'
        #[arg(long, short = 'p')]
        playlist: Option<String>,
    },
    /// Syncs all songs from a specific Playlist to your 'Liked Songs'
    Sync {
        /// The Spotify ID of the playlist to sync
        #[arg(value_name = "PLAYLIST_ID")]
        playlist_id: String,
        /// Output the detailed sync report to a JSON file
        #[arg(long)]
        json: Option<String>,
    },
    /// Lists all your playlists with their IDs
    List,
    /// Inspects a specific track ID to retrieve full forensic metadata
    Inspect {
        /// The Spotify Track ID to inspect
        #[arg(value_name = "TRACK_ID")]
        track_id: String,
    },
    /// Deduplicates 'Liked Songs' by removing dead tracks that share an ISRC with a living track.
    Dedup,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    if dotenv().is_err() {
        // Silently ignore
    }

    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { json, playlist } => {
            handle_scan(json.as_deref(), playlist.as_deref()).await;
        }
        Commands::Sync { playlist_id, json } => {
            handle_sync(playlist_id, json.as_deref()).await;
        }
        Commands::List => {
            handle_list().await;
        }
        Commands::Inspect { track_id } => {
            handle_inspect(track_id).await;
        }
        Commands::Dedup => {
            handle_dedup().await;
        }
    }
}

async fn get_auditor() -> Auditor {
    let spotify = match get_spotify_client().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error initializing Spotify client: {}", e);
            process::exit(1);
        }
    };
    Auditor::new(spotify)
}

async fn handle_dedup() {
    let auditor = get_auditor().await;
    println!("Starting Deduplication of Liked Songs...");
    println!("This will fetch your entire library to find ID conflicts. Please wait.");

    match auditor.deduplicate_liked_songs().await {
        Ok(removed) => {
            if removed.is_empty() {
                println!();
                println!("[OK] No safe duplicates found. Your library is clean.");
            } else {
                println!();
                println!("[CLEANUP] Removed {} dead duplicate tracks:", removed.len());
                for name in removed {
                    println!("   - {}", name);
                }
                println!();
                println!("(Note: We kept the playable versions of these tracks safe).");
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("Deduplication failed: {}", e);
            process::exit(1);
        }
    }
}

async fn handle_scan(json_path: Option<&str>, playlist_id: Option<&str>) {
    let auditor = get_auditor().await;

    let scan_result = if let Some(pid) = playlist_id {
        println!("Starting scan of Playlist ID: {} ...", pid);
        auditor.scan_playlist(pid).await
    } else {
        println!("Starting scan of Liked Songs...");
        auditor.scan_liked_songs().await
    };

    match scan_result {
        Ok(summary) => {
            println!();
            println!("---------------------------------------------------");
            println!("AUDIT REPORT");
            println!("---------------------------------------------------");
            println!(
                "Target:               {}",
                if playlist_id.is_some() {
                    "Playlist"
                } else {
                    "Liked Songs"
                }
            );
            println!("Total Tracks Scanned: {}", summary.total_tracks_scanned);
            println!("Problematic Tracks:   {}", summary.problematic_tracks.len());
            println!("---------------------------------------------------");

            if !summary.problematic_tracks.is_empty() {
                println!();
                println!("Found the following issues:");
                for (i, track) in summary.problematic_tracks.iter().enumerate() {
                    println!("{}. {}", i + 1, track);
                }

                println!();
                println!("Legend:");
                println!("  [REMOVED GLOBALLY]: Track has been removed from Spotify entirely (0 markets).");
                println!("  [GEO-LOCKED]:       Track is available in other countries but restricted in yours.");
            } else {
                println!();
                println!("No unplayable tracks found. Clean!");
            }

            if let Some(path) = json_path {
                match File::create(path) {
                    Ok(mut file) => {
                        let json_content =
                            serde_json::to_string_pretty(&summary).unwrap_or_default();
                        if let Err(e) = file.write_all(json_content.as_bytes()) {
                            eprintln!();
                            eprintln!("[ERROR] Failed to write report to file: {}", e);
                        } else {
                            println!();
                            println!("[SAVED] Report saved to: {}", path);
                        }
                    }
                    Err(e) => eprintln!("[ERROR] Failed to create file '{}': {}", path, e),
                }
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("Audit failed: {}", e);
            process::exit(1);
        }
    }
}

async fn handle_sync(playlist_id: &str, json_path: Option<&str>) {
    let auditor = get_auditor().await;

    println!("Syncing Playlist ID: {} to Liked Songs...", playlist_id);

    match auditor.sync_playlist_to_liked(playlist_id).await {
        Ok(report) => {
            println!();
            println!("---------------------------------------------------");
            println!("SYNC COMPLETE");
            println!("---------------------------------------------------");
            println!("Initial Liked Songs:      {}", report.initial_liked_count);
            println!(
                "Tracks in Source Playlist:{}",
                report.total_tracks_in_playlist
            );
            println!("Tracks Processed:         {}", report.tracks_processed);
            println!("Final Liked Songs:        {}", report.final_liked_count);
            println!("---------------------------------------------------");
            println!("Estimated New Tracks Added: {}", report.estimated_added);
            println!("---------------------------------------------------");

            if let Some(path) = json_path {
                match File::create(path) {
                    Ok(mut file) => {
                        let json_content =
                            serde_json::to_string_pretty(&report).unwrap_or_default();
                        if let Err(e) = file.write_all(json_content.as_bytes()) {
                            eprintln!();
                            eprintln!("[ERROR] Failed to write report to file: {}", e);
                        } else {
                            println!();
                            println!("[SAVED] Detailed report saved to: {}", path);
                        }
                    }
                    Err(e) => eprintln!("[ERROR] Failed to create file '{}': {}", path, e),
                }
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("[ERROR] Sync failed: {}", e);
            process::exit(1);
        }
    }
}

async fn handle_list() {
    let auditor = get_auditor().await;
    println!("Fetching your playlists...");

    match auditor.list_playlists().await {
        Ok(playlists) => {
            // Header
            println!();
            println!(
                "{:<25} | {:<30} | {:<20} | {:<6} | {:<5}",
                "ID", "Name", "Owner", "Tracks", "Collab"
            );
            println!(
                "{:-<25}-+-{:-<30}-+-{:-<20}-+-{:-<6}-+-{:-<5}",
                "", "", "", "", ""
            );

            for pl in playlists {
                let id = pl.id.replace("spotify:playlist:", "");

                let name = if pl.name.len() > 28 {
                    format!("{}..", &pl.name[0..28])
                } else {
                    pl.name
                };

                let owner = if pl.owner_name.len() > 18 {
                    format!("{}..", &pl.owner_name[0..18])
                } else {
                    pl.owner_name
                };

                let collab = if pl.is_collaborative { "Yes" } else { "No" };

                println!(
                    "{:<25} | {:<30} | {:<20} | {:<6} | {:<5}",
                    id, name, owner, pl.total_tracks, collab
                );
            }
            println!();
            println!("Tip: Copy an ID and run 'audit-cli sync <ID>'");
        }
        Err(e) => {
            eprintln!("Failed to list playlists: {}", e);
            process::exit(1);
        }
    }
}

async fn handle_inspect(track_id: &str) {
    let auditor = get_auditor().await;
    println!("Inspecting Track ID: {} ...", track_id);

    match auditor.inspect_track(track_id).await {
        Ok(info) => {
            println!();
            println!("TRACK FORENSICS");
            println!("---------------------------------------------------");
            println!("Name:          {}", info.name);
            println!("Artists:       {}", info.artists.join(", "));
            println!("Album:         {}", info.album);
            println!("Release Date:  {}", info.release_date);
            println!("Popularity:    {} / 100", info.popularity);
            println!("Is Playable:   {:?}", info.is_playable);
            println!("Local File:    {}", info.is_local);
            println!("---------------------------------------------------");
            println!("MARKETS ({})", info.available_markets.len());
            if info.available_markets.is_empty() {
                println!("   [REMOVED GLOBALLY] (0 markets)");
            } else if info.available_markets.len() > 10 {
                println!(
                    "   Available in {} markets (including: {}, ...)",
                    info.available_markets.len(),
                    info.available_markets
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            } else {
                println!("   {}", info.available_markets.join(", "));
            }
            println!("---------------------------------------------------");
            println!("EXTERNAL IDS");
            for (k, v) in &info.external_ids {
                println!("   {}: {}", k, v);
            }
            println!("---------------------------------------------------");
            println!("LINKS");
            for (k, v) in &info.external_urls {
                println!("   {}: {}", k, v);
            }
        }
        Err(e) => {
            eprintln!();
            eprintln!("[ERROR] Inspection failed: {}", e);
            process::exit(1);
        }
    }
}
