use std::path::PathBuf;
use std::time::Duration;
use anyhow::Result;
use audioqueue::audio_emitter::AudioEmitter;

fn collect_audio_entries() -> Vec<PathBuf> {
    let test_files = std::env::var("TEST_AUDIO_FILES")
        .unwrap_or_else(|_| "test_data".to_string());

    match std::fs::read_dir(&test_files) {
        Ok(read_dir) => read_dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| matches!(s, "mp3" | "wav" | "flac" | "ogg"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    }
}

fn audio_tests_enabled() -> bool {
    matches!(std::env::var("RUN_AUDIO_TESTS").as_deref(), Ok("1") | Ok("true") | Ok("TRUE"))
}

#[tokio::test]
async fn test_basic_playback() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let mut entries = collect_audio_entries();

    entries.sort();

    if entries.is_empty() {
        println!("No test audio files found, skipping test");
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();

    println!("Testing with: {}", file_path);

    let mut emitter = match AudioEmitter::new() {
        Ok(e) => e,
        Err(_) => {
            println!("Skipping: no audio device available");
            return Ok(());
        }
    };

    // Load and play
    emitter.load_file(&file_path)?;
    emitter.play()?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Pause
    emitter.pause()?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Resume
    emitter.resume()?;
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Stop
    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_playback_state_transitions() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let mut entries = collect_audio_entries();

    entries.sort();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = match AudioEmitter::new() {
        Ok(e) => e,
        Err(_) => {
            println!("Skipping: no audio device available");
            return Ok(());
        }
    };

    emitter.load_file(&file_path)?;
    emitter.play()?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let (state, _, _, _, _) = emitter.get_status();
    println!("State after play: {:?}", state);

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_sequential_playback() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let mut entries = collect_audio_entries();
    entries.truncate(2);

    if entries.len() < 2 {
        println!("Need at least 2 audio files for this test");
        return Ok(());
    }

    let mut emitter = AudioEmitter::new()?;

    for entry in entries {
        let file_path = entry.to_string_lossy().to_string();
        println!("Playing: {}", file_path);

        emitter.load_file(&file_path)?;
        emitter.play()?;
        tokio::time::sleep(Duration::from_secs(1)).await;
        emitter.stop()?;
    }

    Ok(())
}

#[tokio::test]
async fn test_repeated_start_stop() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;

    for i in 0..3 {
        println!("Iteration {}", i + 1);
        emitter.play()?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        emitter.stop()?;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Ok(())
}

#[tokio::test]
async fn test_volume_changes() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;
    emitter.play()?;

    for volume in [1.0, 0.5, 0.2, 0.8, 1.0] {
        emitter.set_volume(volume)?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_pause_resume_timing() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;
    emitter.play()?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Pause
    emitter.pause()?;
    let pause_time = std::time::Instant::now();

    tokio::time::sleep(Duration::from_secs(2)).await;

    // Resume
    emitter.resume()?;
    let resume_elapsed = pause_time.elapsed();

    println!("Paused for: {:?}", resume_elapsed);

    tokio::time::sleep(Duration::from_secs(1)).await;

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_rapid_control_changes() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let mut emitter = AudioEmitter::new()?;

    for _ in 0..3 {
        for entry in &entries {
            emitter.stop()?;
            emitter.load_file(entry)?;
            emitter.play()?;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_playback_completion() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;
    emitter.play()?;

    // Check completion status periodically
    for i in 0..20 {
        tokio::time::sleep(Duration::from_millis(500)).await;

        if emitter.is_finished() {
            println!("Playback completed after {} checks", i + 1);
            break;
        }
    }

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_invalid_file_handling() -> Result<()> {
    let mut emitter = AudioEmitter::new()?;

    let result = emitter.load_file("nonexistent.mp3");
    assert!(result.is_err());

    let result = emitter.load_file("Cargo.toml");
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_multiple_format_support() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let mut emitter = AudioEmitter::new()?;

    for entry in entries {
        let file_path = entry.to_string_lossy().to_string();
        println!("Testing format: {}", file_path);

        emitter.load_file(&file_path)?;
        emitter.play()?;
        tokio::time::sleep(Duration::from_millis(500)).await;
        emitter.stop()?;
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;

    for _ in 0..5 {
        emitter.play()?;
        emitter.set_volume(0.5)?;
        emitter.pause()?;
        emitter.set_volume(1.0)?;
        emitter.resume()?;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    emitter.stop()?;

    Ok(())
}

#[tokio::test]
async fn test_seek_functionality() -> Result<()> {
    if !audio_tests_enabled() {
        println!("Skipping: set RUN_AUDIO_TESTS=1 to enable");
        return Ok(());
    }
    let entries = collect_audio_entries();

    if entries.is_empty() {
        return Ok(());
    }

    let file_path = entries[0].to_string_lossy().to_string();
    let mut emitter = AudioEmitter::new()?;

    emitter.load_file(&file_path)?;
    emitter.play()?;

    for position in [1.0, 2.0, 0.5] {
        let _ = emitter.seek(position);
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    emitter.stop()?;

    Ok(())
}
