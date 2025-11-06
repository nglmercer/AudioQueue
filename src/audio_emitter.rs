use std::sync::{Arc, Mutex};
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use anyhow::{Result, anyhow};
use rodio::{OutputStream, OutputStreamHandle, Sink, Decoder};
use std::io::BufReader;
use tokio::sync::mpsc::{self, Sender, Receiver};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum EmitterCommand {
    Play(String),
    Pause,
    Resume,
    Stop,
    Volume(f32),
    Seek(f64),
    GetStatus,
}

#[derive(Debug, Clone)]
pub enum EmitterState {
    Stopped,
    Playing,
    Paused,
}

pub struct AudioEmitter {
    state: EmitterState,
    volume: f32,
    current_file: Option<String>,
    position: f64,
    #[allow(dead_code)]
    duration: Option<f64>,
    #[allow(dead_code)]
    command_sender: Sender<EmitterCommand>,
    #[allow(dead_code)]
    command_receiver: Arc<Mutex<Option<Receiver<EmitterCommand>>>>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Arc<Mutex<Sink>>>,
    _stream: Option<OutputStream>, // Keep the stream alive
}

// Manual implementations of Send and Sync for AudioEmitter
// Safety: All mutable access to AudioEmitter is protected by Mutex
// and the only non-Send+Sync field (_stream: OutputStream) is
// never accessed concurrently
unsafe impl Send for AudioEmitter {}
unsafe impl Sync for AudioEmitter {}

impl AudioEmitter {
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel(100);

        // Initialize audio stream - use default device
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| anyhow!("Failed to create audio output stream: {}", e))?;

        println!("Audio stream initialized successfully");

        Ok(Self {
            state: EmitterState::Stopped,
            volume: 1.0,
            current_file: None,
            position: 0.0,
            duration: None,
            command_sender: tx,
            command_receiver: Arc::new(Mutex::new(Some(rx))),
            stream_handle: Some(stream_handle),
            sink: None,
            _stream: Some(stream), // Keep stream alive
        })
    }

    pub fn get_command_sender(&self) -> Sender<EmitterCommand> {
        self.command_sender.clone()
    }

    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        // Check if file exists
        if !path.as_ref().exists() {
            return Err(anyhow!("File does not exist: {}", path_str));
        }

        // Stop any existing playback first
        if let Some(old_sink) = &self.sink {
            old_sink.lock().unwrap().stop();
            old_sink.lock().unwrap().empty();
        }

        // Open the file with rodio
        let file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Failed to open file {}: {}", path_str, e))?;

        match Decoder::new(BufReader::new(file)) {
            Ok(source) => {
                if let Some(stream_handle) = &self.stream_handle {
                    // Create new sink
                    let sink = Sink::try_new(stream_handle)
                        .map_err(|e| anyhow!("Failed to create audio sink: {}", e))?;

                    // Ensure sink is stopped before appending
                    sink.stop();
                    sink.set_volume(self.volume);
                    sink.append(source);

                    // Stop old sink if exists
                    if let Some(old_sink) = &self.sink {
                        old_sink.lock().unwrap().stop();
                    }

                    self.sink = Some(Arc::new(Mutex::new(sink)));
                    self.current_file = Some(path_str);
                    self.state = EmitterState::Stopped;
                    self.position = 0.0;

                    println!("Successfully loaded audio file: {}", self.current_file.as_ref().unwrap());
                    Ok(())
                } else {
                    Err(anyhow!("No audio stream handle available"))
                }
            }
            Err(e) => {
                Err(anyhow!("Failed to decode audio file {}: {}", path_str, e))
            }
        }
    }

    pub fn play(&mut self) -> Result<()> {
        if let Some(sink) = &self.sink {
            let sink_guard = sink.lock().unwrap();

            if sink_guard.is_paused() {
                drop(sink_guard);
                self.sink.as_ref().unwrap().lock().unwrap().play();
                self.state = EmitterState::Playing;
                println!("Resumed playback");
            } else if sink_guard.empty() {
                drop(sink_guard);
                return Err(anyhow!("No audio loaded or audio finished"));
            } else {
                drop(sink_guard);
                // Ensure sink is playing and not stopped
                let sink = self.sink.as_ref().unwrap().lock().unwrap();
                if sink.is_paused() {
                    sink.play();
                }
                drop(sink);
                self.state = EmitterState::Playing;
                println!("Started playback");
            }
        } else {
            return Err(anyhow!("No audio sink available"));
        }

        Ok(())
    }

    /// Play audio and wait for it to complete
    /// This keeps the process alive until the audio finishes playing
    #[allow(dead_code)]
    pub fn play_and_wait(&mut self) -> Result<()> {
        self.play()?;

        if let Some(sink) = &self.sink {
            // Clone the sink to avoid holding the lock across await
            let sink_clone = Arc::clone(sink);

            // Use a simple blocking approach with timeout
            let start_time = Instant::now();
            let timeout = Duration::from_secs(600); // 10 minutes timeout

            loop {
                {
                    let sink_guard = sink_clone.lock().unwrap();
                    if sink_guard.empty() {
                        println!("Audio playback completed");
                        self.state = EmitterState::Stopped;
                        return Ok(());
                    }

                    // Check timeout
                    if start_time.elapsed() > timeout {
                        println!("Playback timeout reached");
                        sink_guard.stop();
                        self.state = EmitterState::Stopped;
                        return Err(anyhow!("Playback timeout after 10 minutes"));
                    }
                }

                // Sleep briefly to avoid busy-waiting
                thread::sleep(Duration::from_millis(100));
            }
        } else {
            Err(anyhow!("No audio sink available"))
        }
    }



    pub fn pause(&mut self) -> Result<()> {
        if let Some(sink) = &self.sink {
            sink.lock().unwrap().pause();
            self.state = EmitterState::Paused;
            println!("Paused playback");
        } else {
            return Err(anyhow!("No audio sink available"));
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(sink) = &self.sink {
            let sink_guard = sink.lock().unwrap();
            sink_guard.stop();
            sink_guard.empty(); // Clear the sink to prevent old audio from playing
            drop(sink_guard);
            self.state = EmitterState::Stopped;
            self.position = 0.0;
            println!("Stopped playback");
        } else {
            return Err(anyhow!("No audio sink available"));
        }
        Ok(())
    }

    pub fn resume(&mut self) -> Result<()> {
        self.play()
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume.clamp(0.0, 1.0);

        if let Some(sink) = &self.sink {
            sink.lock().unwrap().set_volume(self.volume);
            println!("Volume set to {:.2}", self.volume);
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn seek(&mut self, _position: f64) -> Result<()> {
        // Note: Seeking with rodio is limited. For full seeking support,
        // we would need a more sophisticated approach with custom audio rendering
        println!("Warning: Seeking not supported with current rodio implementation");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_status(&self) -> (&EmitterState, Option<&String>, f32, f64, Option<f64>) {
        (&self.state, self.current_file.as_ref(), self.volume, self.position, self.duration)
    }

    #[allow(dead_code)]
    pub fn is_finished(&self) -> bool {
        if let Some(sink) = &self.sink {
            let sink_guard = sink.lock().unwrap();
            sink_guard.empty()
        } else {
            true
        }
    }

    #[allow(dead_code)]
    pub async fn process_commands(&mut self) -> Result<()> {
        let mut receiver = {
            let mut guard = self.command_receiver.lock().unwrap();
            guard.take().ok_or_else(|| anyhow!("Command receiver already taken"))?
        };

        while let Some(command) = receiver.recv().await {
            match command {
                EmitterCommand::Play(file_path) => {
                    // Load and play the file
                    if let Err(e) = self.load_file(&file_path) {
                        eprintln!("Error loading file {}: {}", file_path, e);
                    } else {
                        // Small delay to ensure audio is loaded
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if let Err(e) = self.play() {
                            eprintln!("Error playing file {}: {}", file_path, e);
                        }
                    }
                }
                EmitterCommand::Pause => {
                    if let Err(e) = self.pause() {
                        eprintln!("Error pausing: {}", e);
                    }
                }
                EmitterCommand::Resume => {
                    if let Err(e) = self.resume() {
                        eprintln!("Error resuming: {}", e);
                    }
                }
                EmitterCommand::Stop => {
                    if let Err(e) = self.stop() {
                        eprintln!("Error stopping: {}", e);
                    }
                }
                EmitterCommand::Volume(volume) => {
                    if let Err(e) = self.set_volume(volume) {
                        eprintln!("Error setting volume: {}", e);
                    }
                }
                EmitterCommand::Seek(_) => {
                    if let Err(e) = self.seek(0.0) {
                        eprintln!("Error seeking: {}", e);
                    }
                }
                EmitterCommand::GetStatus => {
                    let (state, file, volume, position, duration) = self.get_status();
                    println!("Emitter Status:");
                    println!("  State: {:?}", state);
                    println!("  File: {:?}", file);
                    println!("  Volume: {:.2}", volume);
                    println!("  Position: {:.2}s", position);
                    if let Some(dur) = duration {
                        println!("  Duration: {:.2}s", dur);
                    }
                    println!("  Finished: {}", self.is_finished());
                }
            }
        }

        Ok(())
    }
}
