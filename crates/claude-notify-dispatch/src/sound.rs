use crate::traits::Dispatcher;
use std::path::PathBuf;

pub struct SoundDispatcher {
    pub volume: f32,
    pub sounds_dir: PathBuf,
}

impl SoundDispatcher {
    pub fn new(volume: f32, sounds_dir: PathBuf) -> Self {
        Self { volume, sounds_dir }
    }

    fn map_title_to_sound(&self, title: &str) -> &'static str {
        let lower = title.to_lowercase();
        if lower.contains("error") || lower.contains("fail") {
            "error.mp3"
        } else if lower.contains("review") {
            "review-complete.mp3"
        } else if lower.contains("question") || lower.contains('?') {
            "question.mp3"
        } else if lower.contains("plan") {
            "plan-ready.mp3"
        } else {
            "task-complete.mp3"
        }
    }

    fn play_sound(&self, filename: &str) -> Result<(), String> {
        use rodio::{Decoder, OutputStream, Sink};
        use std::fs::File;
        use std::io::BufReader;

        let path = self.sounds_dir.join(filename);
        if !path.exists() {
            // No sound file — skip silently
            return Ok(());
        }

        let file = File::open(&path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let (_stream, stream_handle) =
            OutputStream::try_default().map_err(|e| e.to_string())?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        sink.set_volume(self.volume);
        let source = Decoder::new(reader).map_err(|e| e.to_string())?;
        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }
}

impl Dispatcher for SoundDispatcher {
    fn dispatch(&self, title: &str, _body: &str) -> Result<(), String> {
        let filename = self.map_title_to_sound(title);
        // Gracefully handle playback errors — audio is non-critical
        if let Err(e) = self.play_sound(filename) {
            tracing::warn!("Sound playback failed: {}", e);
        }
        Ok(())
    }
}
