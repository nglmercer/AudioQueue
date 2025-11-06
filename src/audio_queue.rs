use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use std::fs::{File, self};
use std::io::{BufReader, BufRead, Write};
use anyhow::{Result, anyhow, Context};

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::sync::mpsc::{self, Sender};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioTrack {
    pub path: PathBuf,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Option<f64>,
    pub position: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioQueueState {
    pub tracks: Vec<AudioTrack>,
    pub current_position: Option<usize>,
    pub playback_state: PlaybackState,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum QueueCommand {
    Add(AudioTrack, Option<usize>),
    Remove(usize),
    Move(usize, usize),
    Play,
    Pause,
    Resume,
    Next,
    Previous,
    Jump(usize),
    Clear,
    GetStatus,
}

#[derive(Debug)]
pub struct AudioQueue {
    pub tracks: VecDeque<AudioTrack>,
    pub current_position: Option<usize>,
    pub playback_state: PlaybackState,
    command_sender: Option<Sender<QueueCommand>>,
}

impl std::fmt::Display for AudioTrack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {} ({:.1}s)",
            self.title.as_deref().unwrap_or("Unknown"),
            self.artist.as_deref().unwrap_or("Unknown Artist"),
            self.duration.unwrap_or(0.0)
        )
    }
}

#[allow(dead_code)]
impl AudioQueue {
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
            current_position: None,
            playback_state: PlaybackState::Stopped,
            command_sender: None,
        }
    }

    /// Load queue state from file
    pub fn load_state<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)
                .context("Failed to read queue state file")?;
            let state: AudioQueueState = serde_json::from_str(&content)
                .context("Failed to parse queue state file")?;

            let (tx, _) = mpsc::channel(100);
            Ok(Self {
                tracks: state.tracks.into(),
                current_position: state.current_position,
                playback_state: state.playback_state,
                command_sender: Some(tx),
            })
        } else {
            Ok(Self::new())
        }
    }

    /// Save current state to file
    pub fn save_state<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let state = AudioQueueState {
            tracks: self.tracks.iter().cloned().collect(),
            current_position: self.current_position,
            playback_state: self.playback_state,
        };

        let content = serde_json::to_string_pretty(&state)
            .context("Failed to serialize queue state")?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create queue state directory")?;
        }

        fs::write(path, content)
            .context("Failed to write queue state file")
    }

    pub fn get_command_sender(&self) -> Option<Sender<QueueCommand>> {
        self.command_sender.clone()
    }

    pub fn set_command_sender(&mut self, sender: Sender<QueueCommand>) {
        self.command_sender = Some(sender);
    }

    pub fn validate_audio_file<P: AsRef<Path>>(path: P) -> Result<bool> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Ok(false);
        }

        // Try to open and probe file with symphonia
        let file = std::fs::File::open(path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        match symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts) {
            Ok(probed) => {
                // Check if we can get to format reader
                let _format = probed.format;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    pub fn extract_metadata<P: AsRef<Path>>(path: P) -> Result<AudioTrack> {
        let path = path.as_ref();

        // Convert to absolute path
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
                .join(path)
                .canonicalize()
                .context("Failed to canonicalize path")?
        };

        let file = std::fs::File::open(&absolute_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let mut probed = symphonia::default::get_probe()
            .format(
                &hint, mss, &fmt_opts, &meta_opts)?;

        let mut title = None;
        let mut artist = None;
        let mut duration = None;

        // Try to get metadata from all metadata revisions
        if let Some(metadata) = probed.format.metadata().current() {
            for tag in metadata.tags() {
                match tag.key.as_str() {
                    // Standard tag names
                    "TITLE" | "TIT2" => title = Some(tag.value.to_string()),
                    "ARTIST" | "TPE1" => artist = Some(tag.value.to_string()),
                    // Alternative tag names
                    "TITLE\x00" => title = Some(tag.value.to_string()),
                    "ARTIST\x00" => artist = Some(tag.value.to_string()),
                    _ => {}
                }
            }
        }

        // If no title found, try to extract from filename
        if title.is_none() {
            if let Some(file_name) = path.file_stem() {
                if let Some(name_str) = file_name.to_str() {
                    title = Some(name_str.to_string());
                }
            }
        }

        // If still no title, use a default
        if title.is_none() {
            title = Some("Unknown Title".to_string());
        }

        // If no artist found, use a default
        if artist.is_none() {
            artist = Some("Unknown Artist".to_string());
        }

        // Try to get duration from the format reader
        let mut track_id = None;
        if let Some(track) = probed.format.tracks().iter().find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL) {
            track_id = Some(track.id);
        }

        if let Some(track_id) = track_id {
            if let Some(codec_params) = probed.format.tracks().iter()
                .find(|t| t.id == track_id)
                .map(|t| t.codec_params.clone()) {
                if let Some(n_frames) = codec_params.n_frames {
                    if let Some(sample_rate) = codec_params.sample_rate {
                        duration = Some(n_frames as f64 / sample_rate as f64);
                    }
                }
            }
        }

        Ok(AudioTrack {
            path: absolute_path,
            title,
            artist,
            duration,
            position: 0,
        })
    }

    pub fn add_track(&mut self, mut track: AudioTrack, position: Option<usize>) -> Result<()> {
        match position {
            Some(pos) => {
                if pos > self.tracks.len() {
                    return Err(anyhow!("Position {} is out of bounds", pos));
                }
                track.position = pos;
                self.tracks.insert(pos, track);
            }
            None => {
                track.position = self.tracks.len();
                self.tracks.push_back(track);
            }
        }

        self.update_positions();
        Ok(())
    }

    fn update_positions(&mut self) {
        for (index, track) in self.tracks.iter_mut().enumerate() {
            track.position = index;
        }
    }

    pub fn remove_track(&mut self, position: usize) -> Result<()> {
        if position >= self.tracks.len() {
            return Err(anyhow!("Position {} is out of bounds", position));
        }

        // If removing the current track, update position
        if let Some(current_pos) = self.current_position {
            if current_pos == position {
                if self.tracks.is_empty() {
                    self.current_position = None;
                    self.playback_state = PlaybackState::Stopped;
                } else if current_pos >= self.tracks.len() {
                    self.current_position = Some(self.tracks.len().saturating_sub(1));
                }
            } else if current_pos > position {
                self.current_position = Some(current_pos - 1);
            }
        }

        self.tracks.remove(position);
        self.update_positions();
        Ok(())
    }

    pub fn move_track(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err(anyhow!("Positions are out of bounds"));
        }

        if from == to {
            return Ok(());
        }

        let track = self.tracks.remove(from).unwrap();
        self.tracks.insert(to, track);
        self.update_positions();

        // Update current position if needed
        if let Some(current) = self.current_position {
            if current == from {
                self.current_position = Some(to);
            } else if from < to && current <= to {
                self.current_position = Some(current - 1);
            } else if from > to && current >= to {
                self.current_position = Some(current + 1);
            }
        }

        Ok(())
    }

    pub fn play(&mut self) -> Result<()> {
        if self.tracks.is_empty() {
            return Err(anyhow!("Queue is empty"));
        }

        if self.current_position.is_none() {
            self.current_position = Some(0);
        }

        self.playback_state = PlaybackState::Playing;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        if self.current_position.is_none() {
            return Err(anyhow!("No track is currently selected"));
        }

        self.playback_state = PlaybackState::Paused;
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        if self.current_position.is_none() {
            return Err(anyhow!("No track is currently selected"));
        }

        if matches!(self.playback_state, PlaybackState::Paused) {
            self.playback_state = PlaybackState::Playing;
        } else {
            self.play()?;
        }

        Ok(())
    }

    pub fn next_track(&mut self) -> Result<()> {
        if self.tracks.is_empty() {
            return Err(anyhow!("Queue is empty"));
        }

        if let Some(current) = self.current_position {
            if current < self.tracks.len() - 1 {
                self.current_position = Some(current + 1);
                Ok(())
            } else {
                Err(anyhow!("Already at last track"))
            }
        } else {
            self.current_position = Some(0);
            Ok(())
        }
    }

    pub fn previous(&mut self) -> Result<()> {
        if self.tracks.is_empty() {
            return Err(anyhow!("Queue is empty"));
        }

        if let Some(current) = self.current_position {
            if current > 0 {
                self.current_position = Some(current - 1);
                Ok(())
            } else {
                Err(anyhow!("Already at first track"))
            }
        } else {
            self.current_position = Some(0);
            Ok(())
        }
    }

    pub fn jump_to(&mut self, position: usize) -> Result<()> {
        if position >= self.tracks.len() {
            return Err(anyhow!("Position {} is out of bounds", position));
        }

        self.current_position = Some(position);
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.tracks.clear();
        self.current_position = None;
        self.playback_state = PlaybackState::Stopped;
        Ok(())
    }

    pub fn get_queue(&self) -> &VecDeque<AudioTrack> {
        &self.tracks
    }

    pub fn get_current_track(&self) -> Option<&AudioTrack> {
        self.current_position.and_then(|pos| self.tracks.get(pos))
    }

    pub fn get_status(&self) -> (PlaybackState, Option<AudioTrack>, usize) {
        (
            self.playback_state,
            self.get_current_track().cloned(),
            self.tracks.len(),
        )
    }

    pub fn display_queue(&self) -> String {
        if self.tracks.is_empty() {
            return "Queue is empty\n".to_string();
        }

        let mut output = String::new();
        output.push_str("Current Queue:\n");
        output.push_str("──────────────────────────────────────────────────\n");

        for (index, track) in self.tracks.iter().enumerate() {
            let current_marker = if self.current_position == Some(index) {
                "▶ "
            } else {
                "  "
            };

            let position = format!("{:2}.", index + 1);
            let title = track.title.as_deref()
                .unwrap_or_else(|| track.path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown"));

            let artist = track.artist.as_deref()
                .unwrap_or("Unknown Artist");

            let duration = track.duration
                .map(|d| format!(" ({:.1}s)", d))
                .unwrap_or_default();

            output.push_str(&format!(
                "{} {} - {} - {}{}\n",
                current_marker, position, title, artist, duration
            ));
        }

        output.push_str(&"─".repeat(50));
        output.push('\n');

        output
    }

    pub fn save_playlist<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        let mut file = File::create(path)
            .context(format!("Failed to create playlist file: {}", path.display()))?;

        // Write M3U header
        writeln!(file, "#EXTM3U")?;

        for track in &self.tracks {
            // Write extended info if available
            if let (Some(title), Some(artist), Some(duration)) =
                (&track.title, &track.artist, track.duration) {
                writeln!(file, "#EXTINF:{},{} - {}",
                        duration.round() as i64, artist, title)?;
            }

            // Write path (relative if possible)
            if let Some(path_str) = track.path.to_str() {
                writeln!(file, "{}", path_str)?;
            }
        }

        file.flush()?;
        Ok(())
    }

    pub fn load_playlist<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let file = File::open(path)
            .context(format!("Failed to open playlist file: {}", path.display()))?;

        let reader = BufReader::new(file);
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        self.clear()?;

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Resolve relative paths
            let track_path = if Path::new(line).is_absolute() {
                PathBuf::from(line)
            } else {
                base_dir.join(line)
            };

            // Validate and add track
            if Self::validate_audio_file(&track_path)? {
                let track = Self::extract_metadata(&track_path)?;
                self.add_track(track, None)?;
            } else {
                eprintln!("Warning: Track not found: {}", track_path.display());
            }
        }

        Ok(())
    }
}

impl Default for AudioQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_basic_operations() {
        let mut queue = AudioQueue::new();

        // Test empty queue
        assert_eq!(queue.get_queue().len(), 0);
        assert!(queue.get_current_track().is_none());

        // Add tracks
        let track1 = AudioTrack {
            path: PathBuf::from("test1.mp3"),
            title: Some("Test Song 1".to_string()),
            artist: Some("Test Artist".to_string()),
            duration: Some(120.0),
            position: 0,
        };

        let track2 = AudioTrack {
            path: PathBuf::from("test2.mp3"),
            title: Some("Test Song 2".to_string()),
            artist: Some("Test Artist".to_string()),
            duration: Some(180.0),
            position: 0,
        };

        queue.add_track(track1, None).unwrap();
        queue.add_track(track2, None).unwrap();

        assert_eq!(queue.get_queue().len(), 2);

        // Test play/pause
        queue.play().unwrap();
        assert_eq!(queue.get_status().0, PlaybackState::Playing);
        assert_eq!(queue.get_current_track().unwrap().title, Some("Test Song 1".to_string()));

        queue.pause().unwrap();
        assert_eq!(queue.get_status().0, PlaybackState::Paused);

        // Test navigation
        queue.next_track().unwrap();
        assert_eq!(queue.get_current_track().unwrap().title, Some("Test Song 2".to_string()));

        queue.previous().unwrap();
        assert_eq!(queue.get_current_track().unwrap().title, Some("Test Song 1".to_string()));
    }

    #[test]
    fn test_load_save_state() {
        let mut original = AudioQueue::new();

        let track = AudioTrack {
            path: PathBuf::from("test.mp3"),
            title: Some("Test Track".to_string()),
            artist: Some("Test Artist".to_string()),
            duration: Some(200.0),
            position: 0,
        };

        original.add_track(track, None).unwrap();
        original.play().unwrap();

        // Save state
        original.save_state("test_state.json").unwrap();

        // Load state
        let loaded = AudioQueue::load_state("test_state.json").unwrap();

        assert_eq!(loaded.get_queue().len(), 1);
        assert_eq!(loaded.get_status().0, PlaybackState::Playing);
        assert_eq!(loaded.get_current_track().unwrap().title, Some("Test Track".to_string()));

        // Cleanup
        std::fs::remove_file("test_state.json").unwrap();
    }
}
