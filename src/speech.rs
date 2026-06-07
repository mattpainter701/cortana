use std::sync::mpsc;

/// Event emitted during speech playback for lip-sync animation.
#[derive(Debug, Clone)]
pub enum SpeechEvent {
    #[allow(dead_code)]
    Started(String),
    #[allow(dead_code)]
    Amplitude { time_ms: u64, rms: f32 },
    Ended,
}

/// Available TTS providers.
pub enum TtsProvider {
    System,
    #[allow(dead_code)]
    ElevenLabs,
    Silent,
}

impl TtsProvider {
    pub fn from_str(s: &str) -> Self {
        match s {
            "elevenlabs" | "eleven_labs" => Self::ElevenLabs,
            "silent" | "none" => Self::Silent,
            _ => Self::System,
        }
    }
}

/// Text-to-speech engine.
pub struct TtsEngine {
    provider: TtsProvider,
}

impl TtsEngine {
    pub fn new(provider: TtsProvider) -> Self {
        Self { provider }
    }

    /// Speak text, returning a channel of speech events for lip sync.
    /// Returns None if speech is disabled or unavailable.
    pub fn speak(&self, text: &str) -> Option<mpsc::Receiver<SpeechEvent>> {
        match &self.provider {
            TtsProvider::Silent => None,
            TtsProvider::System => Self::speak_system(text),
            TtsProvider::ElevenLabs => {
                // ElevenLabs requires API key — fall back to system for now
                Self::speak_system(text)
            }
        }
    }

    fn speak_system(text: &str) -> Option<mpsc::Receiver<SpeechEvent>> {
        let (tx, rx) = mpsc::channel();
        let text = text.to_string();

        std::thread::spawn(move || {
            tx.send(SpeechEvent::Started(text.clone())).ok();

            // Try system TTS via the `tts` crate
            match tts::Tts::default() {
                Ok(mut tts_engine) => {
                    // Estimate amplitude events based on text characteristics
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let total_duration_ms = (words.len() as u64 * 350).max(500); // rough estimate

                    for i in 0..20 {
                        let time_ms = (total_duration_ms * i) / 20;
                        // Simulate amplitude from text properties
                        let progress = i as f32 / 20.0;
                        let rms = if words.is_empty() {
                            0.1
                        } else {
                            // Vary amplitude based on position in phrase
                            let base = 0.2;
                            let emphasis = (progress * std::f32::consts::PI * 2.0).sin().abs() * 0.4;
                            (base + emphasis).min(1.0)
                        };
                        tx.send(SpeechEvent::Amplitude { time_ms, rms }).ok();
                    }

                    // Actually speak
                    if let Err(e) = tts_engine.speak(&text, false) {
                        eprintln!("TTS error: {e}");
                    }

                    // Brief wait for speech to finish
                    std::thread::sleep(std::time::Duration::from_millis(total_duration_ms + 200));
                }
                Err(e) => {
                    eprintln!("TTS init error: {e}");
                    // Emit a few amplitude events for visual feedback even without audio
                    for i in 0..8 {
                        let time_ms = i * 50;
                        let rms = 0.15;
                        tx.send(SpeechEvent::Amplitude { time_ms, rms }).ok();
                    }
                }
            }

            tx.send(SpeechEvent::Ended).ok();
        });

        Some(rx)
    }

    /// Quick speak without waiting for events (fire-and-forget).
    pub fn speak_bg(&self, text: &str) {
        let text = text.to_string();
        std::thread::spawn(move || {
            if let Ok(mut tts_engine) = tts::Tts::default() {
                tts_engine.speak(&text, false).ok();
            }
        });
    }
}
