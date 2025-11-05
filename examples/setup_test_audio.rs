use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎵 Setting up test audio files for AudioQueue...");
    println!("=================================================");

    // Create test_data directory
    let output_dir = Path::new("test_data");
    fs::create_dir_all(output_dir).context("Failed to create test_data directory")?;

    // Files to download with multiple fallback URLs
    let files = vec![
        (
            vec![
                "https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg",
                "https://filesamples.com/samples/audio/mp3/sample1.mp3",
                "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
            ],
            "test_data/SoundHelix-Song-1.mp3",
            "mp3"
        ),
        (
            vec![
                "https://upload.wikimedia.org/wikipedia/commons/3/3c/Boom_whip_crack.ogg",
                "https://filesamples.com/samples/audio/wav/sample1.wav",
                "https://www2.cs.uic.edu/~i101/SoundFiles/BabyElephantWalk60.wav",
            ],
            "test_data/Boom_whip_crack.wav",
            "wav"
        ),
    ];

    let mut downloaded = 0;

    for (urls, output_path, _format) in &files {
        println!("\nDownloading: {}", Path::new(output_path).file_name().unwrap().to_string_lossy());

        match download_file_with_fallback(urls, output_path).await {
            Ok(_) => {
                downloaded += 1;
                let size = fs::metadata(output_path)
                    .map(|m| m.len() / (1024 * 1024))
                    .unwrap_or(0);
                println!("✓ Downloaded ({} MB)", size);
            }
            Err(e) => {
                println!("✗ Failed all URLs: {}", e);
            }
        }
    }

    // Create simple playlist
    let playlist = format!(
        "# Test playlist for AudioQueue\n{}\n{}\n",
        Path::new(&files[0].1).file_name().unwrap().to_string_lossy(),
        Path::new(&files[1].1).file_name().unwrap().to_string_lossy()
    );

    fs::write("test_data/test.m3u", playlist).context("Failed to create playlist")?;

    println!("\n📊 Summary:");
    println!("=========");
    println!("Files downloaded: {}/{}", downloaded, files.len());
    println!("Output directory: test_data");
    println!("Playlist: test_data/test.m3u");

    if downloaded > 0 {
        println!("\n✅ Setup completed!");
        println!("Run tests: cargo test");
        println!("Run test suite: cargo run --example test_audioqueue");
    } else {
        println!("\n❌ No files downloaded. Check internet connection.");
    }

    Ok(())
}

async fn download_file_with_fallback(urls: &[&str], output_path: &str) -> Result<()> {
    let mut last_error = None;

    for (index, url) in urls.iter().enumerate() {
        println!("  Trying URL {}/{}: {}", index + 1, urls.len(), url);

        match download_file(url, output_path).await {
            Ok(_) => {
                println!("  ✓ Success!");
                return Ok(());
            }
            Err(e) => {
                println!("  ✗ Failed: {}", e);
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No URLs provided")))
}

async fn download_file(url: &str, output_path: &str) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .context("Failed to fetch URL")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP {}: {}",
            response.status(),
            response.status().canonical_reason().unwrap_or("Unknown")
        ));
    }

    let bytes = response.bytes()
        .await
        .context("Failed to read response body")?;

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(output_path)
        .await
        .context("Failed to create output file")?;

    file.write_all(&bytes)
        .await
        .context("Failed to write to file")?;

    file.flush()
        .await
        .context("Failed to flush file")?;

    Ok(())
}
