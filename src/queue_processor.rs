use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, mpsc::Receiver};
use tokio::time::{interval, Duration};
use crate::audio_queue::{AudioQueue, QueueCommand, PlaybackState};
use crate::audio_emitter::EmitterCommand;

pub struct QueueProcessor {
    queue: Arc<Mutex<AudioQueue>>,
    emitter_sender: tokio::sync::mpsc::Sender<EmitterCommand>,
    command_receiver: Option<Receiver<QueueCommand>>,
}

impl QueueProcessor {
    pub fn new(
        queue: Arc<Mutex<AudioQueue>>,
        emitter_sender: tokio::sync::mpsc::Sender<EmitterCommand>,
        command_receiver: Receiver<QueueCommand>,
    ) -> Self {
        Self {
            queue,
            emitter_sender,
            command_receiver: Some(command_receiver),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut receiver = self.command_receiver.take()
            .ok_or_else(|| anyhow!("Command receiver already taken"))?;

        // Interval to check if current track finished
        let mut check_interval = interval(Duration::from_millis(1000));

        // Track last known state to detect changes
        let mut last_track_count = 0;
        let mut was_playing = false;

        loop {
            tokio::select! {
                // Handle incoming commands
                Some(command) = receiver.recv() => {
                    if let Err(e) = self.handle_command(command, &self.emitter_sender).await {
                        eprintln!("Error handling command: {}", e);
                    }
                },
                // Check if current track finished
                _ = check_interval.tick() => {
                    if let Err(e) = self.check_track_finished(&self.emitter_sender, &mut was_playing).await {
                        eprintln!("Error checking track status: {}", e);
                    }
                }
            }
        }
    }

    async fn handle_command(&self, command: QueueCommand, emitter_sender: &tokio::sync::mpsc::Sender<EmitterCommand>) -> Result<()> {
        match command {
            QueueCommand::Add(track, position) => {
                let mut queue = self.queue.lock().await;
                queue.add_track(track, position)?;
                println!("Track added to queue");
            }
            QueueCommand::Remove(position) => {
                let mut queue = self.queue.lock().await;
                queue.remove_track(position)?;
                println!("Track removed from queue");
            }
            QueueCommand::Move(from, to) => {
                let mut queue = self.queue.lock().await;
                queue.move_track(from, to)?;
                println!("Track moved in queue");
            }
            QueueCommand::Play => {
                let mut queue = self.queue.lock().await;

                // If no current track, select first one
                if queue.get_current_track().is_none() && !queue.get_queue().is_empty() {
                    // Use jump_to to set the current position
                    let _ = queue.jump_to(0);
                }

                queue.play()?;

                // Get the current track and send it to the emitter
                if let Some(track) = queue.get_current_track() {
                    let file_path = track.path.to_string_lossy().to_string();

                    // Send play command to emitter
                    if let Err(e) = emitter_sender.send(EmitterCommand::Stop).await {
                        eprintln!("Error sending stop command: {}", e);
                    }

                    // Small delay before sending play
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    if let Err(e) = emitter_sender.send(EmitterCommand::Play(file_path)).await {
                        eprintln!("Error sending play command: {}", e);
                    }
                }

                println!("Queue playback started");
            }
            QueueCommand::Pause => {
                let mut queue = self.queue.lock().await;
                queue.pause()?;
                println!("Queue paused");
            }
            QueueCommand::Resume => {
                let mut queue = self.queue.lock().await;
                queue.resume()?;
                println!("Queue resumed");
            }
            QueueCommand::Next => {
                let mut queue = self.queue.lock().await;

                if queue.next().is_ok() {
                    println!("Next track");
                } else {
                    println!("Already at last track or queue is empty");
                }
            }
            QueueCommand::Previous => {
                let mut queue = self.queue.lock().await;

                if queue.previous().is_ok() {
                    println!("Previous track");
                } else {
                    println!("Already at first track or queue is empty");
                }
            }
            QueueCommand::Jump(position) => {
                let mut queue = self.queue.lock().await;

                if queue.jump_to(position).is_ok() {
                    println!("Jumped to position {}", position);
                } else {
                    println!("Invalid position or queue is empty");
                }
            }
            QueueCommand::Clear => {
                let mut queue = self.queue.lock().await;
                queue.clear()?;
                println!("Queue cleared");
            }
            QueueCommand::GetStatus => {
                let queue = self.queue.lock().await;
                let (state, current_track, queue_size) = queue.get_status();

                println!("=== Queue Status ===");
                println!("State: {:?}", state);
                println!("Queue size: {}", queue_size);

                if let Some(track) = current_track {
                    println!("Current: {} - {}",
                        track.title.as_deref().unwrap_or("Unknown"),
                        track.artist.as_deref().unwrap_or("Unknown Artist"));
                    println!("File: {}", track.path.display());
                } else {
                    println!("No current track");
                }
                println!("======================");
            }
        }

        Ok(())
    }

    async fn check_track_finished(
        &self,
        emitter_sender: &tokio::sync::mpsc::Sender<EmitterCommand>,
        was_playing: &mut bool
    ) -> Result<()> {
        // Get current queue state
        let queue = self.queue.lock().await;
        let (playback_state, current_track, queue_size) = queue.get_status();
        drop(queue);

        // Only process if we have tracks
        if queue_size == 0 {
            *was_playing = false;
            return Ok(());
        }

        // If we're currently playing, mark it
        if playback_state == PlaybackState::Playing && current_track.is_some() {
            *was_playing = true;
            return Ok(());
        }

        // If we were playing but now we're not, try to advance
        if *was_playing && (playback_state != PlaybackState::Playing || current_track.is_none()) {
            *was_playing = false;

            println!("🎵 Track finished, advancing to next...");

            // Stop current playback
            if let Err(e) = emitter_sender.send(EmitterCommand::Stop).await {
                eprintln!("Error sending stop command: {}", e);
            }

            // Small delay before advancing
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Try to advance to next track
            if let Err(e) = self.handle_command(QueueCommand::Next, emitter_sender).await {
                eprintln!("Error advancing to next track: {}", e);
            } else {
                // After successfully advancing, try to play the next track
                tokio::time::sleep(Duration::from_millis(50)).await;
                if let Err(e) = self.handle_command(QueueCommand::Play, emitter_sender).await {
                    eprintln!("Error playing next track: {}", e);
                }
            }
        }

        Ok(())
    }
}
