use crate::face::FaceParams;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Rgba {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    fn color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

#[derive(Clone)]
pub struct AvatarFrame {
    width: usize,
    height: usize,
    pixels: Vec<Rgba>,
}

impl AvatarFrame {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Rgba::transparent(); width * height],
        }
    }

    fn get(&self, x: usize, y: usize) -> Rgba {
        self.pixels[y * self.width + x]
    }

    fn blend(&mut self, x: i32, y: i32, color: Rgba, alpha: f32) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let idx = y as usize * self.width + x as usize;
        let dst = self.pixels[idx];
        let a = (alpha * (color.a as f32 / 255.0)).clamp(0.0, 1.0);
        let inv = 1.0 - a;
        self.pixels[idx] = Rgba {
            r: (dst.r as f32 * inv + color.r as f32 * a) as u8,
            g: (dst.g as f32 * inv + color.g as f32 * a) as u8,
            b: (dst.b as f32 * inv + color.b as f32 * a) as u8,
            a: 255,
        };
    }

    fn glow(&mut self, x: f32, y: f32, radius: f32, color: Rgba, alpha: f32) {
        let r = radius.ceil() as i32;
        for py in (y as i32 - r)..=(y as i32 + r) {
            for px in (x as i32 - r)..=(x as i32 + r) {
                let dx = px as f32 - x;
                let dy = py as f32 - y;
                let d = (dx * dx + dy * dy).sqrt();
                if d <= radius {
                    let falloff = (1.0 - d / radius).powf(1.8);
                    self.blend(px, py, color, alpha * falloff);
                }
            }
        }
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, color: Rgba, alpha: f32) {
        let min_x = (cx - rx).floor() as i32;
        let max_x = (cx + rx).ceil() as i32;
        let min_y = (cy - ry).floor() as i32;
        let max_y = (cy + ry).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let nx = (x as f32 + 0.5 - cx) / rx;
                let ny = (y as f32 + 0.5 - cy) / ry;
                let d = nx * nx + ny * ny;
                if d <= 1.0 {
                    let edge = (1.0 - d).clamp(0.0, 1.0);
                    self.blend(x, y, color, alpha * edge.sqrt().max(0.35));
                }
            }
        }
    }

    fn ellipse_outline(
        &mut self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        thickness: f32,
        color: Rgba,
        alpha: f32,
    ) {
        let min_x = (cx - rx - thickness).floor() as i32;
        let max_x = (cx + rx + thickness).ceil() as i32;
        let min_y = (cy - ry - thickness).floor() as i32;
        let max_y = (cy + ry + thickness).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let nx = (x as f32 + 0.5 - cx) / rx;
                let ny = (y as f32 + 0.5 - cy) / ry;
                let d = (nx * nx + ny * ny).sqrt();
                let band = (1.0 - (d - 1.0).abs() / thickness).clamp(0.0, 1.0);
                if band > 0.0 {
                    self.blend(x, y, color, alpha * band);
                }
            }
        }
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgba, alpha: f32) {
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0).ceil() as i32;
        for i in 0..=steps.max(1) {
            let t = i as f32 / steps.max(1) as f32;
            let x = x0 + (x1 - x0) * t;
            let y = y0 + (y1 - y0) * t;
            self.glow(x, y, 1.1, color, alpha);
        }
    }

    fn arc(
        &mut self,
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        start: f32,
        end: f32,
        color: Rgba,
        alpha: f32,
    ) {
        let steps = ((end - start).abs() * rx.max(ry)).ceil() as i32;
        for i in 0..=steps.max(1) {
            let t = i as f32 / steps.max(1) as f32;
            let a = start + (end - start) * t;
            self.glow(cx + a.cos() * rx, cy + a.sin() * ry, 1.0, color, alpha);
        }
    }

    fn speckle(&mut self, x: f32, y: f32, color: Rgba, alpha: f32) {
        self.blend(x.round() as i32, y.round() as i32, color, alpha);
    }
}

pub struct AvatarRenderer;

static AVATAR_SOURCE: OnceLock<Option<image::RgbaImage>> = OnceLock::new();

fn fract(value: f32) -> f32 {
    value - value.floor()
}

impl AvatarRenderer {
    pub fn render_lines(
        cols: u16,
        rows: u16,
        params: &FaceParams,
        time: f32,
    ) -> Vec<Line<'static>> {
        let cols = cols.max(24).min(128) as usize;
        let cell_rows = rows.max(10).min(42) as usize;
        let mut frame = Self::render_asset_frame(cols, cell_rows * 2, time, params)
            .unwrap_or_else(|| Self::render_frame(cols, cell_rows * 2, params, time));
        Self::hologram_displace(&mut frame, time, params);
        Self::enhance_detail(&mut frame);
        if cols < 72 || cell_rows < 28 {
            Self::enhance_detail(&mut frame);
        }
        Self::hologram_grade(&mut frame, time);
        Self::lifelike_projection(&mut frame, time, params);
        Self::data_waterfall(&mut frame, time, params);
        Self::idle_expression_overlay(&mut frame, time, params);
        Self::speech_overlay(&mut frame, params);
        Self::scanlines(&mut frame, time);
        Self::to_halfblocks(&frame)
    }

    fn render_asset_frame(
        width: usize,
        height: usize,
        time: f32,
        params: &FaceParams,
    ) -> Option<AvatarFrame> {
        let image = avatar_source()?;
        load_avatar_asset(image, width, height, time, params)
    }

    fn render_frame(width: usize, height: usize, params: &FaceParams, time: f32) -> AvatarFrame {
        let mut frame = AvatarFrame::new(width, height);
        let w = width as f32;
        let h = height as f32;
        let cx = w * 0.52 + params.gaze_x * w * 0.018;
        let cy = h * 0.36 + params.gaze_y * h * 0.018;
        let scale = w.min(h * 0.86) / 68.0;
        let pulse = 0.88 + time.sin() * 0.08 + params.arousal * 0.18;

        let deep = Rgba::rgb(2, 13, 30);
        let blue = Rgba::rgb(20, 135, 220);
        let cyan = Rgba::rgb(55, 220, 255);
        let bright = Rgba::rgb(190, 250, 255);
        let face = Rgba::rgb(40, 170, 230);
        let shadow = Rgba::rgb(5, 55, 115);

        for y in 0..height as i32 {
            let fade = y as f32 / h;
            for x in 0..width as i32 {
                frame.blend(x, y, Rgba::rgb(0, (10.0 + fade * 20.0) as u8, 26), 0.35);
            }
        }

        frame.glow(cx, cy, 30.0 * scale, blue, 0.18 * pulse);
        frame.glow(cx, cy - 8.0 * scale, 19.0 * scale, cyan, 0.15 * pulse);

        // Hair/crown silhouette first, then translucent face.
        frame.ellipse(
            cx,
            cy - 8.0 * scale,
            17.0 * scale,
            24.0 * scale,
            shadow,
            0.76,
        );
        for side in [-1.0, 1.0] {
            frame.ellipse(
                cx + side * 17.2 * scale,
                cy - 1.0 * scale,
                5.6 * scale,
                21.0 * scale,
                blue,
                0.34,
            );
            frame.line(
                cx + side * 5.0 * scale,
                cy - 24.5 * scale,
                cx + side * 24.0 * scale,
                cy - 31.0 * scale,
                cyan,
                0.34,
            );
            frame.line(
                cx + side * 13.5 * scale,
                cy - 16.0 * scale,
                cx + side * 20.0 * scale,
                cy + 14.0 * scale,
                cyan,
                0.20,
            );
        }
        frame.arc(
            cx,
            cy - 18.5 * scale,
            18.5 * scale,
            7.5 * scale,
            3.30,
            6.12,
            cyan,
            0.38,
        );
        frame.arc(
            cx,
            cy - 17.0 * scale,
            13.5 * scale,
            4.5 * scale,
            3.35,
            6.05,
            blue,
            0.36,
        );
        frame.arc(
            cx,
            cy - 14.0 * scale,
            8.0 * scale,
            2.4 * scale,
            3.42,
            5.98,
            bright,
            0.18,
        );

        frame.ellipse(cx, cy - 2.0 * scale, 12.0 * scale, 18.0 * scale, deep, 0.50);
        frame.ellipse(cx, cy - 2.0 * scale, 10.4 * scale, 16.2 * scale, face, 0.34);
        frame.ellipse(
            cx,
            cy - 4.0 * scale,
            6.8 * scale,
            11.5 * scale,
            Rgba::rgb(85, 210, 245),
            0.12,
        );
        frame.ellipse(
            cx - 5.4 * scale,
            cy + 2.0 * scale,
            3.2 * scale,
            5.0 * scale,
            cyan,
            0.08,
        );
        frame.ellipse(
            cx + 5.4 * scale,
            cy + 2.0 * scale,
            3.2 * scale,
            5.0 * scale,
            cyan,
            0.08,
        );
        frame.ellipse_outline(
            cx,
            cy - 2.0 * scale,
            12.8 * scale,
            18.5 * scale,
            0.10,
            cyan,
            0.55,
        );
        frame.arc(
            cx,
            cy + 4.0 * scale,
            10.5 * scale,
            12.0 * scale,
            0.48,
            2.66,
            cyan,
            0.20,
        );
        frame.arc(
            cx,
            cy + 0.5 * scale,
            8.5 * scale,
            16.5 * scale,
            2.62,
            3.64,
            cyan,
            0.24,
        );
        frame.arc(
            cx,
            cy + 0.5 * scale,
            8.5 * scale,
            16.5 * scale,
            5.78,
            6.80,
            cyan,
            0.24,
        );

        let eye_y = cy - 7.0 * scale;
        let eye_dx = 5.8 * scale;
        let eye_open = (1.0 - params.eye_squint * 0.35).clamp(0.45, 1.0);
        for side in [-1.0, 1.0] {
            let ex = cx + side * eye_dx + params.gaze_x * 1.8 * scale;
            frame.line(
                ex - 3.2 * scale,
                eye_y - 0.8 * scale,
                ex + 3.2 * scale,
                eye_y - 0.8 * scale,
                cyan,
                0.26,
            );
            frame.line(
                ex - 2.8 * scale,
                eye_y + 1.0 * scale,
                ex + 2.8 * scale,
                eye_y + 1.0 * scale,
                cyan,
                0.18,
            );
            frame.glow(ex, eye_y, 2.2 * scale, bright, 0.55);
            frame.ellipse(ex, eye_y, 1.6 * scale, 0.9 * scale * eye_open, bright, 0.94);
            frame.ellipse(
                ex,
                eye_y,
                0.55 * scale,
                0.46 * scale * eye_open,
                Rgba::rgb(20, 80, 120),
                0.75,
            );
            frame.speckle(ex - 0.5 * scale, eye_y - 0.4 * scale, bright, 0.90);
            frame.arc(
                ex,
                eye_y - 3.0 * scale,
                4.0 * scale,
                1.5 * scale,
                3.45,
                5.95,
                cyan,
                0.36,
            );
        }

        frame.line(
            cx - 8.0 * scale,
            cy - 13.5 * scale,
            cx - 3.5 * scale,
            cy - 14.5 * scale,
            cyan,
            0.24,
        );
        frame.line(
            cx + 8.0 * scale,
            cy - 13.5 * scale,
            cx + 3.5 * scale,
            cy - 14.5 * scale,
            cyan,
            0.24,
        );
        frame.line(
            cx,
            cy - 5.0 * scale,
            cx - 1.3 * scale,
            cy + 3.8 * scale,
            cyan,
            0.35,
        );
        frame.line(
            cx,
            cy - 4.0 * scale,
            cx + 1.2 * scale,
            cy + 4.2 * scale,
            cyan,
            0.30,
        );
        frame.line(
            cx - 2.5 * scale,
            cy + 5.0 * scale,
            cx,
            cy + 7.2 * scale,
            bright,
            0.28,
        );
        frame.line(
            cx + 2.5 * scale,
            cy + 5.0 * scale,
            cx,
            cy + 7.2 * scale,
            bright,
            0.28,
        );
        frame.speckle(cx - 1.0 * scale, cy + 4.8 * scale, bright, 0.34);
        frame.speckle(cx + 1.0 * scale, cy + 4.8 * scale, bright, 0.30);

        let mouth_y = cy + 11.0 * scale;
        let open = params.mouth_open.clamp(0.0, 1.0);
        if open > 0.14 {
            frame.ellipse(
                cx,
                mouth_y,
                4.2 * scale,
                (1.2 + open * 4.4) * scale,
                Rgba::rgb(0, 8, 20),
                0.95,
            );
            frame.arc(
                cx,
                mouth_y - 0.8 * scale,
                5.0 * scale,
                2.0 * scale,
                0.15,
                3.00,
                bright,
                0.58,
            );
            frame.arc(
                cx,
                mouth_y + open * 2.0 * scale,
                4.5 * scale,
                2.0 * scale,
                0.12,
                3.02,
                cyan,
                0.50,
            );
        } else {
            frame.arc(
                cx,
                mouth_y - 1.0 * scale,
                4.2 * scale,
                1.0 * scale,
                0.05,
                3.10,
                cyan,
                0.30,
            );
            frame.arc(
                cx,
                mouth_y,
                5.2 * scale,
                (1.2 + params.smile.max(0.0) * 2.0) * scale,
                0.15,
                3.00,
                bright,
                0.56,
            );
        }

        Self::draw_face_speckles(&mut frame, cx, cy, scale, time);
        frame.line(
            cx - 4.0 * scale,
            cy + 18.0 * scale,
            cx - 5.0 * scale,
            cy + 28.0 * scale,
            blue,
            0.36,
        );
        frame.line(
            cx + 4.0 * scale,
            cy + 18.0 * scale,
            cx + 5.0 * scale,
            cy + 28.0 * scale,
            blue,
            0.36,
        );
        frame.ellipse(cx, cy + 33.0 * scale, 18.0 * scale, 5.0 * scale, blue, 0.28);
        frame.arc(
            cx,
            cy + 35.0 * scale,
            22.0 * scale,
            8.0 * scale,
            3.35,
            6.08,
            cyan,
            0.22,
        );

        Self::draw_activation_ring(&mut frame, w * 0.15, h * 0.76, 6.0 * scale, time);
        frame
    }

    fn draw_face_speckles(frame: &mut AvatarFrame, cx: f32, cy: f32, scale: f32, time: f32) {
        let cyan = Rgba::rgb(80, 225, 255);
        let blue = Rgba::rgb(20, 125, 210);
        for i in 0..95 {
            let n = i as f32;
            let sx = fract((n * 12.9898 + 3.1).sin() * 43_758.547) * 2.0 - 1.0;
            let sy = fract((n * 78.233 + 7.7).sin() * 24_634.635) * 2.0 - 1.0;
            let x = cx + sx * 10.0 * scale;
            let y = cy - 2.0 * scale + sy * 15.0 * scale;
            let mask = (sx * sx * 0.85 + sy * sy).sqrt();
            if mask < 1.0 {
                let twinkle = ((time * 2.3 + n * 0.71).sin() + 1.0) * 0.5;
                let alpha = 0.05 + twinkle * 0.16;
                frame.speckle(x, y, if i % 5 == 0 { cyan } else { blue }, alpha);
            }
        }
    }

    fn draw_activation_ring(frame: &mut AvatarFrame, cx: f32, cy: f32, r: f32, time: f32) {
        let cyan = Rgba::rgb(65, 220, 255);
        let blue = Rgba::rgb(20, 135, 220);
        frame.ellipse_outline(cx, cy, r, r, 0.14, cyan, 0.78);
        frame.ellipse_outline(
            cx,
            cy,
            r * 0.70,
            r * 0.70,
            0.16,
            blue,
            0.45 + time.sin() * 0.08,
        );
    }

    fn scanlines(frame: &mut AvatarFrame, time: f32) {
        let cyan = Rgba::rgb(80, 210, 255);
        for y in (0..frame.height as i32).step_by(6) {
            let alpha = 0.035 + ((time * 2.0 + y as f32 * 0.17).sin() + 1.0) * 0.015;
            for x in 0..frame.width as i32 {
                frame.blend(x, y, cyan, alpha);
            }
        }
    }

    fn enhance_detail(frame: &mut AvatarFrame) {
        if frame.width < 3 || frame.height < 3 {
            return;
        }
        let original = frame.pixels.clone();
        let width = frame.width;
        let height = frame.height;

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let idx = y * width + x;
                let center = original[idx];
                let mut blur = [0.0f32; 3];
                for oy in -1..=1 {
                    for ox in -1..=1 {
                        let p = original
                            [((y as isize + oy) as usize) * width + (x as isize + ox) as usize];
                        blur[0] += p.r as f32;
                        blur[1] += p.g as f32;
                        blur[2] += p.b as f32;
                    }
                }
                for channel in &mut blur {
                    *channel /= 9.0;
                }
                let sharpen = |value: u8, blurred: f32| {
                    let v = value as f32;
                    let detailed = v + (v - blurred) * 0.95;
                    let contrasted = (detailed - 128.0) * 1.18 + 128.0;
                    contrasted.clamp(0.0, 255.0) as u8
                };
                frame.pixels[idx] = Rgba::rgb(
                    sharpen(center.r, blur[0]),
                    sharpen(center.g, blur[1]),
                    sharpen(center.b, blur[2]),
                );
            }
        }
    }

    fn hologram_displace(frame: &mut AvatarFrame, time: f32, params: &FaceParams) {
        if frame.width < 4 || frame.height < 4 {
            return;
        }

        let original = frame.pixels.clone();
        let center_x = frame.width as f32 * 0.5;
        let center_y = frame.height as f32 * 0.42;
        let face_rx = frame.width as f32 * 0.24;
        let face_ry = frame.height as f32 * 0.33;
        let jitter = 0.22 + params.arousal * 0.28 + params.mouth_open * 0.18;

        for y in 0..frame.height {
            let yf = y as f32;
            let wave = (time * 1.6 + yf * 0.09).sin() * jitter;
            let glitch = if ((time * 4.0 + yf * 0.17).sin() > 0.992) && y % 9 == 0 {
                (time * 9.0 + yf).sin() * 1.0
            } else {
                0.0
            };
            for x in 0..frame.width {
                let xf = x as f32;
                let nx = (xf - center_x) / face_rx;
                let ny = (yf - center_y) / face_ry;
                let face_mask = (1.0 - (nx * nx + ny * ny).sqrt()).clamp(0.0, 1.0);
                let field_mask = 1.0 - face_mask * 0.82;
                let offset = ((wave + glitch) * field_mask).round() as isize;
                let sx = (x as isize - offset).clamp(0, frame.width as isize - 1) as usize;
                frame.pixels[y * frame.width + x] = original[y * frame.width + sx];
            }
        }
    }

    fn hologram_grade(frame: &mut AvatarFrame, time: f32) {
        let pulse = 0.97 + time.sin() * 0.025;
        for y in 0..frame.height {
            for x in 0..frame.width {
                let idx = y * frame.width + x;
                let p = frame.pixels[idx];
                let mut lum = (p.r as f32 * 0.20 + p.g as f32 * 0.55 + p.b as f32 * 0.25) / 255.0;
                lum = ((lum - 0.43) * 1.28 + 0.43).clamp(0.0, 1.0);
                let local = ((p.b as f32 - p.r as f32) / 255.0).clamp(-0.25, 0.35);
                let graded_blue = (54.0 + lum * 185.0 + local * 42.0) * pulse;
                let graded_green = (28.0 + lum * 205.0) * pulse;
                let graded_red = (1.0 + lum * 34.0) * pulse;
                let mix = 0.58;
                let keep = 1.0 - mix;
                frame.pixels[idx] = Rgba::rgb(
                    (p.r as f32 * keep + graded_red * mix).clamp(0.0, 255.0) as u8,
                    (p.g as f32 * keep + graded_green * mix).clamp(0.0, 255.0) as u8,
                    (p.b as f32 * keep + graded_blue * mix).clamp(0.0, 255.0) as u8,
                );
            }
        }
    }

    fn lifelike_projection(frame: &mut AvatarFrame, time: f32, params: &FaceParams) {
        let w = frame.width as f32;
        let h = frame.height as f32;
        let cx = w * 0.51;
        let cy = h * 0.40;
        let scale = frame.width.min(frame.height) as f32 / 70.0;
        let cyan = Rgba::rgb(138, 246, 255);
        let blue = Rgba::rgb(30, 150, 230);
        let deep = Rgba::rgb(0, 10, 28);
        let soft = Rgba::rgb(70, 205, 245);

        let breath = ((time * 0.74).sin() + 1.0) * 0.5;
        let cheek = params.cheek_lift.clamp(0.0, 1.0);
        frame.glow(
            cx,
            cy + 5.0 * scale,
            15.0 * scale,
            blue,
            0.045 + breath * 0.045 + cheek * 0.035,
        );
        frame.glow(
            cx,
            cy - 2.0 * scale,
            9.5 * scale,
            cyan,
            0.028 + breath * 0.030,
        );

        let eye_y = h * 0.405 + params.gaze_y * scale * 1.6;
        let eye_dx = w * 0.112;
        let eye_rx = w * 0.047;
        let brow_y = h * 0.355 - params.brow_raise * scale * 2.0;
        let cheek_y = h * 0.485 - cheek * scale * 1.6;
        let mouth_y = h * 0.585;

        let squint = params.eye_squint.clamp(0.0, 1.0);
        if squint > 0.035 {
            for side in [-1.0, 1.0] {
                let ex = cx + side * eye_dx + params.gaze_x * scale * 1.2;
                frame.ellipse(
                    ex,
                    eye_y - squint * scale * 0.8,
                    eye_rx * 1.05,
                    h * 0.014 + squint * scale * 1.2,
                    deep,
                    0.08 + squint * 0.30,
                );
                frame.line(
                    ex - eye_rx * 0.95,
                    eye_y - h * 0.018 - squint * scale,
                    ex + eye_rx * 0.88,
                    eye_y - h * 0.016 - squint * scale * 0.5,
                    cyan,
                    0.06 + squint * 0.18,
                );
            }
        }

        if cheek > 0.025 {
            for side in [-1.0, 1.0] {
                let cheek_x = cx + side * w * 0.110;
                frame.glow(
                    cheek_x,
                    cheek_y,
                    (w * 0.025 + cheek * w * 0.030).max(2.0),
                    soft,
                    0.055 + cheek * 0.18,
                );
                frame.arc(
                    cheek_x + side * w * 0.015,
                    cheek_y + h * 0.018,
                    w * 0.038,
                    h * 0.018,
                    if side < 0.0 { 5.45 } else { 3.98 },
                    if side < 0.0 { 6.20 } else { 4.75 },
                    cyan,
                    0.035 + cheek * 0.12,
                );
            }
        }

        if params.mouth_open <= 0.16 {
            let mouth = natural_mouth(time);
            frame.arc(
                cx,
                mouth_y + mouth * 0.8 * scale,
                (5.2 + mouth * 1.5 + cheek * 0.8) * scale,
                (0.85 + mouth * 1.7 + cheek * 0.4) * scale,
                0.18,
                2.96,
                cyan,
                0.12 + mouth * 0.24 + cheek * 0.08,
            );
            if mouth > 0.45 {
                frame.ellipse(
                    cx,
                    mouth_y + 0.55 * scale,
                    3.5 * scale,
                    0.95 * scale,
                    deep,
                    0.10,
                );
            }
        }

        let brow = natural_brow(time);
        let expressive_brow = (brow + params.brow_raise.abs() * 0.70).clamp(0.0, 1.0);
        if expressive_brow > 0.01 {
            let lift = expressive_brow * 1.2 * scale;
            frame.line(
                cx - w * 0.150,
                brow_y - lift,
                cx - w * 0.055,
                brow_y - lift * 1.25,
                cyan,
                0.10 + expressive_brow * 0.22,
            );
            frame.line(
                cx + w * 0.150,
                brow_y - lift,
                cx + w * 0.055,
                brow_y - lift * 1.25,
                cyan,
                0.10 + expressive_brow * 0.22,
            );
        }
    }

    fn data_waterfall(frame: &mut AvatarFrame, time: f32, params: &FaceParams) {
        let cyan = Rgba::rgb(95, 235, 255);
        let blue = Rgba::rgb(15, 120, 210);
        let deep = Rgba::rgb(0, 34, 80);
        let w = frame.width as f32;
        let h = frame.height as f32;
        let face_cx = w * 0.51;
        let face_cy = h * 0.40;
        let face_rx = w * 0.22;
        let face_ry = h * 0.31;
        let intensity = 0.74 + params.arousal * 0.30 + params.mouth_open * 0.18;

        for lane in 0..68 {
            let seed = lane as f32;
            let x = fract((seed * 17.371 + 0.37).sin() * 913.21) * w;
            let side_bias = ((x - face_cx).abs() / (w * 0.5)).clamp(0.0, 1.0);
            if side_bias < 0.35 && lane % 4 != 0 {
                continue;
            }

            let speed = 4.0 + fract((seed * 8.13).sin() * 177.0) * 10.0;
            let phase = time * speed + seed * 13.0;
            let segment_h = 1 + (fract(seed * 11.91) * 4.0) as i32;
            let gap = 5 + (fract(seed * 5.41) * 9.0) as i32;
            let column_shift = (phase as i32).rem_euclid(segment_h + gap);

            for y in (0..frame.height as i32).step_by(1) {
                let local = (y + column_shift).rem_euclid(segment_h + gap);
                if local >= segment_h {
                    continue;
                }

                let yf = y as f32;
                let nx = (x - face_cx) / face_rx;
                let ny = (yf - face_cy) / face_ry;
                let in_face = nx * nx + ny * ny < 1.0;
                if in_face {
                    continue;
                }

                let phase_blink = ((time * 2.7 + seed + y as f32 * 0.07).sin() + 1.0) * 0.5;
                let alpha = (0.11 + phase_blink * 0.36) * intensity * side_bias.max(0.60);
                let color = if local == 0 && phase_blink > 0.58 {
                    cyan
                } else {
                    blue
                };
                let xi = x.round() as i32;
                frame.blend(xi, y, color, alpha);
                if lane % 6 == 0 && local == 0 {
                    frame.blend(xi + 1, y, cyan, alpha * 0.58);
                }
                if lane % 9 == 0 && local == segment_h - 1 {
                    frame.blend(xi - 1, y, deep, alpha * 0.50);
                }
            }
        }

        for row in 0..9 {
            let y = ((row as f32 + 0.5) / 9.5 * h) as i32;
            let pulse = ((time * 1.8 + row as f32 * 0.8).sin() + 1.0) * 0.5;
            for x in (0..frame.width as i32).step_by(3) {
                let xf = x as f32;
                let nx = (xf - face_cx) / face_rx;
                let ny = (y as f32 - face_cy) / face_ry;
                if nx * nx + ny * ny < 1.05 {
                    continue;
                }
                frame.blend(x, y, blue, (0.04 + pulse * 0.07) * intensity);
                if pulse > 0.72 && x % 9 == 0 {
                    frame.blend(x + 1, y, cyan, (pulse - 0.72) * 0.18);
                }
            }
        }

        let band_count = 3;
        for band in 0..band_count {
            let y = fract(time * (0.035 + band as f32 * 0.018) + band as f32 * 0.31) * h;
            for x in 0..frame.width as i32 {
                frame.blend(x, y as i32, cyan, 0.020);
                frame.blend(x, y as i32 + 1, blue, 0.014);
            }
        }

        for spark in 0..72 {
            let s = spark as f32;
            let x = fract((s * 41.7 + time * 0.47).sin() * 2345.6) * w;
            let y = fract((s * 19.3 + time * 0.31).cos() * 765.4) * h;
            let side_bias = ((x - face_cx).abs() / (w * 0.5)).clamp(0.0, 1.0);
            if side_bias < 0.28 {
                continue;
            }
            let twinkle = ((time * 5.2 + s).sin() + 1.0) * 0.5;
            if twinkle > 0.72 {
                frame.glow(x, y, 1.3, cyan, (twinkle - 0.72) * 0.32);
            }
        }

        for packet in 0..44 {
            let p = packet as f32;
            let y = fract(time * (0.11 + fract(p * 1.7) * 0.08) + p * 0.127) * h;
            let side = if packet % 2 == 0 { -1.0 } else { 1.0 };
            let x0 = face_cx + side * (face_rx + 3.0 + fract(p * 2.3) * w * 0.20);
            let len = 2 + (fract(p * 5.9) * 5.0) as i32;
            let flash = ((time * 3.7 + p * 0.9).sin() + 1.0) * 0.5;
            for dx in 0..len {
                frame.blend(
                    (x0 as i32) + dx * side as i32,
                    y as i32,
                    if dx == 0 || flash > 0.72 { cyan } else { blue },
                    (0.08 + flash * 0.16) * intensity,
                );
            }
        }
    }

    fn speech_overlay(frame: &mut AvatarFrame, params: &FaceParams) {
        let open = params.mouth_open.clamp(0.0, 1.0);
        let cx = frame.width as f32 * 0.51;
        let cy = frame.height as f32 * 0.54;
        let scale = frame.width.min(frame.height) as f32 / 70.0;
        let cyan = Rgba::rgb(125, 240, 255);
        let soft = Rgba::rgb(70, 205, 245);
        if open > 0.16 {
            frame.ellipse(
                cx,
                cy,
                (5.2 + open * 1.2) * scale,
                (1.8 + open * 6.4) * scale,
                Rgba::rgb(0, 8, 24),
                0.42 + open * 0.28,
            );
            frame.arc(
                cx,
                cy - 1.0 * scale,
                (5.9 + open * 0.9) * scale,
                (1.9 + open * 0.6) * scale,
                0.1,
                3.05,
                cyan,
                0.48,
            );
            frame.arc(
                cx,
                cy + open * 3.2 * scale,
                (4.9 + open * 0.8) * scale,
                (1.8 + open * 0.8) * scale,
                0.1,
                3.05,
                cyan,
                0.44,
            );
            let cheek = params.cheek_lift.clamp(0.0, 1.0);
            if cheek > 0.02 {
                frame.glow(
                    cx - 7.5 * scale,
                    cy - 1.2 * scale,
                    (3.2 + cheek * 4.2) * scale,
                    soft,
                    0.08 + cheek * 0.24,
                );
                frame.glow(
                    cx + 7.5 * scale,
                    cy - 1.2 * scale,
                    (3.2 + cheek * 4.2) * scale,
                    soft,
                    0.08 + cheek * 0.24,
                );
                frame.line(
                    cx - 7.0 * scale,
                    cy + 1.2 * scale,
                    cx - 3.8 * scale,
                    cy + open * 2.0 * scale,
                    cyan,
                    0.08 + cheek * 0.18,
                );
                frame.line(
                    cx + 7.0 * scale,
                    cy + 1.2 * scale,
                    cx + 3.8 * scale,
                    cy + open * 2.0 * scale,
                    cyan,
                    0.08 + cheek * 0.18,
                );
            }
        } else {
            frame.arc(cx, cy, 5.4 * scale, 1.4 * scale, 0.15, 3.0, cyan, 0.13);
        }
    }

    fn idle_expression_overlay(frame: &mut AvatarFrame, time: f32, params: &FaceParams) {
        let speaking = params.mouth_open > 0.16 || params.arousal > 0.45;
        let cx = frame.width as f32 * 0.50;
        let cy = frame.height as f32 * 0.39;
        let scale = frame.width.min(frame.height) as f32 / 70.0;
        let cyan = Rgba::rgb(130, 245, 255);
        let soft = Rgba::rgb(70, 205, 245);
        let shadow = Rgba::rgb(0, 20, 50);

        let eye_y = frame.height as f32 * 0.405;
        let eye_dx = frame.width as f32 * 0.112;
        let eye_rx = frame.width as f32 * 0.047;
        let eye_ry = frame.height as f32 * 0.020;

        let blink = blink_amount(time);
        for side in [-1.0, 1.0] {
            let ex = cx + side * eye_dx;
            if blink > 0.02 {
                frame.ellipse(ex, eye_y, eye_rx, eye_ry * 2.7, shadow, blink * 0.86);
                frame.ellipse(
                    ex,
                    eye_y + eye_ry * 0.22,
                    eye_rx * 0.86,
                    eye_ry * 1.40,
                    Rgba::rgb(8, 80, 120),
                    blink * 0.42,
                );
                frame.line(
                    ex - eye_rx * 1.02,
                    eye_y + eye_ry * 0.12,
                    ex + eye_rx * 1.02,
                    eye_y + eye_ry * 0.12,
                    cyan,
                    0.22 * blink,
                );
            }
        }

        if !speaking {
            let micro_smile = natural_mouth(time);
            if micro_smile > 0.12 {
                frame.arc(
                    cx,
                    cy + 11.5 * scale,
                    5.8 * scale,
                    (1.0 + micro_smile * 1.3) * scale,
                    0.16,
                    2.98,
                    cyan,
                    0.07 + micro_smile * 0.14,
                );
            }

            let brow = ((time * 0.29 + 1.0).sin() + 1.0) * 0.5;
            let brow_y = frame.height as f32 * 0.355;
            if brow > 0.60 || params.brow_raise.abs() > 0.05 {
                let lift = ((brow - 0.60).max(0.0) * 1.8 + params.brow_raise.abs() * 1.4) * scale;
                let alpha = 0.10 + (brow - 0.60).max(0.0) * 0.24 + params.brow_raise.abs() * 0.16;
                frame.line(
                    cx - frame.width as f32 * 0.145,
                    brow_y - lift,
                    cx - frame.width as f32 * 0.055,
                    brow_y - lift * 1.25,
                    soft,
                    alpha,
                );
                frame.line(
                    cx + frame.width as f32 * 0.145,
                    brow_y - lift,
                    cx + frame.width as f32 * 0.055,
                    brow_y - lift * 1.25,
                    soft,
                    alpha,
                );
            }

            let cheek = (((time * 0.23 + 2.4).sin() + 1.0) * 0.5).max(params.cheek_lift * 1.8);
            if cheek > 0.55 {
                frame.glow(
                    cx - 7.8 * scale,
                    cy + 3.5 * scale,
                    (3.6 + params.cheek_lift * 3.2) * scale,
                    soft,
                    (cheek - 0.55) * 0.12 + params.cheek_lift * 0.10,
                );
                frame.glow(
                    cx + 7.8 * scale,
                    cy + 3.5 * scale,
                    (3.6 + params.cheek_lift * 3.2) * scale,
                    soft,
                    (cheek - 0.55) * 0.12 + params.cheek_lift * 0.10,
                );
            }
        }
    }

    fn to_halfblocks(frame: &AvatarFrame) -> Vec<Line<'static>> {
        let mut lines = Vec::with_capacity(frame.height / 2);
        for y in (0..frame.height).step_by(2) {
            let mut spans = Vec::with_capacity(frame.width);
            for x in 0..frame.width {
                let top = frame.get(x, y);
                let bottom = frame.get(x, (y + 1).min(frame.height - 1));
                spans.push(Span::styled(
                    "▀",
                    Style::default().fg(top.color()).bg(bottom.color()),
                ));
            }
            lines.push(Line::from(spans));
        }
        lines
    }
}

fn blink_amount(time: f32) -> f32 {
    let cadence = 3.8;
    let phase = time.rem_euclid(cadence);
    let primary = blink_curve(phase, 0.46);
    let double_phase = (phase - 0.43).max(0.0);
    let double = if (0.43..0.76).contains(&phase) {
        blink_curve(double_phase, 0.32) * 0.58
    } else {
        0.0
    };
    let secondary_phase = (time + 1.85).rem_euclid(9.2);
    primary
        .max(double)
        .max(blink_curve(secondary_phase, 0.30) * 0.52)
}

fn blink_curve(phase: f32, width: f32) -> f32 {
    if phase > width {
        return 0.0;
    }
    let t = phase / width;
    (std::f32::consts::PI * t).sin().powf(0.7)
}

fn idle_gaze(time: f32) -> (f32, f32) {
    let period = 2.6;
    let step = (time / period).floor();
    let local = (time / period).fract();
    let hold = smoothstep(0.0, 0.20, local) * (1.0 - smoothstep(0.78, 1.0, local));
    let target_x = (fract((step + 1.0) * 12.9898).sin() * 2.0 - 1.0) * 1.15;
    let target_y = (fract((step + 3.0) * 78.233).cos() * 2.0 - 1.0) * 0.42;
    let drift_x = (time * 0.22).sin() * 0.34 + (time * 0.09).sin() * 0.26;
    let drift_y = (time * 0.19).cos() * 0.18;
    (drift_x + target_x * hold, drift_y + target_y * hold)
}

fn natural_mouth(time: f32) -> f32 {
    let slow = ((time * 0.31).sin() * 0.5 + 0.5).powf(2.0);
    let small = ((time * 1.05 + 1.4).sin() * 0.5 + 0.5).powf(4.0);
    (slow * 0.55 + small * 0.18).clamp(0.0, 1.0)
}

fn natural_brow(time: f32) -> f32 {
    let period = 5.8;
    let local = (time / period + 0.18).fract();
    let rise = smoothstep(0.08, 0.22, local) * (1.0 - smoothstep(0.48, 0.68, local));
    rise * (0.55 + ((time * 0.37).sin() + 1.0) * 0.22)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn avatar_asset_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CORTANA_AVATAR") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    for path in [
        PathBuf::from("assets/cortana-avatar.png"),
        PathBuf::from("assets/cortana-avatar.jpg"),
        PathBuf::from("assets/cortana-avatar.jpeg"),
    ] {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

fn avatar_source() -> Option<&'static image::RgbaImage> {
    AVATAR_SOURCE
        .get_or_init(|| {
            let path = avatar_asset_path()?;
            image::open(path).ok().map(|image| image.to_rgba8())
        })
        .as_ref()
}

fn load_avatar_asset(
    image: &image::RgbaImage,
    width: usize,
    height: usize,
    time: f32,
    params: &FaceParams,
) -> Option<AvatarFrame> {
    let (src_w, src_h) = image.dimensions();
    if src_w == 0 || src_h == 0 {
        return None;
    }

    let dst_aspect = width as f32 / height as f32;
    let src_aspect = src_w as f32 / src_h as f32;
    let (base_crop_x, base_crop_y, base_crop_w, base_crop_h) = if src_aspect > dst_aspect {
        let crop_w = (src_h as f32 * dst_aspect) as u32;
        ((src_w - crop_w) / 2, 0, crop_w, src_h)
    } else {
        let crop_h = (src_w as f32 / dst_aspect) as u32;
        let centered = (src_h - crop_h) / 2;
        let face_bias = (crop_h as f32 * 0.14) as u32;
        (0, centered.saturating_sub(face_bias), src_w, crop_h)
    };
    let width_ratio = width as f32 / 88.0;
    let height_ratio = height as f32 / 84.0;
    let compact = (1.0 - width_ratio.min(height_ratio)).clamp(0.0, 1.0);
    let detail_zoom = 1.0 + compact * 0.72;
    let crop_w = ((base_crop_w as f32 / detail_zoom).round() as u32).clamp(1, src_w);
    let crop_h = ((base_crop_h as f32 / detail_zoom).round() as u32).clamp(1, src_h);
    let focal_x = base_crop_x as f32 + base_crop_w as f32 * 0.50;
    let focal_y = base_crop_y as f32 + base_crop_h as f32 * (0.47 - compact * 0.07);
    let crop_x = (focal_x - crop_w as f32 * 0.5)
        .round()
        .clamp(0.0, (src_w - crop_w) as f32) as u32;
    let crop_y = (focal_y - crop_h as f32 * 0.5)
        .round()
        .clamp(0.0, (src_h - crop_h) as f32) as u32;

    let mut frame = AvatarFrame::new(width, height);
    let breathing_zoom = 1.0 + (time * 0.68).sin() * 0.024 + params.arousal * 0.006;
    let speech_stability = (1.0 - params.mouth_open * 0.28).clamp(0.76, 1.0);
    let (gaze_x, gaze_y) = idle_gaze(time);
    let sway_x = ((time * 0.36).sin() * 0.020 + (time * 0.17).sin() * 0.010 + gaze_x * 0.006)
        * crop_w as f32
        * speech_stability;
    let sway_y = ((time * 0.29).sin() * 0.013 + (time * 0.11).cos() * 0.005 + gaze_y * 0.005)
        * crop_h as f32
        * speech_stability;
    let center_x = crop_x as f32 + crop_w as f32 * 0.5 + sway_x;
    let center_y = crop_y as f32 + crop_h as f32 * 0.5 + sway_y;
    let nod = (time * 0.74).sin() * 0.006 * speech_stability;
    let shear =
        ((time * 0.24).sin() * 0.010 + params.head_tilt.to_radians() * 0.020) * speech_stability;

    for y in 0..height {
        for x in 0..width {
            let mut nx = (x as f32 + 0.5) / width as f32 - 0.5;
            let mut ny = (y as f32 + 0.5) / height as f32 - 0.5;
            let face_mask =
                (1.0 - ((nx / 0.30).powi(2) + ((ny + 0.08) / 0.38).powi(2)).sqrt()).clamp(0.0, 1.0);
            nx -= shear * ny * face_mask;
            ny += nod * face_mask;
            let sx = center_x + nx * crop_w as f32 / breathing_zoom - 0.5;
            let sy = center_y + ny * crop_h as f32 / breathing_zoom - 0.5;
            frame.pixels[y * width + x] = sample_bilinear(&image, sx, sy);
        }
    }
    Some(frame)
}

fn sample_bilinear(image: &image::RgbaImage, x: f32, y: f32) -> Rgba {
    let max_x = (image.width() - 1) as f32;
    let max_y = (image.height() - 1) as f32;
    let x = x.clamp(0.0, max_x);
    let y = y.clamp(0.0, max_y);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(image.width() - 1);
    let y1 = (y0 + 1).min(image.height() - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let p00 = image.get_pixel(x0, y0).0;
    let p10 = image.get_pixel(x1, y0).0;
    let p01 = image.get_pixel(x0, y1).0;
    let p11 = image.get_pixel(x1, y1).0;
    let mix = |i: usize| {
        let top = p00[i] as f32 * (1.0 - tx) + p10[i] as f32 * tx;
        let bottom = p01[i] as f32 * (1.0 - tx) + p11[i] as f32 * tx;
        (top * (1.0 - ty) + bottom * ty).clamp(0.0, 255.0) as u8
    };
    Rgba {
        r: mix(0),
        g: mix(1),
        b: mix(2),
        a: mix(3),
    }
}
