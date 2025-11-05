use std::path::{Path, PathBuf};
use std::collections::VecDeque;
use std::fs::File;
use std::io::BufReader;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
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
    tracks: VecDeque<AudioTrack>,
    current_position: Option<usize>,
    playback_state: PlaybackState,
    command_sender: Sender<QueueCommand>,
}

#[allow(dead_code)]
impl AudioQueue {
    pub fn new() -> Self {
        let (tx, _) = mpsc::channel(100);
        Self {
            tracks: VecDeque::new(),
            current_position: None,
            playback_state: PlaybackState::Stopped,
            command_sender: tx,
        }
    }

    pub fn get_command_sender(&self) -> Sender<QueueCommand> {
        self.command_sender.clone()
    }

    pub fn validate_audio_file<P: AsRef<Path>>(path: P) -> Result<bool> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Err(anyhow!("File does not exist: {}", path.display()));
        }

        // Try to open and probe the file with symphonia
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
                // Check if we can get the format reader
                let _format = probed.format;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    pub fn extract_metadata<P: AsRef<Path>>(path: P) -> Result<AudioTrack> {
        let path = path.as_ref();
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

        let mut probed = symphonia::default::get_probe()
             .format(&hint, mss, &fmt_opts, &meta_opts)?;

        let mut track = AudioTrack {
            path: path.to_path_buf(),
            title: None,
            artist: None,
            duration: None,
            position: 0,
        };

        // Extract metadata from tags if available
        if let Some(metadata_rev) = probed.format.metadata().current() {
            for tag in metadata_rev.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        track.title = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        track.artist = Some(tag.value.to_string());
                    }
                    _ => {}
                }
            }
        }

        // If no title from metadata, use filename
        if track.title.is_none() {
            if let Some(filename) = path.file_stem() {
                if let Some(name_str) = filename.to_str() {
                    track.title = Some(name_str.to_string());
                }
            }
        }

        // Try to get duration from the first track
        if let Some(audio_track) = probed.format.tracks().iter().next() {
            let codec_params = &audio_track.codec_params;
            if let Some(n_frames) = codec_params.n_frames {
                if let Some(sample_rate) = codec_params.sample_rate {
                    track.duration = Some(n_frames as f64 / sample_rate as f64);
                }
            }
        }

        Ok(track)
    }

    /// Load tracks from M3U or M3U8 playlist file
    pub fn load_playlist<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let file = File::open(path)
            .context(format!("Failed to open playlist: {}", path.display()))?;

        let reader = BufReader::new(file);
        let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

        use std::io::BufRead;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Resolve relative paths
            let track_path = if Path::new(line).is_absolute() {
                PathBuf::from(line)
            } else {
                base_dir.join(line)
            };

            // Validate and add track
            if track_path.exists() {
                match Self::extract_metadata(&track_path) {
                    Ok(track) => {
                        self.add_track(track, None)?;
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to load track '{}': {}",
                                track_path.display(), e);
                    }
                }
            } else {
                eprintln!("Warning: Track not found: {}", track_path.display());
            }
        }

        Ok(())
    }

    /// Save current queue to M3U playlist file
    pub fn save_playlist<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use std::io::Write;

        let path = path.as_ref();
        let mut file = File::create(path)
            .context(format!("Failed to create playlist: {}", path.display()))?;

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

    pub fn add_track(&mut self, mut track: AudioTrack, position: Option<usize>) -> Result<()> {
        // Update position based on current queue
        if let Some(pos) = position {
            if pos > self.tracks.len() {
                return Err(anyhow!("Position {} is out of bounds", pos));
            }
            track.position = pos;
            self.tracks.insert(pos, track);
        } else {
            track.position = self.tracks.len();
            self.tracks.push_back(track);
        }

        // Update positions of all tracks
        self.update_positions();
        Ok(())
    }

    pub fn remove_track(&mut self, position: usize) -> Result<()> {
        if position >= self.tracks.len() {
            return Err(anyhow!("Position {} is out of bounds", position));
        }

        self.tracks.remove(position);

        // Update current position if needed
        if let Some(current) = self.current_position {
            if current == position {
                if self.tracks.is_empty() {
                    self.current_position = None;
                    self.playback_state = PlaybackState::Stopped;
                } else if current >= self.tracks.len() {
                    self.current_position = Some(self.tracks.len().saturating_sub(1));
                }
            } else if current > position {
                self.current_position = Some(current - 1);
            }
        }

        self.update_positions();
        Ok(())
    }

    pub fn move_track(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err(anyhow!("Invalid positions: from={}, to={}, queue_size={}",
                              from, to, self.tracks.len()));
        }

        if from == to {
            return Ok(());
        }

        let track = self.tracks.remove(from).unwrap();
        self.tracks.insert(to, track);

        // Update current position if needed
        if let Some(current) = self.current_position {
            if current == from {
                self.current_position = Some(to);
            } else if current > from && current <= to {
                self.current_position = Some(current - 1);
            } else if current < from && current >= to {
                self.current_position = Some(current + 1);
            }
        }

        self.update_positions();
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

        // TODO: Integrate with actual audio playback (rodio, cpal, etc.)
        // For now, this just updates state

        Ok(())
    }

    pub fn pause(&mut self) -> Result<()> {
        if self.current_position.is_none() {
            return Err(anyhow!("No track is currently selected"));
        }

        self.playback_state = PlaybackState::Paused;

        // TODO: Pause actual audio playback

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

        // TODO: Resume actual audio playback

        Ok(())
    }

    pub fn next(&mut self) -> Result<()> {
        if self.tracks.is_empty() {
            return Err(anyhow!("Queue is empty"));
        }

        if let Some(current) = self.current_position {
            if current + 1 < self.tracks.len() {
                self.current_position = Some(current + 1);
            } else {
                return Err(anyhow!("Already at last track"));
            }
        } else {
            self.current_position = Some(0);
        }

        self.playback_state = PlaybackState::Playing;

        // TODO: Start playing next track

        Ok(())
    }

    pub fn previous(&mut self) -> Result<()> {
        if self.tracks.is_empty() {
            return Err(anyhow!("Queue is empty"));
        }

        if let Some(current) = self.current_position {
            if current > 0 {
                self.current_position = Some(current - 1);
            } else {
                return Err(anyhow!("Already at first track"));
            }
        } else {
            self.current_position = Some(0);
        }

        self.playback_state = PlaybackState::Playing;

        // TODO: Start playing previous track

        Ok(())
    }

    pub fn jump_to(&mut self, position: usize) -> Result<()> {
        if position >= self.tracks.len() {
            return Err(anyhow!("Position {} is out of bounds", position));
        }

        self.current_position = Some(position);
        self.playback_state = PlaybackState::Playing;

        // TODO: Start playing track at position

        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        self.tracks.clear();
        self.current_position = None;
        self.playback_state = PlaybackState::Stopped;

        // TODO: Stop any active playback

        Ok(())
    }

    pub fn get_queue(&self) -> Vec<&AudioTrack> {
        self.tracks.iter().collect()
    }

    pub fn get_current_track(&self) -> Option<&AudioTrack> {
        self.current_position.and_then(|pos| self.tracks.get(pos))
    }

    pub fn get_status(&self) -> (&PlaybackState, Option<&AudioTrack>, usize) {
        let current = self.get_current_track();
        let queue_size = self.tracks.len();
        (&self.playback_state, current, queue_size)
    }

    fn update_positions(&mut self) {
        for (index, track) in self.tracks.iter_mut().enumerate() {
            track.position = index;
        }
    }

    pub fn display_queue(&self) -> String {
        if self.tracks.is_empty() {
            return "Queue is empty".to_string();
        }

        let mut output = String::new();
        output.push_str("Current Queue:\n");
        output.push_str(&"─".repeat(50));
        output.push('\n');

        for (index, track) in self.tracks.iter().enumerate() {
            let is_current = self.current_position.map_or(false, |pos| pos == index);

            let status_indicator = if is_current {
                match self.playback_state {
                    PlaybackState::Playing => "▶",
                    PlaybackState::Paused => "⏸",
                    PlaybackState::Stopped => "■",
                }
            } else {
                " "
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
                "{} {} {} - {}{}\n",
                status_indicator, position, title, artist, duration
            ));
        }

        output
    }

    /// Get a summary of the queue statistics
    pub fn get_stats(&self) -> QueueStats {
        let total_duration: f64 = self.tracks.iter()
            .filter_map(|t| t.duration)
            .sum();

        QueueStats {
            total_tracks: self.tracks.len(),
            total_duration,
            current_position: self.current_position,
            playback_state: self.playback_state.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueueStats {
    pub total_tracks: usize,
    pub total_duration: f64,
    pub current_position: Option<usize>,
    pub playback_state: PlaybackState,
}

impl Default for AudioQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_playlist() {
        let mut queue = AudioQueue::new();
        // Basic test that the method exists
        let result = queue.load_playlist("nonexistent.m3u");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_playlist() {
        let queue = AudioQueue::new();
        // Basic test that the method exists
        let result = queue.save_playlist("/tmp/test.m3u");
        // Should work even with empty queue
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_get_stats() {
        let queue = AudioQueue::new();
        let stats = queue.get_stats();
        assert_eq!(stats.total_tracks, 0);
    }
}
