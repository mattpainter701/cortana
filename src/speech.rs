use std::sync::mpsc;

/// Event emitted during speech playback for lip-sync animation.
#[derive(Debug, Clone)]
pub enum SpeechEvent {
    #[allow(dead_code)]
    Started(String),
    #[allow(dead_code)]
    Amplitude {
        time_ms: u64,
        rms: f32,
    },
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

            let total_duration_ms = estimate_speech_duration_ms(&text);
            let audio_text = text.clone();
            let audio_thread = std::thread::spawn(move || match tts::Tts::default() {
                Ok(mut tts_engine) => {
                    if let Err(e) = tts_engine.speak(&audio_text, false) {
                        eprintln!("TTS error: {e}");
                    }
                }
                Err(e) => eprintln!("TTS init error: {e}"),
            });

            let started = std::time::Instant::now();
            loop {
                let time_ms = started.elapsed().as_millis() as u64;
                if time_ms > total_duration_ms {
                    break;
                }
                let rms = estimated_rms(&text, time_ms, total_duration_ms);
                tx.send(SpeechEvent::Amplitude { time_ms, rms }).ok();
                std::thread::sleep(std::time::Duration::from_millis(45));
            }

            let _ = audio_thread.join();
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

fn estimate_speech_duration_ms(text: &str) -> u64 {
    let words = text.split_whitespace().count() as u64;
    let punctuation_pause = text
        .chars()
        .filter(|c| matches!(c, '.' | ',' | ';' | ':' | '?' | '!'))
        .count() as u64
        * 90;
    (words * 285 + punctuation_pause).clamp(650, 12_000)
}

fn estimated_rms(text: &str, time_ms: u64, total_duration_ms: u64) -> f32 {
    let progress = time_ms as f32 / total_duration_ms.max(1) as f32;
    let envelope = (std::f32::consts::PI * progress).sin().max(0.18);
    let syllable = (time_ms as f32 / 82.0 * std::f32::consts::TAU).sin().abs();
    let secondary = (time_ms as f32 / 137.0 * std::f32::consts::TAU).sin().abs();
    let emphasis = if emphasized_region(text, progress) {
        0.18
    } else {
        0.0
    };
    (0.12 + envelope * (syllable * 0.48 + secondary * 0.16) + emphasis).clamp(0.08, 0.92)
}

fn emphasized_region(text: &str, progress: f32) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return false;
    }
    let idx = ((words.len() as f32 - 1.0) * progress).round() as usize;
    let word = words[idx.min(words.len() - 1)];
    word.ends_with('!')
        || word.ends_with('?')
        || word.chars().filter(|c| c.is_uppercase()).count() > 1
}
