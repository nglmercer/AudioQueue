use std::path::{Path, PathBuf};
use anyhow::Result;

use audioqueue::{AudioQueue, AudioTrack, AudioEmitter};

fn collect_audio_entries() -> Vec<PathBuf> {
    let dir = std::env::var("TEST_AUDIO_FILES").unwrap_or_else(|_| "test_data".into());
    match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.extension()
                        .and_then(|s| s.to_str())
                        .map(|s| matches!(s, "mp3" | "wav" | "flac" | "ogg"))
                        .unwrap_or(false)
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn make_dummy_tracks() -> Vec<AudioTrack> {
    let mk = |path: &str, title: &str, artist: &str| AudioTrack {
        path: PathBuf::from(path),
        title: Some(title.to_string()),
        artist: Some(artist.to_string()),
        duration: Some(120.0),
        position: 0,
    };
    vec![
        mk("samples/track1.mp3", "Sample One", "Artist A"),
        mk("samples/track2.wav", "Sample Two", "Artist B"),
        mk("samples/track3.ogg", "Sample Three", "Artist C"),
    ]
}

fn main() -> Result<()> {
    println!("AudioQueue Basic Usage Example");
    println!("=============================");

    let mut queue = AudioQueue::new();

    // Try to load real audio files; if none, use dummy tracks
    let mut added = 0usize;
    let entries = collect_audio_entries();
    if !entries.is_empty() {
        println!("\nLoading real files from: {}", entries[0].parent().unwrap_or(Path::new(".")).display());
        for path in entries.iter().take(3) {
            match AudioQueue::extract_metadata(path) {
                Ok(track) => {
                    queue.add_track(track, None)?;
                    added += 1;
                }
                Err(e) => {
                    eprintln!("  Skipping {}: {}", path.display(), e);
                }
            }
        }
    }

    if added == 0 {
        println!("\nNo real audio files found. Using dummy tracks instead.");
        for t in make_dummy_tracks() {
            queue.add_track(t, None)?;
        }
    }

    println!("\nCurrent queue:\n{}", queue.display_queue());

    // Start actual audio playback using AudioEmitter
    // Use AudioEmitter for real audio playback
    println!("\nStarting audio playback...");

    if !queue.get_queue().is_empty() {
        println!("Found {} tracks in queue", queue.get_queue().len());

        // Try to create AudioEmitter
        println!("Creating AudioEmitter...");
        let mut emitter = match AudioEmitter::new() {
            Ok(e) => {
                println!("AudioEmitter created successfully");
                e
            }
            Err(e) => {
                eprintln!("Failed to create AudioEmitter: {}", e);
                return Ok(());
            }
        };

        // Start queue to set current track
        queue.play()?;

        // Load and play the first track
        if let Some(track) = queue.get_current_track() {
            let file_path = track.path.to_string_lossy().to_string();
            println!("Attempting to load: {}", file_path);

            match emitter.load_file(&track.path) {
                Ok(_) => {
                    println!("File loaded successfully");
                    println!("Playing: {}", file_path);

                    match emitter.play() {
                        Ok(_) => {
                            println!("Playback started successfully");

                            // Let it play for a few seconds
                            println!("Playing for 3 seconds...");
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            println!("Finished playing");

                            // Stop playback
                            emitter.stop()?;
                            println!("Playback stopped.");
                        }
                        Err(e) => {
                            eprintln!("Error starting playback: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error loading track: {}", e);
                }
            }
        } else {
            println!("No current track in queue");
        }
    } else {
        println!("No tracks in queue to play.");
    }

    Ok(())
}
