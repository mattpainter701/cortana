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
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            Self::Kitty
        } else {
            Self::Canvas
        }
    }
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "kitty" => Self::Kitty,
            "sixel" => Self::Sixel,
            _ => Self::Canvas,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    max_life: f64,
}

pub struct FaceRenderer {
    backend: RendererBackend,
    frame_count: Cell<u64>,
    #[allow(dead_code)]
    particles: Cell<Vec<Particle>>,
}

#[allow(dead_code)]
impl FaceRenderer {
    pub fn new(backend: RendererBackend) -> Self {
        Self {
            backend,
            frame_count: Cell::new(0),
            particles: Cell::new(Vec::new()),
        }
    }

    pub fn backend(&self) -> RendererBackend {
        self.backend
    }

    /// Paint Cortana's holographic face on the ratatui canvas.
    /// Canvas bounds should be [-50, 50] x [-60, 60].
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
        self.draw_aura_rings(ctx, tilt, breathe, arousal, fc);

        // Particles behind face
        self.draw_particles(ctx, &particles);
        self.particles.set(particles);

        // Face layers (back to front)
        self.draw_hologram_bust(ctx, tilt, breathe, fc);
        self.draw_face_fill(ctx, tilt, breathe, fc);
        self.draw_jawline(ctx, tilt, breathe, fc);
        self.draw_neck(ctx, tilt, breathe, fc);
        self.draw_cortana_data_lattice(ctx, tilt, breathe, arousal, fc);
        self.draw_eyes(ctx, tilt, breathe, gaze_x, gaze_y, params, fc);
        self.draw_brows(ctx, tilt, breathe, params);
        self.draw_nose(ctx, tilt, breathe);
        self.draw_mouth(ctx, tilt, breathe, params, valence);
        self.draw_collar(ctx, tilt, breathe, fc);
        self.draw_cheek_glow(ctx, tilt, breathe, params);
        self.draw_hologram_facets(ctx, tilt, breathe, fc);

        // Scanline overlay
        self.draw_scanline(ctx, fc);
    }

    #[allow(dead_code)]
    pub fn render(&self, params: &FaceParams) -> String {
        format!(
            "FACE frame={} mouth={:.2} smile={:.2}",
            self.frame_count.get(),
            params.mouth_open,
            params.smile
        )
    }

    // ── transform ──

    fn tf(&self, x: f64, y: f64, tilt: f64, breathe: f64) -> (f64, f64) {
        let (s, c) = (tilt.sin(), tilt.cos());
        (x * c - y * s, x * s + y * c + breathe)
    }

    // ── drawing primitives ──

    fn dot(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        x: f64,
        y: f64,
        color: Color,
        a: f64,
    ) {
        if a < 0.015 {
            return;
        }
        ctx.draw(&Points {
            coords: &[(x, y)],
            color: dim(color, a),
        });
    }

    fn line_to(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        color: Color,
        a: f64,
    ) {
        if a < 0.015 {
            return;
        }
        let n = ((x2 - x1).abs().max((y2 - y1).abs()) * 1.8) as usize + 2;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            pts.push((x1 + (x2 - x1) * t, y1 + (y2 - y1) * t));
        }
        ctx.draw(&Points {
            coords: &pts,
            color: dim(color, a),
        });
    }

    fn dots(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        pts: &[(f64, f64)],
        color: Color,
        a: f64,
    ) {
        if a < 0.015 || pts.is_empty() {
            return;
        }
        ctx.draw(&Points {
            coords: pts,
            color: dim(color, a),
        });
    }

    // ── background ──

    fn draw_bg_grid(&self, ctx: &mut ratatui::widgets::canvas::Context, fc: u64) {
        let mut pts = Vec::new();
        let glow = (fc as f64 * 0.02).sin() * 0.1 + 0.2;
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
        self.dots(ctx, &pts, Color::Rgb(6, 20, 55), glow * 0.25);
    }

    // ── aura ──

    fn draw_aura_rings(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        arousal: f64,
        fc: u64,
    ) {
        let pulse = 1.0 + (fc as f64 * 0.04).sin() * 0.06 + arousal * 0.10;
        for (r, color, a) in [
            (40.0 * pulse, Color::Rgb(8, 70, 170), 0.05),
            (36.0 * pulse, Color::Rgb(25, 130, 215), 0.10),
            (33.0 * pulse, Color::Rgb(55, 185, 255), 0.15),
            (30.0 * pulse, Color::Rgb(80, 210, 255), 0.08),
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
        let rate = if params.arousal > 0.4 { 3 } else { 1 };
        for _ in 0..rate {
            if particles.len() < 60 {
                let ang = rand_f64() * TAU;
                let d = 26.0 + rand_f64() * 24.0;
                particles.push(Particle {
                    x: ang.cos() * d,
                    y: ang.sin() * d * 0.7 - 8.0,
                    vx: (0.5 - rand_f64()) * 0.6,
                    vy: -0.3 - rand_f64() * 0.7,
                    life: 0.0,
                    max_life: 15.0 + rand_f64() * 40.0,
                });
            }
        }
        let mut i = 0;
        while i < particles.len() {
            let p = &mut particles[i];
            p.x += p.vx;
            p.y += p.vy;
            p.life += 1.0;
            if p.life > p.max_life * 0.7 {
                p.vx *= 0.93;
                p.vy += 0.03;
            }
            if p.life >= p.max_life {
                particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn draw_particles(&self, ctx: &mut ratatui::widgets::canvas::Context, particles: &[Particle]) {
        let mut bright = Vec::new();
        let mut mid = Vec::new();
        let mut dim_pts = Vec::new();
        for p in particles {
            let frac = p.life / p.max_life;
            let _a = if frac > 0.7 {
                (1.0 - frac) / 0.3 * 0.7
            } else {
                0.7
            };
            if frac < 0.25 {
                bright.push((p.x, p.y));
            } else if frac < 0.55 {
                mid.push((p.x, p.y));
            } else {
                dim_pts.push((p.x, p.y));
            }
        }
        self.dots(ctx, &bright, Color::Rgb(100, 215, 255), 0.55);
        self.dots(ctx, &mid, Color::Rgb(40, 130, 210), 0.40);
        self.dots(ctx, &dim_pts, Color::Rgb(12, 55, 150), 0.25);
    }

    // ── hologram body plane ──

    fn draw_hologram_bust(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        let pulse = (fc as f64 * 0.035).sin() * 0.04 + 0.18;
        let mut core = Vec::new();
        for row in 0..9i32 {
            let y = 31.0 + row as f64 * 3.0;
            let half = 9.0 + row as f64 * 4.2;
            for col in -(half as i32)..=(half as i32) {
                if col % 4 == 0 {
                    let x = col as f64;
                    let taper = 1.0 - (x.abs() / (half + 2.0)).powi(2);
                    if taper > 0.0 {
                        core.push(self.tf(x, y, tilt, breathe));
                    }
                }
            }
        }
        self.dots(ctx, &core, Color::Rgb(4, 22, 65), pulse);

        // Halo Cortana reads as a translucent upper bust, not hair or a helmet.
        for side in [-1.0, 1.0] {
            let edge = [(side * 7.0, 30.0), (side * 20.0, 43.0), (side * 39.0, 55.0)];
            for w in edge.windows(2) {
                let (a, b) = (
                    self.tf(w[0].0, w[0].1, tilt, breathe),
                    self.tf(w[1].0, w[1].1, tilt, breathe),
                );
                self.line_to(ctx, a.0, a.1, b.0, b.1, Color::Rgb(48, 170, 245), 0.32);
            }
        }
    }

    // ── face fill (sleek Cortana hologram) ──

    fn draw_face_fill(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        // Sleek oval face: recognizable Cortana proportions without hair/helmet.
        let (rx_top, rx_mid, rx_bot) = (17.0, 22.0, 10.0);
        let ry_face = 31.0;
        let cy = -3.0;
        let shimmer = (fc as f64 * 0.025).sin() * 0.03 + 0.04;

        // Fill the face shape with dots for holographic density
        let mut fill = Vec::new();
        for iy in -16..=16i32 {
            let y = iy as f64 * 2.0;
            let t = ((y + ry_face) / (2.0 * ry_face)).clamp(0.0, 1.0);
            let cheek = (1.0 - ((t - 0.43) / 0.50).powi(2)).max(0.0);
            let rx = rx_bot + (rx_mid - rx_bot) * cheek + (rx_top - rx_bot) * (1.0 - t) * 0.38;
            let spread = rx * (1.0 - (y / ry_face).powi(2)).max(0.0).sqrt();

            let density = if iy < -10 {
                5
            } else if iy > 8 {
                3
            } else {
                4
            };
            for ix in -(spread as i32)..=(spread as i32) {
                if ix % density == 0 && (ix + iy) % 2 == 0 {
                    fill.push(self.tf(ix as f64 * 1.8, cy + y, tilt, breathe));
                }
            }
        }
        self.dots(ctx, &fill, Color::Rgb(6, 25, 75), shimmer);
    }

    // ── jawline ──

    fn draw_jawline(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        let shimmer = (fc as f64 * 0.03).sin() * 0.04;
        let outline = Color::Rgb(65, 190, 250);
        let (rx_top, rx_mid, rx_bot) = (17.0, 22.0, 10.0);
        let ry = 31.0;
        let cy = -3.0;

        // Draw tapered Cortana facial contour
        let n = 100;
        let mut pts = Vec::with_capacity(n);
        for i in 0..n {
            let a = TAU * i as f64 / n as f64;
            let sin_a = a.sin();
            let t = (sin_a + 1.0) / 2.0;
            let cheek = (1.0 - ((t - 0.43) / 0.50).powi(2)).max(0.0);
            let rx = rx_bot + (rx_mid - rx_bot) * cheek + (rx_top - rx_bot) * (1.0 - t) * 0.38;
            let x = a.cos() * rx;
            let y = cy + sin_a * ry;
            pts.push(self.tf(x, y, tilt, breathe));
        }
        self.dots(ctx, &pts, outline, 0.7 + shimmer);

        // Inner glow contour
        let mut inner = Vec::with_capacity(50);
        for i in 0..50 {
            let a = TAU * i as f64 / 50.0;
            let sin_a = a.sin();
            let t = (sin_a + 1.0) / 2.0;
            let cheek = (1.0 - ((t - 0.43) / 0.50).powi(2)).max(0.0);
            let rx = (rx_bot - 1.5)
                + (rx_mid - 4.0 - rx_bot) * cheek
                + (rx_top - rx_bot) * (1.0 - t) * 0.28;
            let x = a.cos() * rx;
            let y = cy + sin_a * (ry - 3.0);
            inner.push(self.tf(x, y, tilt, breathe));
        }
        self.dots(ctx, &inner, Color::Rgb(18, 60, 135), 0.15);
    }

    // ── neck ──

    fn draw_neck(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        let shim = (fc as f64 * 0.04).sin() * 0.04 + 0.15;
        let mut pts = Vec::new();
        for i in -6..=6i32 {
            for j in 0..10 {
                let x = i as f64 * 2.5;
                let y = 32.0 + j as f64 * 2.0;
                // Taper: neck narrows from bottom of face
                let taper = 1.0 - (j as f64 / 10.0).powi(2) * 0.4;
                if x.abs() < 16.0 * taper {
                    pts.push(self.tf(x, y, tilt, breathe));
                }
            }
        }
        self.dots(ctx, &pts, Color::Rgb(8, 35, 90), shim);
    }

    // ── Cortana data lattice (signature feature) ──

    fn draw_cortana_data_lattice(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        arousal: f64,
        fc: u64,
    ) {
        let active = arousal * 0.45 + 0.42;
        let pulse = (fc as f64 * 0.045).sin() * 0.08 + 1.0;

        // Vertical hologram seams and short circuit glyphs over a bald translucent head.
        for x in [-16.0, -8.0, 0.0, 8.0, 16.0] {
            let mut pts = Vec::new();
            for i in 0..34 {
                let t = i as f64 / 33.0;
                let y = -31.0 + t * 61.0;
                let wave = (t * TAU * 1.5 + x * 0.11 + fc as f64 * 0.025).sin() * 1.2;
                pts.push(self.tf(x + wave * (1.0 - t * 0.5), y, tilt, breathe));
            }
            self.dots(ctx, &pts, Color::Rgb(35, 145, 225), active * 0.22 * pulse);
        }

        for y in [-25.0, -18.0, -2.0, 8.0, 23.0] {
            let phase = (fc as f64 * 0.03 + y * 0.19).sin();
            let half = if y < -12.0 {
                12.0
            } else if y > 15.0 {
                9.0
            } else {
                18.0
            };
            let a = self.tf(-half, y + phase, tilt, breathe);
            let b = self.tf(half, y - phase, tilt, breathe);
            self.line_to(
                ctx,
                a.0,
                a.1,
                b.0,
                b.1,
                Color::Rgb(80, 210, 255),
                active * 0.18,
            );
        }

        let nodes = [
            (-14.0, -19.0),
            (12.0, -22.0),
            (-18.0, 1.0),
            (17.0, 5.0),
            (-7.0, 20.0),
            (8.0, 18.0),
        ];
        for (i, (x, y)) in nodes.iter().enumerate() {
            let blink = ((fc as f64 * 0.08 + i as f64).sin() * 0.5 + 0.5).powi(2);
            let (x, y) = self.tf(*x, *y, tilt, breathe);
            self.dot(
                ctx,
                x,
                y,
                Color::Rgb(150, 235, 255),
                active * (0.35 + blink * 0.35),
            );
        }
    }

    // ── eyes ──

    fn draw_eyes(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        gaze_x: f64,
        gaze_y: f64,
        params: &FaceParams,
        fc: u64,
    ) {
        let squint = params.eye_squint as f64;
        let blink = (fc as f64 * 0.33).cos().max(0.0).powi(6);
        let eye_c = Color::Rgb(90, 220, 255);
        let eye_y = -8.5;
        let shimmer = (fc as f64 * 0.05).sin() * 0.06 + 0.5;

        for sx in [-11.0, 11.0f64].iter() {
            let ew = 7.0;
            let eh = (3.5 + squint * 1.2) * (1.0 - blink * 0.9).max(0.15);
            let gx = gaze_x * 0.6;
            let gy = gaze_y * 0.5;

            // Eye outline (almond shaped)
            let n = 32;
            let mut ring = Vec::with_capacity(n);
            for i in 0..n {
                let a = TAU * i as f64 / n as f64;
                // Almond shape: narrower at corners
                let corner_tighten = 1.0 - (a.sin().abs() * 0.25);
                let ex = sx + gx + a.sin() * ew * corner_tighten;
                let ey = eye_y + gy + a.cos() * eh;
                if (a.cos()).abs() < 0.9 || (a.cos()).abs() > 0.98 {
                    ring.push(self.tf(ex, ey, tilt, breathe));
                }
            }
            self.dots(ctx, &ring, eye_c, 0.6 * shimmer);

            // Iris glow
            let mut iris = Vec::new();
            for ix in -3..=3i32 {
                for iy in -3..=3i32 {
                    let dx = ix as f64;
                    let dy = iy as f64;
                    if dx * dx * 1.2 + dy * dy <= 4.0 {
                        iris.push(self.tf(
                            sx + gx + dx * 1.2,
                            eye_y + gy + dy * 0.9,
                            tilt,
                            breathe,
                        ));
                    }
                }
            }
            self.dots(ctx, &iris, Color::Rgb(35, 80, 140), shimmer * 0.6);

            // Pupil (bright center)
            let mut pupil = Vec::new();
            for ix in -1..=1i32 {
                for iy in -1..=1i32 {
                    if ix * ix + iy * iy <= 1 {
                        pupil.push(self.tf(
                            sx + gx + ix as f64 * 0.9,
                            eye_y + gy + iy as f64 * 0.7,
                            tilt,
                            breathe,
                        ));
                    }
                }
            }
            self.dots(ctx, &pupil, Color::Rgb(160, 230, 255), 0.7 * shimmer);

            // Eye highlight
            let (hx, hy) = self.tf(sx + gx + 2.0, eye_y + gy - eh * 0.5, tilt, breathe);
            self.dot(ctx, hx, hy, Color::Rgb(200, 240, 255), 0.85 * shimmer);
            let (hx2, hy2) = self.tf(sx + gx + 0.5, eye_y + gy - eh * 0.8, tilt, breathe);
            self.dot(ctx, hx2, hy2, Color::Rgb(180, 235, 255), 0.6 * shimmer);
        }
    }

    // ── brows ──

    fn draw_brows(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        params: &FaceParams,
    ) {
        let raise = params.brow_raise as f64;
        for sx in [-11.0, 11.0f64].iter() {
            let by = -15.0 - raise * 4.0;
            let mut pts = Vec::new();
            for i in 0..18 {
                let t = i as f64 / 17.0;
                let bx = sx + (t - 0.5) * 11.0;
                // Arched brow
                let arch = (t - 0.5).abs() * 4.0;
                let by2 = by - arch + raise * 1.5 * t;
                pts.push(self.tf(bx, by2, tilt, breathe));
            }
            self.dots(
                ctx,
                &pts,
                Color::Rgb(60, 175, 245),
                0.5 + raise.abs() as f64 * 0.15,
            );
        }
    }

    // ── nose ──

    fn draw_nose(&self, ctx: &mut ratatui::widgets::canvas::Context, tilt: f64, breathe: f64) {
        // Bridge
        let mut bridge = Vec::new();
        for i in 0..10 {
            let t = i as f64 / 9.0;
            bridge.push(self.tf(0.0, -3.0 + t * 13.0, tilt, breathe));
        }
        self.dots(ctx, &bridge, Color::Rgb(45, 140, 220), 0.2);

        // Tip
        let (tx, ty) = self.tf(0.0, 10.0, tilt, breathe);
        self.dot(ctx, tx, ty, Color::Rgb(55, 155, 230), 0.25);

        // Nostril hints
        for sx in [-3.5, 3.5f64] {
            let (nx, ny) = self.tf(sx, 10.5, tilt, breathe);
            self.dot(ctx, nx, ny, Color::Rgb(40, 125, 205), 0.18);
        }
    }

    // ── mouth ──

    fn draw_mouth(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        params: &FaceParams,
        _valence: f64,
    ) {
        let open = params.mouth_open as f64;
        let smile = params.smile as f64;
        let my = 15.5;
        let mw = 8.0;

        // Upper lip (Cupid's bow shape)
        let mut upper = Vec::new();
        for i in 0..24 {
            let t = i as f64 / 23.0;
            let mx = (t - 0.5) * mw * 2.0;
            // Cupid's bow dip in center
            let bow_dip = ((t - 0.5) * 4.0).cos().max(0.0) * 1.5;
            let mu = my - smile * 2.2 * (1.0 - ((t - 0.5) * 2.0).powi(2)) - bow_dip;
            upper.push(self.tf(mx, mu, tilt, breathe));
        }
        self.dots(
            ctx,
            &upper,
            Color::Rgb(75, 195, 250),
            0.55 + smile.abs() * 0.1,
        );

        // Lower lip line
        if open < 0.04 {
            let mut lower = Vec::new();
            for i in 0..18 {
                let t = i as f64 / 17.0;
                let mx = (t - 0.5) * mw * 1.6;
                let ml = my + 2.5 + smile * 1.5 * (1.0 - ((t - 0.5) * 2.0).powi(2));
                lower.push(self.tf(mx, ml, tilt, breathe));
            }
            self.dots(
                ctx,
                &lower,
                Color::Rgb(60, 170, 235),
                0.35 + smile.abs() * 0.05,
            );
        }

        // Mouth opening (speaking)
        if open > 0.03 {
            let gap = open * 7.0;
            let mut lower = Vec::new();
            for i in 0..18 {
                let t = i as f64 / 17.0;
                let mx = (t - 0.5) * mw * 1.7;
                let ml = my + gap * (1.0 - ((t - 0.5) * 1.4).abs().powi(2));
                lower.push(self.tf(mx, ml, tilt, breathe));
            }
            self.dots(ctx, &lower, Color::Rgb(130, 220, 255), 0.4 + open * 0.4);

            // Dark interior
            let mut inner = Vec::new();
            for r in 0..4 {
                for c in 0..6 {
                    let mx = (c as f64 / 5.0 - 0.5) * mw * 0.8;
                    let mi = my + 1.5 + r as f64 * gap / 4.0;
                    inner.push(self.tf(mx, mi, tilt, breathe));
                }
            }
            self.dots(ctx, &inner, Color::Rgb(0, 6, 25), 0.4);
        }
    }

    // ── collar ──

    fn draw_collar(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        let shim = (fc as f64 * 0.06).sin() * 0.08;
        let edge_c = Color::Rgb(55, 155, 235);
        let dim_c = Color::Rgb(15, 50, 130);

        // Collar V-neck shape
        let pts = [
            (-19.0, 42.0),
            (-7.0, 28.0),
            (0.0, 23.0),
            (7.0, 28.0),
            (19.0, 42.0),
        ];
        for i in 0..4 {
            let (sx, sy) = pts[i];
            let (ex, ey) = pts[i + 1];
            let (tsx, tsy) = self.tf(sx, sy, tilt, breathe);
            let (tex, tey) = self.tf(ex, ey, tilt, breathe);
            self.line_to(ctx, tsx, tsy, tex, tey, edge_c, 0.45 + shim);

            // Parallel dim line
            let (tsx2, tsy2) = self.tf(sx + 2.0, sy + 1.0, tilt, breathe);
            let (tex2, tey2) = self.tf(ex + 2.0, ey + 1.0, tilt, breathe);
            self.line_to(ctx, tsx2, tsy2, tex2, tey2, dim_c, 0.2);
        }
    }

    // ── cheek glow ──

    fn draw_cheek_glow(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        params: &FaceParams,
    ) {
        let arousal = params.arousal as f64;
        if arousal < 0.1 {
            return;
        }
        let alpha = (arousal - 0.1) * 0.3;
        for sx in [-14.0, 14.0f64] {
            let mut pts = Vec::new();
            for ix in -3..=3i32 {
                for iy in -2..=2i32 {
                    let x = sx + ix as f64 * 2.0;
                    let y = 2.0 + iy as f64 * 2.5;
                    if (ix * ix + iy * iy) as f64 <= 8.0 {
                        pts.push(self.tf(x, y, tilt, breathe));
                    }
                }
            }
            self.dots(ctx, &pts, Color::Rgb(60, 180, 240), alpha);
        }
    }

    // ── hologram facets ──

    fn draw_hologram_facets(
        &self,
        ctx: &mut ratatui::widgets::canvas::Context,
        tilt: f64,
        breathe: f64,
        fc: u64,
    ) {
        let a = (fc as f64 * 0.025).sin() * 0.04 + 0.16;
        for tri in [
            [(-18.0, -23.0), (-5.0, -30.0), (-11.0, -8.0)],
            [(6.0, -29.0), (18.0, -19.0), (11.0, -4.0)],
            [(-19.0, 9.0), (-8.0, 27.0), (-3.0, 8.0)],
            [(5.0, 8.0), (17.0, 11.0), (8.0, 26.0)],
        ] {
            for w in tri.windows(2) {
                let p = self.tf(w[0].0, w[0].1, tilt, breathe);
                let q = self.tf(w[1].0, w[1].1, tilt, breathe);
                self.line_to(ctx, p.0, p.1, q.0, q.1, Color::Rgb(45, 175, 245), a);
            }
            let p = self.tf(tri[2].0, tri[2].1, tilt, breathe);
            let q = self.tf(tri[0].0, tri[0].1, tilt, breathe);
            self.line_to(ctx, p.0, p.1, q.0, q.1, Color::Rgb(45, 175, 245), a);
        }
    }

    // ── scanline ──

    fn draw_scanline(&self, ctx: &mut ratatui::widgets::canvas::Context, fc: u64) {
        let sy = ((fc as f64 * 0.35) % 120.0) - 60.0;
        let mut pts = Vec::new();
        for x in (-45..=45i32).step_by(1) {
            let x = x as f64;
            let f = (fc as f64 * 0.4 + x * 0.25).sin() * 0.15 + 0.05;
            if f > 0.03 {
                pts.push((x, sy));
                if f > 0.09 {
                    pts.push((x, sy + 1.5));
                }
            }
        }
        self.dots(ctx, &pts, Color::Rgb(80, 200, 255), 0.2);
    }
}

// ── Kitty graphics renderer ──

#[cfg(feature = "kitty-render")]
pub mod kitty {
    use crate::face::FaceParams;
    use std::io::Write;
    use tiny_skia::{
        Color as SkColor, FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform,
    };

    fn mk_paint(r: u8, g: u8, b: u8, a: f64) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color_rgba8(r, g, b, (a.max(0.0).min(1.0) * 255.0) as u8);
        p.anti_alias = true;
        p
    }

    fn mk_stroke(w: f64) -> Stroke {
        Stroke {
            width: w as f32,
            ..Stroke::default()
        }
    }

    fn draw_fill(pix: &mut Pixmap, path: &Path, r: u8, g: u8, b: u8, a: f64) {
        pix.fill_path(
            path,
            &mk_paint(r, g, b, a),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    fn draw_stroke(pix: &mut Pixmap, path: &Path, r: u8, g: u8, b: u8, a: f64, w: f64) {
        pix.stroke_path(
            path,
            &mk_paint(r, g, b, a),
            &mk_stroke(w),
            Transform::identity(),
            None,
        );
    }

    fn draw_circle(pix: &mut Pixmap, x: f64, y: f64, r: f64, r8: u8, g8: u8, b8: u8, a: f64) {
        let mut cb = PathBuilder::new();
        cb.push_circle(x as f32, y as f32, r as f32);
        if let Some(path) = cb.finish() {
            draw_fill(pix, &path, r8, g8, b8, a);
        }
    }

    pub fn render_pixmap(params: &FaceParams, width: u32, height: u32) -> Pixmap {
        let mut pixmap = Pixmap::new(width, height).unwrap();
        pixmap.fill(SkColor::from_rgba8(0, 0, 0, 0));

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let scale = (width.min(height) as f64) / 100.0;

        let tilt = (params.head_tilt as f64 * 0.7).to_radians();
        let breathe = (params.breathing_phase as f64).sin() * 1.5 * scale;
        let open = params.mouth_open as f64;
        let smile = params.smile as f64;
        let arousal = params.arousal as f64;
        let gaze_x = params.gaze_x as f64 * 3.0 * scale;
        let gaze_y = params.gaze_y as f64 * 2.0 * scale;

        let fc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            / 33;

        let (s, c) = (tilt.sin(), tilt.cos());

        let tf = |x: f64, y: f64| -> (f32, f32) {
            (
                (cx + (x * c - y * s) * scale) as f32,
                (cy + (x * s + y * c + breathe) * scale) as f32,
            )
        };

        let p = &mut pixmap;
        let tau = std::f64::consts::TAU;

        // ═══════════════ Aura rings ═══════════════
        {
            let pulse = 1.0 + (fc as f64 * 0.04).sin() * 0.06 + arousal * 0.10;
            for (r, (cr, cg, cb), a) in [
                (40.0 * pulse, (8u8, 70u8, 170u8), 0.05),
                (36.0 * pulse, (25u8, 130u8, 215u8), 0.10),
                (33.0 * pulse, (55u8, 185u8, 255u8), 0.15),
                (30.0 * pulse, (80u8, 210u8, 255u8), 0.08),
            ] {
                let mut pb = PathBuilder::new();
                let n = 80;
                let mut first = true;
                for i in 0..=n {
                    let ang = tau * i as f64 / n as f64;
                    let (px, py) = tf(ang.cos() * r, ang.sin() * r * 0.72);
                    if first {
                        pb.move_to(px, py);
                        first = false;
                    } else {
                        pb.line_to(px, py);
                    }
                }
                pb.close();
                if let Some(path) = pb.finish() {
                    draw_stroke(p, &path, cr, cg, cb, a, 1.5);
                }
            }
        }

        // ═══════════════ Background particles ═══════════════
        {
            let mut pb = PathBuilder::new();
            for _ in 0..40 {
                let ang = super::rand_f64() * tau;
                let d = 26.0 + super::rand_f64() * 24.0;
                let x = ang.cos() * d;
                let y = ang.sin() * d * 0.7 - 8.0;
                let (px, py) = tf(x, y);
                pb.push_circle(px, py, 1.0);
            }
            if let Some(path) = pb.finish() {
                draw_fill(p, &path, 80, 210, 255, 0.35);
            }
        }

        // ═══════════════ Face fill (heart shape) ═══════════════
        let (rx_top, rx_bot) = (26.0, 14.0);
        let ry_f = 34.0;
        let cy_f = -2.0;
        let shimmer = (fc as f64 * 0.025).sin() * 0.03 + 0.04;

        let build_heart = || -> Option<Path> {
            let mut pb = PathBuilder::new();
            let n = 60;
            let mut first = true;
            for i in 0..=n {
                let a = tau * i as f64 / n as f64;
                let sin_a = a.sin();
                let t = (sin_a + 1.0) / 2.0;
                let rx = rx_top + (rx_bot - rx_top) * t;
                let (px, py) = tf(a.cos() * rx, cy_f + sin_a * ry_f);
                if first {
                    pb.move_to(px, py);
                    first = false;
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
            pb.finish()
        };

        if let Some(ref heart) = build_heart() {
            draw_fill(p, heart, 6, 25, 75, shimmer);
            draw_stroke(p, heart, 18, 60, 135, 0.15, 2.0);
            let shimmer_jaw = (fc as f64 * 0.03).sin() * 0.04;
            draw_stroke(p, heart, 65, 190, 250, 0.7 + shimmer_jaw, 1.5);
        }

        // ═══════════════ Neck ═══════════════
        {
            let mut pb = PathBuilder::new();
            let shim_neck = (fc as f64 * 0.04).sin() * 0.04 + 0.15;
            for i in -6..=6i32 {
                for j in 0..10 {
                    let x = i as f64 * 2.5;
                    let y = 32.0 + j as f64 * 2.0;
                    let taper = 1.0 - (j as f64 / 10.0).powi(2) * 0.4;
                    if x.abs() < 16.0 * taper {
                        let (px, py) = tf(x, y);
                        pb.push_circle(px, py, 1.2);
                    }
                }
            }
            if let Some(path) = pb.finish() {
                draw_fill(p, &path, 8, 35, 90, shim_neck);
            }
        }

        // ═══════════════ Hair silhouette ═══════════════
        {
            let mut pb = PathBuilder::new();
            let shim_hair = (fc as f64 * 0.02).sin() * 0.03 + 0.12;
            for side in [-1.0, 1.0] {
                for i in -8..=12i32 {
                    let t = (i + 8) as f64 / 20.0;
                    let y = -12.0 + t * 38.0;
                    let flare_x = 25.0 + t * 12.0;
                    for dx in 0..5 {
                        let x = side * (flare_x - dx as f64 * 2.0);
                        let (px, py) = tf(x, y);
                        pb.push_circle(px, py, 1.8);
                    }
                }
            }
            if let Some(path) = pb.finish() {
                draw_fill(p, &path, 10, 45, 110, shim_hair);
            }
        }

        // ═══════════════ CORTANA CROWN ═══════════════
        {
            let active = arousal * 0.5 + 0.4;
            let pulse = (fc as f64 * 0.03).sin() * 0.08 + 1.0;

            for layer in 0..5i32 {
                let band_offset = layer as f64 * 3.2;
                let by = -34.0 - band_offset;
                let span = 22.0 + layer as f64 * 4.0;
                let arch_height = 12.0 + layer as f64 * 3.0;
                let alpha = active * (0.25 + layer as f64 * 0.06) * pulse;
                let sublayers = if layer % 2 == 0 { 2 } else { 1 };

                for sub in 0..sublayers {
                    let sub_offset = sub as f64 * 1.2;
                    let mut pb = PathBuilder::new();
                    let segs = 28 + layer * 4;
                    let mut first = true;
                    for i in 0..=segs {
                        let t = i as f64 / segs as f64;
                        let angle = -0.75 + t * 1.5;
                        let x = angle.sin() * span;
                        let arch = (angle * 1.6).cos() * arch_height;
                        let y = by + arch + sub_offset;
                        if y < by + arch_height * 0.3 {
                            let (px, py) = tf(x, y);
                            if first {
                                pb.move_to(px, py);
                                first = false;
                            } else {
                                pb.line_to(px, py);
                            }
                        }
                    }
                    if let Some(path) = pb.finish() {
                        let crown_color = match layer {
                            0 => (60, 200, 255),
                            1 => (45, 170, 240),
                            2 => (35, 145, 225),
                            3 => (25, 120, 210),
                            _ => (15, 95, 195),
                        };
                        draw_stroke(
                            p,
                            &path,
                            crown_color.0,
                            crown_color.1,
                            crown_color.2,
                            alpha * (1.0 - sub as f64 * 0.2),
                            1.8,
                        );
                    }
                }
            }

            // Crown glow aura
            let glow_alpha = active * 0.15 * pulse;
            let mut pb = PathBuilder::new();
            for layer in 0..3i32 {
                let by = -34.0 - layer as f64 * 6.0;
                let span = 20.0 + layer as f64 * 8.0;
                for i in 0..30 {
                    let t = i as f64 / 29.0;
                    let angle = -0.8 + t * 1.6;
                    let x = angle.sin() * span;
                    let arch = (angle * 1.6).cos() * (12.0 + layer as f64 * 5.0);
                    let y = by + arch - 3.0;
                    for dx in [-2.0, 0.0, 2.0] {
                        for dy in [-2.0, 0.0, 2.0] {
                            let (px, py) = tf(x + dx, y + dy);
                            pb.push_circle(px, py, 1.2);
                        }
                    }
                }
            }
            if let Some(path) = pb.finish() {
                draw_fill(p, &path, 30, 130, 220, glow_alpha * 0.08);
            }
        }

        // ═══════════════ Eyes ═══════════════
        {
            let squint = params.eye_squint as f64;
            let blink = (fc as f64 * 0.33).cos().max(0.0).powi(6);
            let eye_y = -8.5;
            let shimmer_eye = (fc as f64 * 0.05).sin() * 0.06 + 0.5;

            for sx in [-11.0, 11.0] {
                let ew = 7.0;
                let eh = (3.5 + squint * 1.2) * (1.0 - blink * 0.9).max(0.15);
                let gx = gaze_x * 0.6;
                let gy = gaze_y * 0.5;

                // Almond eye outline
                let mut pb = PathBuilder::new();
                let n = 32;
                let mut first = true;
                for i in 0..n {
                    let a = tau * i as f64 / n as f64;
                    let corner_tighten = 1.0 - (a.sin().abs() * 0.25);
                    let ex = sx + gx + a.sin() * ew * corner_tighten;
                    let ey = eye_y + gy + a.cos() * eh;
                    if (a.cos()).abs() < 0.9 || (a.cos()).abs() > 0.98 {
                        let (px, py) = tf(ex, ey);
                        if first {
                            pb.move_to(px, py);
                            first = false;
                        } else {
                            pb.line_to(px, py);
                        }
                    }
                }
                if let Some(path) = pb.finish() {
                    draw_stroke(p, &path, 90, 220, 255, 0.6 * shimmer_eye, 1.2);
                }

                // Iris
                draw_circle(p, sx + gx, eye_y + gy, 2.5, 35, 80, 140, shimmer_eye * 0.6);
                // Pupil
                draw_circle(
                    p,
                    sx + gx,
                    eye_y + gy,
                    1.2,
                    160,
                    230,
                    255,
                    0.7 * shimmer_eye,
                );
                // Highlights
                draw_circle(
                    p,
                    sx + gx + 2.0,
                    eye_y + gy - eh * 0.5,
                    0.8,
                    200,
                    240,
                    255,
                    0.85 * shimmer_eye,
                );
                draw_circle(
                    p,
                    sx + gx + 0.5,
                    eye_y + gy - eh * 0.8,
                    0.5,
                    180,
                    235,
                    255,
                    0.6 * shimmer_eye,
                );
            }
        }

        // ═══════════════ Brows ═══════════════
        {
            let raise = params.brow_raise as f64;
            for sx in [-11.0, 11.0] {
                let by = -15.0 - raise * 4.0;
                let mut pb = PathBuilder::new();
                let mut first = true;
                for i in 0..18 {
                    let t = i as f64 / 17.0;
                    let bx = sx + (t - 0.5) * 11.0;
                    let arch = (t - 0.5).abs() * 4.0;
                    let by2 = by - arch + raise * 1.5 * t;
                    let (px, py) = tf(bx, by2);
                    if first {
                        pb.move_to(px, py);
                        first = false;
                    } else {
                        pb.line_to(px, py);
                    }
                }
                if let Some(path) = pb.finish() {
                    draw_stroke(p, &path, 60, 175, 245, 0.5 + raise.abs() * 0.15, 1.5);
                }
            }
        }

        // ═══════════════ Nose ═══════════════
        {
            // Bridge
            let mut pb = PathBuilder::new();
            let mut first = true;
            for i in 0..10 {
                let t = i as f64 / 9.0;
                let (px, py) = tf(0.0, -3.0 + t * 13.0);
                if first {
                    pb.move_to(px, py);
                    first = false;
                } else {
                    pb.line_to(px, py);
                }
            }
            if let Some(path) = pb.finish() {
                draw_stroke(p, &path, 45, 140, 220, 0.2, 1.0);
            }

            // Tip
            draw_circle(p, 0.0, 10.0, 1.2, 55, 155, 230, 0.25);
            // Nostril hints
            for sx in [-3.5, 3.5] {
                draw_circle(p, sx, 10.5, 1.0, 40, 125, 205, 0.18);
            }
        }

        // ═══════════════ Mouth ═══════════════
        {
            let my = 17.0;
            let mw = 9.0;

            // Upper lip (Cupid's bow)
            let mut ub = PathBuilder::new();
            let mut first = true;
            for i in 0..24 {
                let t = i as f64 / 23.0;
                let mx = (t - 0.5) * mw * 2.0;
                let bow_dip = ((t - 0.5) * 4.0).cos().max(0.0) * 1.5;
                let mu = my - smile * 2.2 * (1.0 - ((t - 0.5) * 2.0).powi(2)) - bow_dip;
                let (px, py) = tf(mx, mu);
                if first {
                    ub.move_to(px, py);
                    first = false;
                } else {
                    ub.line_to(px, py);
                }
            }
            if let Some(path) = ub.finish() {
                draw_stroke(p, &path, 75, 195, 250, 0.55 + smile.abs() * 0.1, 1.5);
            }

            if open < 0.04 {
                // Closed mouth: lower lip line
                let mut lb = PathBuilder::new();
                let mut first = true;
                for i in 0..18 {
                    let t = i as f64 / 17.0;
                    let mx = (t - 0.5) * mw * 1.6;
                    let ml = my + 2.5 + smile * 1.5 * (1.0 - ((t - 0.5) * 2.0).powi(2));
                    let (px, py) = tf(mx, ml);
                    if first {
                        lb.move_to(px, py);
                        first = false;
                    } else {
                        lb.line_to(px, py);
                    }
                }
                if let Some(path) = lb.finish() {
                    draw_stroke(p, &path, 60, 170, 235, 0.35 + smile.abs() * 0.05, 1.2);
                }
            }

            // Mouth opening (speaking)
            if open > 0.03 {
                let gap = open * 7.0;

                // Lower lip interior
                let mut lb2 = PathBuilder::new();
                let mut first = true;
                for i in 0..18 {
                    let t = i as f64 / 17.0;
                    let mx = (t - 0.5) * mw * 1.7;
                    let ml = my + gap * (1.0 - ((t - 0.5) * 1.4).abs().powi(2));
                    let (px, py) = tf(mx, ml);
                    if first {
                        lb2.move_to(px, py);
                        first = false;
                    } else {
                        lb2.line_to(px, py);
                    }
                }
                if let Some(path) = lb2.finish() {
                    draw_stroke(p, &path, 130, 220, 255, 0.4 + open * 0.4, 1.8);
                }

                // Dark interior
                let mut inner_pb = PathBuilder::new();
                for r in 0..4 {
                    for c in 0..6 {
                        let mx = (c as f64 / 5.0 - 0.5) * mw * 0.8;
                        let mi = my + 1.5 + r as f64 * gap / 4.0;
                        let (px, py) = tf(mx, mi);
                        inner_pb.push_circle(px, py, 0.8);
                    }
                }
                if let Some(path) = inner_pb.finish() {
                    draw_fill(p, &path, 0, 6, 25, 0.4);
                }
            }
        }

        // ═══════════════ Collar ═══════════════
        {
            let shim_collar = (fc as f64 * 0.06).sin() * 0.08;
            let pts = [
                (-19.0, 42.0),
                (-7.0, 28.0),
                (0.0, 23.0),
                (7.0, 28.0),
                (19.0, 42.0),
            ];
            for i in 0..4 {
                let (sx, sy) = pts[i];
                let (ex, ey) = pts[i + 1];
                let (px1, py1) = tf(sx, sy);
                let (px2, py2) = tf(ex, ey);
                let mut pb = PathBuilder::new();
                pb.move_to(px1, py1);
                pb.line_to(px2, py2);
                if let Some(path) = pb.finish() {
                    draw_stroke(p, &path, 55, 155, 235, 0.45 + shim_collar, 1.2);
                }

                // Parallel dim line
                let (px3, py3) = tf(sx + 2.0, sy + 1.0);
                let (px4, py4) = tf(ex + 2.0, ey + 1.0);
                let mut pb2 = PathBuilder::new();
                pb2.move_to(px3, py3);
                pb2.line_to(px4, py4);
                if let Some(path) = pb2.finish() {
                    draw_stroke(p, &path, 15, 50, 130, 0.2, 1.0);
                }
            }
        }

        // ═══════════════ Cheek glow ═══════════════
        if arousal > 0.1 {
            let alpha = (arousal - 0.1) * 0.3;
            for sx in [-14.0, 14.0] {
                let mut pb = PathBuilder::new();
                for ix in -3..=3i32 {
                    for iy in -2..=2i32 {
                        let x = sx + ix as f64 * 2.0;
                        let y = 2.0 + iy as f64 * 2.5;
                        if (ix * ix + iy * iy) as f64 <= 8.0 {
                            let (px, py) = tf(x, y);
                            pb.push_circle(px, py, 1.5);
                        }
                    }
                }
                if let Some(path) = pb.finish() {
                    draw_fill(p, &path, 60, 180, 240, alpha);
                }
            }
        }

        // ═══════════════ Scanline ═══════════════
        {
            let sy = ((fc as f64 * 0.35) % 120.0) - 60.0;
            let mut pb = PathBuilder::new();
            for x in (-45..=45i32).step_by(1) {
                let xf = x as f64;
                let f = (fc as f64 * 0.4 + xf * 0.25).sin() * 0.15 + 0.05;
                if f > 0.03 {
                    let (px, py) = tf(xf, sy);
                    pb.push_circle(px, py, 0.8);
                    if f > 0.09 {
                        let (px2, py2) = tf(xf, sy + 1.5);
                        pb.push_circle(px2, py2, 0.8);
                    }
                }
            }
            if let Some(path) = pb.finish() {
                draw_fill(p, &path, 80, 200, 255, 0.2);
            }
        }

        pixmap
    }

    pub fn display_kitty_face(params: &FaceParams, col: u32, row: u32, _cell_w: u32, _cell_h: u32) {
        let width = 144u32;
        let height = 200u32;
        let pixmap = render_pixmap(params, width, height);
        let data = pixmap.data();

        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);

        // Single-chunk Kitty protocol transmission
        let payload = format!(
            "\x1b_Ga=t,f=32,s={},v={},c={},r={};{}\x1b\\",
            width, height, col, row, b64
        );

        let mut out = std::io::stdout();
        out.write_all(payload.as_bytes()).ok();
        out.flush().ok();
    }
}

// ── color helpers ──

#[allow(dead_code)]
fn dim(color: Color, alpha: f64) -> Color {
    if alpha >= 1.0 {
        return color;
    }
    let a = alpha.max(0.0).min(1.0);
    match color {
        Color::Rgb(r, g, b) => Color::Rgb(
            (r as f64 * a) as u8,
            (g as f64 * a) as u8,
            (b as f64 * a) as u8,
        ),
        _ => color,
    }
}

#[allow(dead_code)]
fn rand_f64() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as f64;
    (nanos / 1_000_000_000.0).fract()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_params() -> FaceParams {
        FaceParams::from_signals(0.0, 0.0, 0.3, 0.0)
    }

    #[test]
    fn renderer_creates_with_canvas_backend() {
        let r = FaceRenderer::new(RendererBackend::Canvas);
        assert_eq!(r.backend(), RendererBackend::Canvas);
    }

    #[test]
    fn renderer_output_contains_face_prefix() {
        let r = FaceRenderer::new(RendererBackend::Canvas);
        let out = r.render(&test_params());
        assert!(out.starts_with("FACE"));
        assert!(out.contains("mouth="));
        assert!(out.contains("smile="));
    }

    #[test]
    fn backend_detection_prefers_env_var() {
        std::env::set_var("KITTY_WINDOW_ID", "1");
        assert_eq!(RendererBackend::detect(), RendererBackend::Kitty);
        std::env::remove_var("KITTY_WINDOW_ID");
    }

    #[test]
    fn backend_from_string_maps_correctly() {
        assert_eq!(RendererBackend::from_str("kitty"), RendererBackend::Kitty);
        assert_eq!(RendererBackend::from_str("sixel"), RendererBackend::Sixel);
        assert_eq!(RendererBackend::from_str("canvas"), RendererBackend::Canvas);
        assert_eq!(
            RendererBackend::from_str("unknown"),
            RendererBackend::Canvas
        );
    }
}
