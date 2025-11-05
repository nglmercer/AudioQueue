pub mod audio_queue;
pub mod audio_emitter;
pub mod queue_processor;

// Re-exportar tipos públicos para uso externo
pub use audio_queue::{
    AudioQueue, AudioTrack, PlaybackState, QueueCommand
};
pub use audio_emitter::AudioEmitter;
pub use queue_processor::QueueProcessor;

// Versión y metadatos del crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");
