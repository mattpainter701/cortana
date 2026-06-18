use crate::face::FaceParams;
use crate::speech::SpeechEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceMode {
    Idle,
    Attentive,
    Thinking,
    Speaking,
}

#[derive(Debug, Clone)]
pub struct ExpressionController {
    mode: PresenceMode,
    speech_rms: f32,
    speech_age_seconds: f32,
    speech_elapsed_ms: u64,
    speech_valence: f32,
    response_valence: f32,
    response_age_seconds: f32,
}

impl Default for ExpressionController {
    fn default() -> Self {
        Self {
            mode: PresenceMode::Idle,
            speech_rms: 0.0,
            speech_age_seconds: 99.0,
            speech_elapsed_ms: 0,
            speech_valence: 0.0,
            response_valence: 0.0,
            response_age_seconds: 99.0,
        }
    }
}

impl ExpressionController {
    pub fn observe_speech(&mut self, event: &SpeechEvent) {
        match event {
            SpeechEvent::Started(text) => {
                self.mode = PresenceMode::Speaking;
                self.speech_rms = 0.12;
                self.speech_age_seconds = 0.0;
                self.speech_elapsed_ms = 0;
                self.speech_valence = estimate_valence(text);
            }
            SpeechEvent::Amplitude { time_ms, rms } => {
                self.mode = PresenceMode::Speaking;
                self.speech_rms = clamp01(*rms);
                self.speech_age_seconds = 0.0;
                self.speech_elapsed_ms = *time_ms;
            }
            SpeechEvent::Ended => {
                self.mode = PresenceMode::Idle;
                self.speech_rms = 0.0;
                self.speech_age_seconds = 99.0;
                self.speech_elapsed_ms = 0;
            }
        }
    }

    pub fn observe_response(&mut self, text: &str) {
        self.response_valence = estimate_valence(text);
        self.response_age_seconds = 0.0;
    }

    pub fn set_mode(&mut self, mode: PresenceMode) {
        if self.mode != PresenceMode::Speaking {
            self.mode = mode;
        }
    }

    pub fn tick(&mut self, delta_seconds: f32, idle_seconds: f32) -> FaceParams {
        self.speech_age_seconds += delta_seconds;
        self.response_age_seconds += delta_seconds;

        match self.mode {
            PresenceMode::Speaking => self.speaking_params(idle_seconds),
            PresenceMode::Thinking => self.thinking_params(idle_seconds),
            PresenceMode::Attentive => self.attentive_params(idle_seconds),
            PresenceMode::Idle => self.idle_params(idle_seconds),
        }
    }

    fn speaking_params(&self, idle_seconds: f32) -> FaceParams {
        let fresh = (1.0 - self.speech_age_seconds / 0.18).clamp(0.0, 1.0);
        let syllable = ((self.speech_elapsed_ms as f32 / 95.0) * std::f32::consts::TAU)
            .sin()
            .abs();
        let rms = (self.speech_rms * fresh + syllable * 0.16).clamp(0.0, 1.0);
        let emphasis = ((rms - 0.38) / 0.42).clamp(0.0, 1.0);
        let mut params = FaceParams::from_signals(rms, self.speech_valence, 0.66, idle_seconds);

        params.mouth_open = (rms * 1.12 + syllable * 0.18).clamp(0.10, 1.0);
        params.cheek_lift = (rms * 0.72 + self.speech_valence.max(0.0) * 0.38).clamp(0.0, 1.0);
        params.brow_raise = (params.brow_raise + emphasis * 0.34).clamp(-1.0, 1.0);
        params.eye_squint = (params.eye_squint + emphasis * 0.22).clamp(0.0, 1.0);
        params.head_tilt = (params.head_tilt + (self.speech_elapsed_ms as f32 * 0.008).sin() * 1.6)
            .clamp(-10.0, 10.0);
        params
    }

    fn thinking_params(&self, idle_seconds: f32) -> FaceParams {
        let mut params = FaceParams::from_signals(0.03, -0.12, 0.45, idle_seconds);
        params.brow_raise = -0.30;
        params.eye_squint = 0.32;
        params.cheek_lift = 0.06;
        params.gaze_y -= 0.08;
        params
    }

    fn attentive_params(&self, idle_seconds: f32) -> FaceParams {
        let mut params = FaceParams::from_signals(0.0, 0.18, 0.30, idle_seconds);
        params.cheek_lift = 0.20;
        params.eye_squint = 0.10;
        params
    }

    fn idle_params(&self, idle_seconds: f32) -> FaceParams {
        let recent_response = (1.0 - self.response_age_seconds / 3.2).clamp(0.0, 1.0);
        let valence = self.response_valence * recent_response;
        let mut params = FaceParams::from_signals(0.0, valence, 0.10, idle_seconds);
        params.mouth_open = 0.0;
        let ambient = ((idle_seconds * 0.38).sin() * 0.5 + 0.5) * 0.06;
        params.cheek_lift = (0.08 + ambient + valence.max(0.0) * 0.28).clamp(0.0, 1.0);
        params
    }
}

fn estimate_valence(text: &str) -> f32 {
    let lower = text.to_lowercase();
    let positive = [
        "good", "great", "done", "fixed", "ready", "yes", "nice", "closer",
    ];
    let negative = [
        "error", "failed", "sorry", "couldn't", "cannot", "bad", "broken",
    ];
    let mut score: f32 = 0.0;
    for word in positive {
        if lower.contains(word) {
            score += 0.18;
        }
    }
    for word in negative {
        if lower.contains(word) {
            score -= 0.22;
        }
    }
    score.clamp(-0.7, 0.7)
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_amplitude_opens_mouth_and_lifts_cheeks() {
        let mut controller = ExpressionController::default();
        controller.observe_speech(&SpeechEvent::Started("Great, fixed.".into()));
        controller.observe_speech(&SpeechEvent::Amplitude {
            time_ms: 120,
            rms: 0.7,
        });

        let params = controller.tick(0.016, 1.0);

        assert!(params.mouth_open > 0.5);
        assert!(params.cheek_lift > 0.2);
    }

    #[test]
    fn thinking_has_focused_expression() {
        let mut controller = ExpressionController::default();
        controller.set_mode(PresenceMode::Thinking);

        let params = controller.tick(0.016, 1.0);

        assert!(params.eye_squint > 0.1);
        assert!(params.brow_raise < 0.0);
    }
}
