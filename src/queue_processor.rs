use std::sync::Arc;
use anyhow::{Result, anyhow};
use tokio::sync::{Mutex, mpsc::Receiver};
use tokio::time::{interval, Duration};
use crate::audio_queue::{AudioQueue, QueueCommand};
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
        let mut check_interval = interval(Duration::from_millis(500));

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
                    if let Err(e) = self.check_track_finished(&self.emitter_sender).await {
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

                // Get current track and play it
                if let Some(track) = queue.get_current_track() {
                    let file_path = track.path.to_string_lossy().to_string();
                    drop(queue);

                    // Stop current playback and start new one
                    emitter_sender.send(EmitterCommand::Stop).await?;
                    emitter_sender.send(EmitterCommand::Play(file_path.clone())).await?;
                    println!("Playing: {}", file_path);
                } else {
                    println!("Queue is empty");
                }
            }
            QueueCommand::Pause => {
                let mut queue = self.queue.lock().await;
                queue.pause()?;

                emitter_sender.send(EmitterCommand::Pause).await?;
            }
            QueueCommand::Resume => {
                let mut queue = self.queue.lock().await;
                queue.resume()?;

                emitter_sender.send(EmitterCommand::Resume).await?;
            }
            QueueCommand::Next => {
                let mut queue = self.queue.lock().await;

                if queue.next().is_ok() {
                    if let Some(track) = queue.get_current_track() {
                        let file_path = track.path.to_string_lossy().to_string();
                        drop(queue);

                        emitter_sender.send(EmitterCommand::Stop).await?;
                        emitter_sender.send(EmitterCommand::Play(file_path.clone())).await?;
                        println!("Next track: {}", file_path);
                    }
                } else {
                    println!("Already at last track or queue is empty");
                }
            }
            QueueCommand::Previous => {
                let mut queue = self.queue.lock().await;

                if queue.previous().is_ok() {
                    if let Some(track) = queue.get_current_track() {
                        let file_path = track.path.to_string_lossy().to_string();
                        drop(queue);

                        emitter_sender.send(EmitterCommand::Stop).await?;
                        emitter_sender.send(EmitterCommand::Play(file_path.clone())).await?;
                        println!("Previous track: {}", file_path);
                    }
                } else {
                    println!("Already at first track or queue is empty");
                }
            }
            QueueCommand::Jump(position) => {
                let mut queue = self.queue.lock().await;

                if queue.jump_to(position).is_ok() {
                    if let Some(track) = queue.get_current_track() {
                        let file_path = track.path.to_string_lossy().to_string();
                        drop(queue);

                        emitter_sender.send(EmitterCommand::Stop).await?;
                        emitter_sender.send(EmitterCommand::Play(file_path.clone())).await?;
                        println!("Jumped to track {}: {}", position, file_path);
                    }
                } else {
                    println!("Invalid position or queue is empty");
                }
            }
            QueueCommand::Clear => {
                let mut queue = self.queue.lock().await;
                queue.clear()?;

                emitter_sender.send(EmitterCommand::Stop).await?;
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

    async fn check_track_finished(&self, _emitter_sender: &tokio::sync::mpsc::Sender<EmitterCommand>) -> Result<()> {
        // For now, we can't easily check if track finished without access to emitter
        // This is a limitation of the current design. Auto-advance functionality
        // will need to be implemented differently or we need to restore emitter access.
        // For now, we'll skip the auto-advance feature.

        Ok(())
    }
}
