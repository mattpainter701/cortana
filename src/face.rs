/// Continuous facial animation parameters for Cortana's terminal renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceParams {
    pub mouth_open: f32,
    pub smile: f32,
    pub brow_raise: f32,
    pub eye_squint: f32,
    pub cheek_lift: f32,
    pub head_tilt: f32,
    pub gaze_x: f32,
    pub gaze_y: f32,
    pub breathing_phase: f32,
    pub arousal: f32,
    pub valence: f32,
}

impl Default for FaceParams {
    fn default() -> Self {
        Self {
            mouth_open: 0.0,
            smile: 0.0,
            brow_raise: 0.0,
            eye_squint: 0.0,
            cheek_lift: 0.0,
            head_tilt: 0.0,
            gaze_x: 0.0,
            gaze_y: 0.0,
            breathing_phase: 0.0,
            arousal: 0.0,
            valence: 0.0,
        }
    }
}

impl FaceParams {
    /// Builds face parameters from the low-bandwidth signals emitted by the agent and speech layer.
    pub fn from_signals(audio_rms: f32, valence: f32, arousal: f32, idle_seconds: f32) -> Self {
        let audio_rms = clamp01(audio_rms);
        let valence = clamp(valence, -1.0, 1.0);
        let arousal = clamp01(arousal);
        let breathing_phase = idle_seconds * std::f32::consts::TAU / 4.0;

        Self {
            mouth_open: clamp01(audio_rms * 0.85 + arousal * 0.15),
            smile: clamp(valence * 0.8 + arousal * 0.15, -1.0, 1.0),
            brow_raise: clamp(arousal * 0.6 - valence.min(0.0) * 0.35, -1.0, 1.0),
            eye_squint: clamp01(valence.max(0.0) * 0.35 + audio_rms * 0.2),
            cheek_lift: clamp01(valence.max(0.0) * 0.35 + audio_rms * 0.28),
            head_tilt: clamp(
                valence * 4.0 + (idle_seconds * 1.7).sin() * 1.5,
                -10.0,
                10.0,
            ),
            gaze_x: clamp((idle_seconds * 0.7).sin() * 0.25, -1.0, 1.0),
            gaze_y: clamp((idle_seconds * 0.5).cos() * 0.15, -1.0, 1.0),
            breathing_phase,
            arousal,
            valence,
        }
    }

    /// Encodes the current face parameters as the daemon/renderer protocol line.
    pub fn to_protocol_line(self) -> String {
        format!(
            "FACE mouth_open={:.3} smile={:.3} brow_raise={:.3} eye_squint={:.3} cheek_lift={:.3} head_tilt={:.3} gaze_x={:.3} gaze_y={:.3} breathing_phase={:.3} arousal={:.3} valence={:.3}",
            self.mouth_open,
            self.smile,
            self.brow_raise,
            self.eye_squint,
            self.cheek_lift,
            self.head_tilt,
            self.gaze_x,
            self.gaze_y,
            self.breathing_phase,
            self.arousal,
            self.valence
        )
    }
}

/// Smoothed facial state used by the TUI renderer to avoid abrupt emotional jumps.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FaceState {
    params: FaceParams,
}

impl FaceState {
    pub fn params(self) -> FaceParams {
        self.params
    }

    pub fn tick_toward(&mut self, target: FaceParams, delta_seconds: f32) {
        let alpha = clamp01(delta_seconds * 10.0);
        self.params = self.params.lerp(target, alpha);
    }
}

trait Lerp {
    fn lerp(self, target: Self, alpha: f32) -> Self;
}

impl Lerp for FaceParams {
    fn lerp(self, target: Self, alpha: f32) -> Self {
        Self {
            mouth_open: lerp(self.mouth_open, target.mouth_open, alpha),
            smile: lerp(self.smile, target.smile, alpha),
            brow_raise: lerp(self.brow_raise, target.brow_raise, alpha),
            eye_squint: lerp(self.eye_squint, target.eye_squint, alpha),
            cheek_lift: lerp(self.cheek_lift, target.cheek_lift, alpha),
            head_tilt: lerp(self.head_tilt, target.head_tilt, alpha),
            gaze_x: lerp(self.gaze_x, target.gaze_x, alpha),
            gaze_y: lerp(self.gaze_y, target.gaze_y, alpha),
            breathing_phase: lerp(self.breathing_phase, target.breathing_phase, alpha),
            arousal: lerp(self.arousal, target.arousal, alpha),
            valence: lerp(self.valence, target.valence, alpha),
        }
    }
}

fn lerp(from: f32, to: f32, alpha: f32) -> f32 {
    from + (to - from) * alpha
}

fn clamp01(value: f32) -> f32 {
    clamp(value, 0.0, 1.0)
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    value.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_are_clamped_to_supported_ranges() {
        let params = FaceParams::from_signals(5.0, -5.0, 2.0, 0.0);

        assert_eq!(params.mouth_open, 1.0);
        assert_eq!(params.valence, -1.0);
        assert_eq!(params.arousal, 1.0);
        assert!((-10.0..=10.0).contains(&params.head_tilt));
    }

    #[test]
    fn face_state_smooths_toward_target() {
        let mut state = FaceState::default();
        let target = FaceParams::from_signals(1.0, 1.0, 1.0, 1.0);

        state.tick_toward(target, 0.016);

        assert!(state.params().mouth_open > 0.0);
        assert!(state.params().mouth_open < target.mouth_open);
    }

    #[test]
    fn protocol_line_contains_face_prefix() {
        let line = FaceParams::default().to_protocol_line();

        assert!(line.starts_with("FACE mouth_open="));
        assert!(line.contains("valence="));
    }
}
