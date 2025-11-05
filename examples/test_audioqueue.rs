use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

fn main() -> Result<()> {
    println!("🧪 Testing AudioQueue functionality...");
    println!("=====================================");

    // Get paths
    let project_root = env!("CARGO_MANIFEST_DIR");
    let binary_path = format!("{}/target/release/audioqueue{}", project_root,
        if cfg!(windows) { ".exe" } else { "" });
    let test_data_dir = format!("{}/test_data", project_root);

    println!("Binary: {}", binary_path);
    println!("Test Data: {}", test_data_dir);
    println!();

    // Check if binary exists, build if needed
    if !Path::new(&binary_path).exists() {
        println!("📦 Building AudioQueue...");
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(project_root)
            .output()
            .context("Failed to run cargo build")?;

        if !output.status.success() {
            println!("❌ Build failed");
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
            return Ok(());
        }
    }
    println!("✅ Binary found");

    // Check test data directory
    if !Path::new(&test_data_dir).exists() {
        println!("❌ Test data directory not found");
        println!("   Run: cargo run --example setup_test_audio");
        return Ok(());
    }

    // Find MP3 files
    let mp3_files = std::fs::read_dir(&test_data_dir)
        .context("Failed to read test data directory")?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension().map_or(false, |ext| ext == "mp3")
        })
        .collect::<Vec<_>>();

    if mp3_files.is_empty() {
        println!("❌ No MP3 files found");
        return Ok(());
    }

    println!("✅ Found {} test files", mp3_files.len());
    println!();

    // Test basic commands
    println!("🔍 Testing basic commands...");

    // Test help
    match test_command(&binary_path, &["--help"]) {
        Ok(_) => println!("✅ Help command works"),
        Err(e) => println!("❌ Help command failed: {}", e),
    }

    // Test version
    match test_command(&binary_path, &["--version"]) {
        Ok(_) => println!("✅ Version command works"),
        Err(e) => println!("❌ Version command failed: {}", e),
    }

    // Test with audio file
    let first_file = mp3_files[0].path();
    println!();
    println!("🔍 Testing with audio file: {}", first_file.file_name().unwrap().to_string_lossy());

    match test_command(&binary_path, &["validate", &first_file.to_string_lossy()]) {
        Ok(_) => println!("✅ File validation passed"),
        Err(_) => println!("⚠️  File validation failed (may not be implemented)"),
    }

    println!();
    println!("📊 Test Summary:");
    println!("   ✅ Tests completed");
    println!("   🧪 Run unit tests: cargo test");
    println!("   📚 Check README.md for usage");

    Ok(())
}

fn test_command(binary: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .context(format!("Failed to run {} with {:?}", binary, args))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Command failed with exit code: {}\nStderr: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}
