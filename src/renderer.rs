use crate::face::FaceParams;
use ratatui::style::Color;
use ratatui::widgets::canvas::Points;
use std::cell::Cell;
use std::f64::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererBackend {
    Canvas,
    Kitty,
    #[allow(dead_code)]
    Sixel,
}

impl RendererBackend {
    pub fn detect() -> Self {
        if std::env::var("KITTY_WINDOW_ID").is_ok() { Self::Kitty } else { Self::Canvas }
    }
    pub fn from_str(s: &str) -> Self {
        match s { "kitty" => Self::Kitty, "sixel" => Self::Sixel, _ => Self::Canvas }
    }
}

#[derive(Clone, Copy)]
struct Particle {
    x: f64, y: f64, vx: f64, vy: f64, life: f64, max_life: f64,
}

pub struct FaceRenderer {
    backend: RendererBackend,
    frame_count: Cell<u64>,
    particles: Cell<Vec<Particle>>,
}

impl FaceRenderer {
    pub fn new(backend: RendererBackend) -> Self {
        Self { backend, frame_count: Cell::new(0), particles: Cell::new(Vec::new()) }
    }

    pub fn backend(&self) -> RendererBackend { self.backend }

    /// Paint the face at canvas scale. Canvas bounds should be [-50, 50] x [-60, 60].
    pub fn paint_on_canvas(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        params: &FaceParams,
    ) {
        let fc = self.frame_count.get() + 1;
        self.frame_count.set(fc);

        let mut particles = self.particles.take();
        self.update_particles(&mut particles, params, fc);

        let tilt = (params.head_tilt as f64 * 0.7).to_radians();
        let breathe = (params.breathing_phase as f64).sin() * 2.0;
        let arousal = params.arousal as f64;
        let valence = params.valence as f64;
        let gaze_x = params.gaze_x as f64 * 4.0;
        let gaze_y = params.gaze_y as f64 * 3.0;

        // Background
        self.draw_bg_grid(ctx, fc);
        self.draw_aura_ring(ctx, tilt, breathe, arousal, fc);

        // Particles behind face
        self.draw_particles(ctx, &particles);
        self.particles.set(particles);

        // Face
        self.draw_head_oval(ctx, tilt, breathe, fc);
        self.draw_eyes(ctx, tilt, breathe, gaze_x, gaze_y, params, fc);
        self.draw_brows(ctx, tilt, breathe, params);
        self.draw_nose_line(ctx, tilt, breathe);
        self.draw_mouth(ctx, tilt, breathe, params, valence);
        self.draw_collar_v(ctx, tilt, breathe, fc);
        self.draw_crown_arcs(ctx, tilt, breathe, arousal, fc);

        // Scanline
        self.draw_scanline(ctx, fc);
    }

    pub fn render(&self, params: &FaceParams) -> String {
        format!("FACE frame={} mouth={:.2} smile={:.2}", self.frame_count.get(), params.mouth_open, params.smile)
    }

    // ── helpers ──

    fn tf(&self, x: f64, y: f64, tilt: f64, breathe: f64) -> (f64, f64) {
        let (s, c) = (tilt.sin(), tilt.cos());
        (x * c - y * s, x * s + y * c + breathe)
    }

    fn dot(&self, ctx: &mut ratatui::widgets::canvas::Context, x: f64, y: f64, color: Color, a: f64) {
        if a < 0.015 { return; }
        ctx.draw(&Points { coords: &[(x, y)], color: dim(color, a) });
    }

    fn line_to(&self, ctx: &mut ratatui::widgets::canvas::Context, x1: f64, y1: f64, x2: f64, y2: f64, color: Color, a: f64) {
        if a < 0.015 { return; }
        let n = ((x2 - x1).abs().max((y2 - y1).abs()) * 1.8) as usize + 2;
        let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            pts.push((x1 + (x2 - x1) * t, y1 + (y2 - y1) * t));
        }
        ctx.draw(&Points { coords: &pts, color: dim(color, a) });
    }

    /// Draw many dots by plotting them as a dense line — single call to ctx.draw per batch
    fn dots(&self, ctx: &mut ratatui::widgets::canvas::Context, pts: &[(f64, f64)], color: Color, a: f64) {
        if a < 0.015 || pts.is_empty() { return; }
        ctx.draw(&Points { coords: pts, color: dim(color, a) });
    }

    // ── background grid ──

    fn draw_bg_grid(&self, ctx: &mut ratatui::widgets::canvas::Context, fc: u64) {
        let mut pts = Vec::new();
        for i in -4..=4i32 {
            for j in -5..=5i32 {
                let cx = i as f64 * 12.0;
                let cy = j as f64 * 12.0;
                for k in 0..6 {
                    let a = TAU * k as f64 / 6.0 - TAU / 12.0;
                    pts.push((cx + a.cos() * 7.0, cy + a.sin() * 7.0));
                }
            }
        }
        self.dots(ctx, &pts, Color::Rgb(8, 25, 60), 0.25);
    }

    // ── aura ──

    fn draw_aura_ring(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, arousal: f64, fc: u64) {
        let pulse = 1.0 + (fc as f64 * 0.04).sin() * 0.06 + arousal * 0.10;
        for (r, color, a) in [
            (38.0 * pulse, Color::Rgb(10, 80, 180), 0.06),
            (34.0 * pulse, Color::Rgb(30, 140, 220), 0.12),
            (31.0 * pulse, Color::Rgb(60, 190, 255), 0.18),
        ] {
            let n = 60;
            let mut pts = Vec::with_capacity(n);
            for i in 0..n {
                let ang = TAU * i as f64 / n as f64;
                pts.push(self.tf(ang.cos() * r, ang.sin() * r * 0.72, tilt, breathe));
            }
            self.dots(ctx, &pts, color, a);
        }
    }

    // ── particles ──

    fn update_particles(&self, particles: &mut Vec<Particle>, params: &FaceParams, _fc: u64) {
        let rate = if params.arousal > 0.4 { 2 } else { 1 };
        for _ in 0..rate {
            if particles.len() < 40 {
                let ang = rand_f64() * TAU;
                let d = 28.0 + rand_f64() * 22.0;
                particles.push(Particle {
                    x: ang.cos() * d, y: ang.sin() * d * 0.7 - 8.0,
                    vx: (0.5 - rand_f64()) * 0.5, vy: -0.4 - rand_f64() * 0.6,
                    life: 0.0, max_life: 18.0 + rand_f64() * 35.0,
                });
            }
        }
        let mut i = 0;
        while i < particles.len() {
            let p = &mut particles[i];
            p.x += p.vx; p.y += p.vy; p.life += 1.0;
            if p.life > p.max_life * 0.7 { p.vx *= 0.93; p.vy += 0.03; }
            if p.life >= p.max_life { particles.swap_remove(i); } else { i += 1; }
        }
    }

    fn draw_particles(&self, ctx: &mut ratatui::widgets::canvas::Context, particles: &[Particle]) {
        let mut bright = Vec::new();
        let mut mid = Vec::new();
        let mut dim_pts = Vec::new();
        for p in particles {
            let frac = p.life / p.max_life;
            let a = if frac > 0.7 { (1.0 - frac) / 0.3 * 0.6 } else { 0.6 };
            if frac < 0.3 { bright.push((p.x, p.y)); }
            else if frac < 0.6 { mid.push((p.x, p.y)); }
            else { dim_pts.push((p.x, p.y)); }
        }
        self.dots(ctx, &bright, Color::Rgb(80, 200, 240), 0.5);
        self.dots(ctx, &mid, Color::Rgb(30, 120, 200), 0.35);
        self.dots(ctx, &dim_pts, Color::Rgb(10, 50, 140), 0.2);
    }

    // ── head oval ──

    fn draw_head_oval(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, fc: u64) {
        let shimmer = (fc as f64 * 0.03).sin() * 0.05;
        let outline = Color::Rgb(70, 195, 250);

        // Oval: rx=24, ry=32, centered at (0, -2)
        let (rx, ry) = (24.0, 32.0);
        let n = 80;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let a = TAU * i as f64 / n as f64;
            pts.push(self.tf(a.cos() * rx, -2.0 + a.sin() * ry, tilt, breathe));
        }
        self.dots(ctx, &pts, outline, 0.65 + shimmer);

        // Inner glow oval
        let mut inner = Vec::with_capacity(40);
        for i in 0..40 {
            let a = TAU * i as f64 / 40.0;
            inner.push(self.tf(a.cos() * (rx - 3.0), -1.0 + a.sin() * (ry - 3.0), tilt, breathe));
        }
        self.dots(ctx, &inner, Color::Rgb(15, 55, 130), 0.12);

        // Subtle fill
        let mut fill = Vec::new();
        for ix in -10..=10i32 {
            for iy in -14..=14i32 {
                let x = ix as f64 * 2.2;
                let y = iy as f64 * 2.1;
                let nr = ((x / rx).powi(2) + (y / ry).powi(2)).sqrt();
                if nr < 0.92 {
                    fill.push(self.tf(x, -2.0 + y, tilt, breathe));
                }
            }
        }
        self.dots(ctx, &fill, Color::Rgb(8, 30, 80), 0.03);
    }

    // ── eyes ──

    fn draw_eyes(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, gaze_x: f64, gaze_y: f64, params: &FaceParams, fc: u64) {
        let squint = params.eye_squint as f64;
        let blink = (fc as f64 * 0.33).cos().max(0.0).powi(4); // sharper blink
        let eye_c = Color::Rgb(80, 210, 255);
        let eye_y = -8.0;

        for sx in [-10.5, 10.5f64].iter() {
            let ew = 6.5;
            let eh = (3.0 + squint * 1.5) * (1.0 - blink * 0.9).max(0.2);

            // Eye ring
            let n = 28;
            let mut ring = Vec::with_capacity(n);
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                let ex = sx + gaze_x * 0.6 + a.sin() * ew;
                let ey = eye_y + gaze_y * 0.6 + a.cos() * eh;
                // Taper ends
                if ey > eye_y - eh * 0.85 && ey < eye_y + eh * 0.85 {
                    ring.push(self.tf(ex, ey, tilt, breathe));
                }
            }
            self.dots(ctx, &ring, eye_c, 0.55);

            // Iris (filled circle)
            let mut iris = Vec::new();
            for ix in -2..=2i32 {
                for iy in -2..=2i32 {
                    let dx = ix as f64; let dy = iy as f64;
                    if dx * dx + dy * dy <= 3.5 {
                        iris.push(self.tf(sx + gaze_x * 0.9 + dx * 1.1, eye_y + gaze_y * 0.9 + dy * 0.8, tilt, breathe));
                    }
                }
            }
            self.dots(ctx, &iris, Color::Rgb(30, 60, 110), 0.55);

            // Pupil
            let mut pupil = Vec::new();
            for ix in -1..=1i32 {
                for iy in -1..=1i32 {
                    if ix * ix + iy * iy <= 1 {
                        pupil.push(self.tf(sx + gaze_x * 1.1 + ix as f64 * 0.8, eye_y + gaze_y * 1.1 + iy as f64 * 0.6, tilt, breathe));
                    }
                }
            }
            self.dots(ctx, &pupil, Color::Rgb(0, 5, 25), 0.65);

            // Highlight dot
            let (hx, hy) = self.tf(sx + gaze_x * 0.4 + 1.8, eye_y + gaze_y * 0.3 - eh * 0.4, tilt, breathe);
            self.dot(ctx, hx, hy, Color::Rgb(160, 230, 255), 0.7);
        }
    }

    // ── brows ──

    fn draw_brows(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, params: &FaceParams) {
        let raise = params.brow_raise as f64;
        for sx in [-10.5, 10.5f64].iter() {
            let by = -16.0 - raise * 4.5;
            let mut pts = Vec::new();
            for i in 0..14 {
                let t = i as f64 / 13.0;
                let bx = sx + (t - 0.5) * 10.0;
                let arch = ((t - 0.5) * 1.6).sin().abs() * 2.0;
                let by2 = by - raise * 0.5 * t - arch;
                pts.push(self.tf(bx, by2, tilt, breathe));
            }
            self.dots(ctx, &pts, Color::Rgb(55, 165, 235), 0.45 + raise.abs() as f64 * 0.1);
        }
    }

    // ── nose ──

    fn draw_nose_line(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64) {
        let mut pts = Vec::new();
        for i in 0..8 {
            let t = i as f64 / 7.0;
            pts.push(self.tf(0.0, -3.0 + t * 15.0, tilt, breathe));
        }
        self.dots(ctx, &pts, Color::Rgb(40, 130, 210), 0.18);
        let (tx, ty) = self.tf(0.0, 12.0, tilt, breathe);
        self.dot(ctx, tx, ty, Color::Rgb(55, 145, 225), 0.22);
    }

    // ── mouth ──

    fn draw_mouth(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, params: &FaceParams, valence: f64) {
        let open = params.mouth_open as f64;
        let smile = params.smile as f64;
        let my = 18.0;
        let mw = 8.0;

        // Upper lip
        let mut upper = Vec::new();
        for i in 0..20 {
            let t = i as f64 / 19.0;
            let mx = (t - 0.5) * mw * 2.0;
            let mu = my - smile * 2.0 * (1.0 - (t * 2.0 - 1.0).powi(2));
            upper.push(self.tf(mx, mu, tilt, breathe));
        }
        self.dots(ctx, &upper, Color::Rgb(65, 185, 240), 0.5);

        // Mouth opening
        if open > 0.03 {
            let gap = open * 6.0;
            let mut lower = Vec::new();
            for i in 0..14 {
                let t = i as f64 / 13.0;
                let mx = (t - 0.5) * mw * 1.5;
                let ml = my + gap * (1.0 - (t * 2.0 - 1.0).abs().powi(2));
                lower.push(self.tf(mx, ml, tilt, breathe));
            }
            self.dots(ctx, &lower, Color::Rgb(120, 210, 250), 0.35 + open * 0.5);

            // Dark interior
            let mut inner = Vec::new();
            for r in 0..3 {
                for c in 0..5 {
                    let mx = (c as f64 / 4.0 - 0.5) * mw * 0.7;
                    let mi = my + 1.2 + r as f64 * gap / 3.5;
                    inner.push(self.tf(mx, mi, tilt, breathe));
                }
            }
            self.dots(ctx, &inner, Color::Rgb(0, 8, 30), 0.35);
        }
    }

    // ── collar ──

    fn draw_collar_v(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, fc: u64) {
        let shim = (fc as f64 * 0.06).sin() * 0.08;
        let edge = Color::Rgb(50, 150, 230);
        let pts = [(-17.0, 40.0), (-5.0, 26.0), (0.0, 22.0), (5.0, 26.0), (17.0, 40.0)];
        for i in 0..4 {
            let (sx, sy) = pts[i]; let (ex, ey) = pts[i + 1];
            let (tsx, tsy) = self.tf(sx, sy, tilt, breathe);
            let (tex, tey) = self.tf(ex, ey, tilt, breathe);
            self.line_to(ctx, tsx, tsy, tex, tey, edge, 0.4 + shim);
        }
    }

    // ── crown arcs ──

    fn draw_crown_arcs(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64, arousal: f64, fc: u64) {
        let active = arousal * 0.5 + 0.35;
        for layer in 0..3 {
            let by = -32.0 - layer as f64 * 3.5;
            let r = 16.0 + layer as f64 * 4.0;
            let a = active * (0.3 + layer as f64 * 0.08);
            let segs = 14 + layer * 4;
            for i in 0..segs {
                let a1 = -TAU * 0.22 + TAU * i as f64 / segs as f64 + (fc as f64 * 0.012).sin() * 0.1;
                let a2 = -TAU * 0.22 + TAU * (i + 1) as f64 / segs as f64 + (fc as f64 * 0.012).sin() * 0.1;
                let mid = (a1 + a2) / 2.0;
                if mid.sin() > -0.2 {
                    let (sx, sy) = (a1.sin() * r, by + a1.cos() * r * 0.65);
                    let (ex, ey) = (a2.sin() * r, by + a2.cos() * r * 0.65);
                    let (tsx, tsy) = self.tf(sx, sy, tilt, breathe);
                    let (tex, tey) = self.tf(ex, ey, tilt, breathe);
                    self.line_to(ctx, tsx, tsy, tex, tey, Color::Rgb(45, 150, 230), a);
                }
            }
        }
    }

    // ── scanline ──

    fn draw_scanline(&self, ctx: &mut ratatui::widgets::canvas::Context, fc: u64) {
        let sy = ((fc as f64 * 0.35) % 120.0) - 60.0;
        let mut pts = Vec::new();
        for x in (-45..=45i32).step_by(1) {
            let x = x as f64;
            let f = (fc as f64 * 0.4 + x * 0.25).sin() * 0.12 + 0.05;
            if f > 0.03 {
                pts.push((x, sy));
                if f > 0.09 { pts.push((x, sy + 1.5)); }
            }
        }
        self.dots(ctx, &pts, Color::Rgb(70, 190, 250), 0.18);
    }
}

fn dim(color: Color, alpha: f64) -> Color {
    if alpha >= 1.0 { return color; }
    let a = alpha.max(0.0).min(1.0);
    match color {
        Color::Rgb(r, g, b) => Color::Rgb((r as f64 * a) as u8, (g as f64 * a) as u8, (b as f64 * a) as u8),
        _ => color,
    }
}

fn rand_f64() -> f64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_usize(42);
    (h.finish() as f64) / (u64::MAX as f64)
}
