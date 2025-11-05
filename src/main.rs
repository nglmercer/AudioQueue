use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::Duration;

mod audio_queue;
mod audio_emitter;
mod queue_processor;

use audio_queue::{AudioQueue, AudioQueueState, QueueCommand};
use audio_emitter::{AudioEmitter, EmitterCommand};
use queue_processor::QueueProcessor;

#[derive(Parser)]
#[command(name = "audioqueue")]
#[command(about = "A command-line audio queue manager")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add an audio file to the queue
    Add {
        /// Path to the audio file
        file: PathBuf,
        #[arg(short, long)]
        /// Position in queue (optional, adds to end by default)
        position: Option<usize>,
    },
    /// List all files in the queue
    List,
    /// Remove a file from the queue
    Remove {
        /// Position of the file in queue
        position: usize,
    },
    /// Move a file to a new position in queue
    Move {
        /// Current position of the file
        from: usize,
        /// New position for the file
        to: usize,
    },
    /// Play the audio queue
    Play,
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Skip to next track
    Next,
    /// Skip to previous track
    Previous,
    /// Jump to specific position in queue
    Jump {
        /// Position to jump to
        position: usize,
    },
    /// Clear the entire queue
    Clear,
    /// Show current playback status
    Status,
    /// Set volume (0.0 to 1.0)
    Volume {
        /// Volume level (0.0 to 1.0)
        level: f32,
    },
    /// Start the daemon/service that manages playback
    Start,
}

struct AudioQueueManager {
    queue: Arc<Mutex<AudioQueue>>,
    emitter: Arc<Mutex<AudioEmitter>>,
    queue_sender: mpsc::Sender<QueueCommand>,
    emitter_sender: mpsc::Sender<EmitterCommand>,
    _processor_handle: tokio::task::JoinHandle<()>, // Keep processor alive
    state_file: PathBuf, // Persistent state file
}

impl AudioQueueManager {
    fn get_state_file_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push("audioqueue_state.json");
        path
    }

    async fn save_state(&self) -> Result<()> {
        let queue = self.queue.lock().await;
        let state = AudioQueueState {
            tracks: queue.get_queue().iter().cloned().collect(),
            current_position: if queue.get_current_track().is_some() {
                Some(queue.get_queue().len() - 1)
            } else {
                None
            },
            playback_state: queue.get_status().0,
        };
        drop(queue);

        let content = serde_json::to_string_pretty(&state)
            .context("Failed to serialize queue state")?;

        std::fs::write(&self.state_file, content)
            .context("Failed to write state file")?;

        Ok(())
    }

    async fn new() -> Result<Self> {
        let state_file = Self::get_state_file_path();

        // Try to load existing queue state
        let queue = if state_file.exists() {
            match std::fs::read_to_string(&state_file) {
                Ok(content) => {
                    match serde_json::from_str::<AudioQueueState>(&content) {
                        Ok(state) => {
                            let mut queue = AudioQueue::new();
                            for track in state.tracks {
                                queue.add_track(track, None).unwrap_or_default();
                            }
                            if let Some(pos) = state.current_position {
                                queue.jump_to(pos).unwrap_or_default();
                            }
                            queue.playback_state = state.playback_state;
                            Arc::new(Mutex::new(queue))
                        }
                        Err(_) => {
                            eprintln!("Warning: Invalid state file, creating new queue");
                            Arc::new(Mutex::new(AudioQueue::new()))
                        }
                    }
                }
                Err(_) => {
                    eprintln!("Warning: Could not read state file, creating new queue");
                    Arc::new(Mutex::new(AudioQueue::new()))
                }
            }
        } else {
            Arc::new(Mutex::new(AudioQueue::new()))
        };

        let emitter = Arc::new(Mutex::new(AudioEmitter::new()?));

        // Create channels for queue commands
        let (queue_tx, queue_rx) = mpsc::channel(100);
        let queue_sender = queue_tx;

        // Get emitter command sender
        let emitter_sender = emitter.lock().await.get_command_sender();

        // Set up command sender for the queue
        {
            let mut queue_guard = queue.lock().await;
            queue_guard.set_command_sender(queue_sender.clone());
        }

        // Start the queue processor in a separate task
        let queue_clone = queue.clone();
        let emitter_sender_clone = emitter_sender.clone();
        let processor_handle = tokio::spawn(async move {
            let mut processor = QueueProcessor::new(queue_clone, emitter_sender_clone, queue_rx);
            if let Err(e) = processor.run().await {
                eprintln!("Queue processor error: {}", e);
            }
        });

        // Give the processor a moment to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(Self {
            queue,
            emitter,
            queue_sender,
            emitter_sender,
            _processor_handle: processor_handle,
            state_file,
        })
    }

    async fn handle_add(&self, file: PathBuf, position: Option<usize>) -> Result<()> {
        // Validate audio file
        if !AudioQueue::validate_audio_file(&file)? {
            return Err(anyhow::anyhow!("File is not a valid audio file: {}", file.display()));
        }

        // Extract metadata
        let track = AudioQueue::extract_metadata(&file)?;

        // Add to queue
        self.queue_sender.send(QueueCommand::Add(track, position)).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Added {} to queue", file.display());

        // Show updated queue
        self.handle_list().await?;
        Ok(())
    }

    async fn handle_list(&self) -> Result<()> {
        let queue = self.queue.lock().await;
        println!("{}", queue.display_queue());
        Ok(())
    }

    async fn handle_remove(&self, position: usize) -> Result<()> {
        self.queue_sender.send(QueueCommand::Remove(position)).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Removed item at position {}", position);

        // Show updated queue
        self.handle_list().await?;
        Ok(())
    }

    async fn handle_move(&self, from: usize, to: usize) -> Result<()> {
        self.queue_sender.send(QueueCommand::Move(from, to)).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Moved item from position {} to {}", from, to);

        // Show updated queue
        self.handle_list().await?;
        Ok(())
    }

    async fn handle_play(&self) -> Result<()> {
        // Send command to queue processor
        self.queue_sender.send(QueueCommand::Play).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Also handle audio playback directly
        let queue = self.queue.lock().await;
        if let Some(track) = queue.get_current_track() {
            let file_path = track.path.to_string_lossy().to_string();
            drop(queue);

            // Use local emitter for audio playback
            let mut emitter = AudioEmitter::new()?;
            if let Err(e) = emitter.load_file(&file_path) {
                eprintln!("Error loading file {}: {}", file_path, e);
            } else if let Err(e) = emitter.play() {
                eprintln!("Error playing file {}: {}", file_path, e);
            } else {
                println!("Now playing: {}", file_path);
            }
        } else {
            println!("No current track to play");
        }

        println!("Starting playback");
        Ok(())
    }

    async fn handle_pause(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::Pause).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        if let Err(e) = self.emitter.lock().await.pause() {
            eprintln!("Error pausing: {}", e);
        } else {
            println!("Paused playback");
        }
        Ok(())
    }

    async fn handle_resume(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::Resume).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        if let Err(e) = self.emitter.lock().await.resume() {
            eprintln!("Error resuming: {}", e);
        } else {
            println!("Resumed playback");
        }
        Ok(())
    }

    async fn handle_next(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::Next).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Skipped to next track");
        self.handle_status().await?;
        Ok(())
    }

    async fn handle_previous(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::Previous).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Went to previous track");
        self.handle_status().await?;
        Ok(())
    }

    async fn handle_jump(&self, position: usize) -> Result<()> {
        self.queue_sender.send(QueueCommand::Jump(position)).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        println!("Jumped to position {}", position);
        self.handle_status().await?;
        Ok(())
    }

    async fn handle_clear(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::Clear).await?;

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        // Stop any playing audio
        if let Err(e) = self.emitter.lock().await.stop() {
            eprintln!("Error stopping: {}", e);
        }

        println!("Cleared queue");
        Ok(())
    }

    async fn handle_status(&self) -> Result<()> {
        self.queue_sender.send(QueueCommand::GetStatus).await?;

        // Give queue processor time to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Also get direct status from queue for immediate response
        let (state, current_track, queue_size) = {
            let queue = self.queue.lock().await;
            queue.get_status()
        };

        println!("=== Queue Status ===");
        println!("State: {:?}", state);
        println!("Queue size: {}", queue_size);

        if let Some(track) = current_track {
            println!("Current track: {} - {} ({:.1}s)",
                track.title.as_deref().unwrap_or("Unknown"),
                track.artist.as_deref().unwrap_or("Unknown Artist"),
                track.duration.unwrap_or(0.0));
        } else {
            println!("No current track");
        }
        println!("======================");

        Ok(())
    }

    async fn handle_volume(&self, level: f32) -> Result<()> {
        let clamped_level = level.clamp(0.0, 1.0);

        // Wait for the processor to handle the command
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Save state after modification
        self.save_state().await?;

        if let Err(e) = self.emitter.lock().await.set_volume(clamped_level) {
            eprintln!("Error setting volume: {}", e);
        } else {
            println!("Volume set to {:.2}", clamped_level);
        }
        Ok(())
    }

    async fn handle_start(&self) -> Result<()> {
        println!("Starting AudioQueue daemon...");
        println!("AudioQueue daemon running. Press Ctrl+C to stop.");

        // Keep the main task alive
        tokio::signal::ctrl_c().await?;
        println!("\nShutting down AudioQueue daemon...");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let manager = AudioQueueManager::new().await?;

    match cli.command {
        Commands::Add { file, position } => {
            manager.handle_add(file, position).await?;
        }
        Commands::List => {
            manager.handle_list().await?;
        }
        Commands::Remove { position } => {
            manager.handle_remove(position).await?;
        }
        Commands::Move { from, to } => {
            manager.handle_move(from, to).await?;
        }
        Commands::Play => {
            manager.handle_play().await?;
        }
        Commands::Pause => {
            manager.handle_pause().await?;
        }
        Commands::Resume => {
            manager.handle_resume().await?;
        }
        Commands::Next => {
            manager.handle_next().await?;
        }
        Commands::Previous => {
            manager.handle_previous().await?;
        }
        Commands::Jump { position } => {
            manager.handle_jump(position).await?;
        }
        Commands::Clear => {
            manager.handle_clear().await?;
        }
        Commands::Status => {
            manager.handle_status().await?;
        }
        Commands::Volume { level } => {
            manager.handle_volume(level).await?;
        }
        Commands::Start => {
            manager.handle_start().await?;
        }
    }

    Ok(())
}
