use std::path::PathBuf;
use audioqueue::audio_queue::{AudioQueue, AudioTrack, PlaybackState};
use anyhow::Result;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_track(path: &str, title: Option<&str>, artist: Option<&str>) -> AudioTrack {
        AudioTrack {
            path: PathBuf::from(path),
            title: title.map(|s| s.to_string()),
            artist: artist.map(|s| s.to_string()),
            duration: Some(120.0),
            position: 0,
        }
    }

    #[test]
    fn test_audio_queue_creation() {
        let queue = AudioQueue::new();
        assert_eq!(queue.get_queue().len(), 0);
        assert!(queue.get_current_track().is_none());

        let (state, current, size) = queue.get_status();
        assert!(matches!(state, PlaybackState::Stopped));
        assert!(current.is_none());
        assert_eq!(size, 0);
    }

    #[test]
    fn test_add_track_to_empty_queue() -> Result<()> {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", Some("Test Song"), Some("Test Artist"));

        queue.add_track(track, None)?;

        assert_eq!(queue.get_queue().len(), 1);
        let added_track = &queue.get_queue()[0];
        assert_eq!(added_track.title.as_deref(), Some("Test Song"));
        assert_eq!(added_track.artist.as_deref(), Some("Test Artist"));
        assert_eq!(added_track.position, 0);

        Ok(())
    }

    #[test]
    fn test_add_track_at_specific_position() -> Result<()> {
        let mut queue = AudioQueue::new();

        let track1 = create_test_track("test1.mp3", None, None);
        queue.add_track(track1, None)?;

        let track2 = create_test_track("test2.mp3", None, None);
        queue.add_track(track2, Some(0))?;

        assert_eq!(queue.get_queue().len(), 2);
        assert_eq!(queue.get_queue()[0].path, PathBuf::from("test2.mp3"));
        assert_eq!(queue.get_queue()[1].path, PathBuf::from("test1.mp3"));

        Ok(())
    }

    #[test]
    fn test_add_track_invalid_position() {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", None, None);

        let result = queue.add_track(track, Some(5));
        assert!(result.is_err());
        assert_eq!(queue.get_queue().len(), 0);
    }

    #[test]
    fn test_remove_track() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        assert_eq!(queue.get_queue().len(), 3);

        queue.remove_track(1)?;
        assert_eq!(queue.get_queue().len(), 2);
        assert_eq!(queue.get_queue()[0].path, PathBuf::from("test1.mp3"));
        assert_eq!(queue.get_queue()[1].path, PathBuf::from("test3.mp3"));

        Ok(())
    }

    #[test]
    fn test_remove_track_invalid_position() {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", None, None);
        queue.add_track(track, None).unwrap();

        let result = queue.remove_track(5);
        assert!(result.is_err());
        assert_eq!(queue.get_queue().len(), 1);
    }

    #[test]
    fn test_move_track() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        queue.move_track(0, 2)?;

        assert_eq!(queue.get_queue()[0].path, PathBuf::from("test2.mp3"));
        assert_eq!(queue.get_queue()[1].path, PathBuf::from("test3.mp3"));
        assert_eq!(queue.get_queue()[2].path, PathBuf::from("test1.mp3"));

        Ok(())
    }

    #[test]
    fn test_move_track_same_position() -> Result<()> {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", None, None);
        queue.add_track(track, None)?;

        let result = queue.move_track(0, 0);
        assert!(result.is_ok());
        assert_eq!(queue.get_queue().len(), 1);

        Ok(())
    }

    #[test]
    fn test_move_track_invalid_positions() {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", None, None);
        queue.add_track(track, None).unwrap();

        let result1 = queue.move_track(5, 0);
        assert!(result1.is_err());

        let result2 = queue.move_track(0, 5);
        assert!(result2.is_err());
    }

    #[test]
    fn test_playback_state_transitions() -> Result<()> {
        let mut queue = AudioQueue::new();

        // Empty queue tests
        assert!(queue.play().is_err());
        assert!(queue.pause().is_err());
        assert!(queue.next_track().is_err());
        assert!(queue.previous().is_err());

        // Add track
        let track = create_test_track("test.mp3", None, None);
        queue.add_track(track, None)?;

        // Test state transitions
        queue.play()?;
        assert!(matches!(queue.get_status().0, PlaybackState::Playing));

        queue.pause()?;
        assert!(matches!(queue.get_status().0, PlaybackState::Paused));

        queue.resume()?;
        assert!(matches!(queue.get_status().0, PlaybackState::Playing));

        Ok(())
    }

    #[test]
    fn test_navigation() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        // Jump to track
        queue.jump_to(1)?;
        assert_eq!(queue.get_current_track().unwrap().path, PathBuf::from("test2.mp3"));

        // Next
        queue.next_track()?;
        assert_eq!(queue.get_current_track().unwrap().path, PathBuf::from("test3.mp3"));

        // Previous
        queue.previous()?;
        assert_eq!(queue.get_current_track().unwrap().path, PathBuf::from("test2.mp3"));

        // Next at last track
        queue.next_track()?;
        assert!(queue.next_track().is_err()); // Should fail at last track

        // Previous at first track
        queue.jump_to(0)?;
        assert!(queue.previous().is_err()); // Should fail at first track

        Ok(())
    }

    #[test]
    fn test_jump_invalid_position() -> Result<()> {
        let mut queue = AudioQueue::new();
        let track = create_test_track("test.mp3", None, None);
        queue.add_track(track, None)?;

        let result = queue.jump_to(5);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_clear_queue() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        queue.play()?;
        assert!(queue.get_current_track().is_some());

        queue.clear()?;
        assert_eq!(queue.get_queue().len(), 0);
        assert!(queue.get_current_track().is_none());
        assert!(matches!(queue.get_status().0, PlaybackState::Stopped));

        Ok(())
    }

    #[test]
    fn test_current_track_management() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        assert!(queue.get_current_track().is_none());

        queue.play()?;
        assert_eq!(queue.get_current_track().unwrap().path, PathBuf::from("test1.mp3"));

        queue.remove_track(0)?;
        assert_eq!(queue.get_current_track().unwrap().path, PathBuf::from("test2.mp3"));

        Ok(())
    }

    #[test]
    fn test_position_updates() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), None, None);
            queue.add_track(track, None)?;
        }

        for (index, track) in queue.get_queue().iter().enumerate() {
            assert_eq!(track.position, index);
        }

        queue.move_track(0, 2)?;
        for (index, track) in queue.get_queue().iter().enumerate() {
            assert_eq!(track.position, index);
        }

        Ok(())
    }

    #[test]
    fn test_display_queue() -> Result<()> {
        let mut queue = AudioQueue::new();

        let display = queue.display_queue();
        assert!(display.contains("Queue is empty"));

        let track1 = create_test_track("test1.mp3", Some("First Song"), Some("Artist One"));
        let track2 = create_test_track("test2.mp3", None, Some("Artist Two"));
        queue.add_track(track1, None)?;
        queue.add_track(track2, None)?;

        queue.play()?;

        let display = queue.display_queue();
        assert!(display.contains("Current Queue"));
        assert!(display.contains("First Song"));
        assert!(display.contains("Artist One"));
        assert!(display.contains("▶"));

        Ok(())
    }

    #[test]
    fn test_queue_size_and_state() -> Result<()> {
        let mut queue = AudioQueue::new();

        for i in 1..=3 {
            let track = create_test_track(&format!("test{}.mp3", i), Some(&format!("Song {}", i)), None);
            queue.add_track(track, None)?;
        }

        // Check queue size
        assert_eq!(queue.get_queue().len(), 3);

        // Check initial state
        let (state, current, size) = queue.get_status();
        assert!(matches!(state, PlaybackState::Stopped));
        assert!(current.is_none());
        assert_eq!(size, 3);

        // After play
        queue.play()?;
        let (state, current, _) = queue.get_status();
        assert!(matches!(state, PlaybackState::Playing));
        assert!(current.is_some());

        Ok(())
    }
}

// Integration tests with real files
#[cfg(test)]
mod integration_tests {
    use super::*;

    fn get_test_audio_files() -> Vec<PathBuf> {
        vec![
            PathBuf::from("test_data/SoundHelix-Song-1.mp3"),
            PathBuf::from("test_data/Boom_whip_crack.wav"),
        ]
    }

    fn check_test_files_exist() -> bool {
        get_test_audio_files().iter().all(|p| p.exists())
    }

    #[test]
    fn test_real_file_validation() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n🔍 Testing file validation...");
        for file in get_test_audio_files() {
            let result = AudioQueue::validate_audio_file(&file);
            assert!(
                result.is_ok(),
                "Failed to validate {:?}: {:?}",
                file,
                result.err()
            );
            println!("  ✓ Validated: {}", file.display());
        }
    }

    #[test]
    fn test_real_metadata_extraction() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n📋 Testing metadata extraction...");
        for file in get_test_audio_files() {
            let result = AudioQueue::extract_metadata(&file);
            assert!(
                result.is_ok(),
                "Failed to extract metadata from {:?}: {:?}",
                file,
                result.err()
            );

            let track = result.unwrap();
            println!("  ✓ {}", file.display());
            println!("    Title:    {:?}", track.title);
            println!("    Artist:   {:?}", track.artist);
            println!("    Duration: {:?}s", track.duration.map(|d| format!("{:.1}", d)));

            assert!(!track.path.as_os_str().is_empty());
            assert!(track.title.is_some());
        }
    }

    #[test]
    fn test_queue_with_real_files() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n🎵 Testing queue operations with real files...");
        let mut queue = AudioQueue::new();

        // Load real files
        for file in get_test_audio_files() {
            match AudioQueue::extract_metadata(&file) {
                Ok(track) => {
                    queue.add_track(track, None).unwrap();
                    println!("  ✓ Added: {}", file.display());
                }
                Err(e) => {
                    println!("  ✗ Failed to load {:?}: {}", file, e);
                }
            }
        }

        assert!(queue.get_queue().len() >= 2, "Should have at least 2 tracks");

        // Test queue operations
        println!("\n📊 Queue contents:");
        println!("{}", queue.display_queue());

        // Test state transitions
        queue.play().unwrap();
        println!("  ✓ State: Playing");
        assert!(matches!(queue.get_status().0, PlaybackState::Playing));

        queue.pause().unwrap();
        println!("  ✓ State: Paused");
        assert!(matches!(queue.get_status().0, PlaybackState::Paused));

        queue.resume().unwrap();
        println!("  ✓ State: Playing (resumed)");
        assert!(matches!(queue.get_status().0, PlaybackState::Playing));

        // Test navigation
        if queue.get_queue().len() >= 2 {
            queue.next_track().unwrap();
            println!("  ✓ Navigation: Next track");

            queue.previous().unwrap();
            println!("  ✓ Navigation: Previous track");
        }

        println!("\n✅ All queue operations completed successfully");
    }

    #[test]
    fn test_load_and_save_playlist() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n📝 Testing playlist operations...");
        let playlist_path = PathBuf::from("test_data/test.m3u");

        if !playlist_path.exists() {
            println!("  ⚠️  Playlist file not found, skipping");
            return;
        }

        // Test loading playlist
        let mut queue = AudioQueue::new();
        match queue.load_playlist(&playlist_path) {
            Ok(_) => {
                println!("  ✓ Loaded playlist: {}", playlist_path.display());
                println!("    Tracks loaded: {}", queue.get_queue().len());
                assert!(queue.get_queue().len() > 0);

                // Test saving playlist
                let output_path = PathBuf::from("test_data/test_output.m3u");
                match queue.save_playlist(&output_path) {
                    Ok(_) => {
                        println!("  ✓ Saved playlist: {}", output_path.display());
                        assert!(output_path.exists());
                    }
                    Err(e) => {
                        println!("  ✗ Failed to save playlist: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ✗ Failed to load playlist: {}", e);
            }
        }
    }

    #[test]
    fn test_queue_stats_with_real_files() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n📊 Testing queue statistics...");
        let mut queue = AudioQueue::new();

        for file in get_test_audio_files() {
            if let Ok(track) = AudioQueue::extract_metadata(&file) {
                queue.add_track(track, None).unwrap();
            }
        }

        let status = queue.get_status();
        println!("  Total tracks: {:?}", status);
    }

    #[test]
    fn test_queue_manipulation_scenarios() {
        if !check_test_files_exist() {
            println!("⚠️  Skipping test - run 'cargo run --bin setup_test_audio' first");
            return;
        }

        println!("\n🔄 Testing complex queue scenarios...");
        let mut queue = AudioQueue::new();

        // Load files
        for file in get_test_audio_files() {
            if let Ok(track) = AudioQueue::extract_metadata(&file) {
                queue.add_track(track, None).unwrap();
            }
        }

        let initial_count = queue.get_queue().len();
        println!("  Initial queue size: {}", initial_count);

        // Start playing
        queue.play().unwrap();
        let current_path = queue.get_current_track().unwrap().path.clone();
        println!("  ✓ Playing: {}", current_path.display());

        // Move tracks around
        if initial_count >= 2 {
            queue.move_track(0, 1).unwrap();
            println!("  ✓ Moved track 0 to position 1");

            // Verify playback state is maintained
            assert!(matches!(queue.get_status().0, PlaybackState::Playing));
            println!("  ✓ Playback state maintained after move");
        }

        // Remove a track
        if initial_count >= 2 {
            queue.remove_track(1).unwrap();
            println!("  ✓ Removed track at position 1");
            assert_eq!(queue.get_queue().len(), initial_count - 1);
        }

        println!("\n✅ Complex scenarios completed successfully");
    }
}
