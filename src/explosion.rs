use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

const PARTICLE_COUNT: usize = 40;
const DURATION_MS: f64 = 300.0;
const GRAVITY: f64 = 800.0; // pixels/s²

/// Simple xorshift64 PRNG (no external dependency)
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self(seed | 1) // ensure non-zero
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Random f64 in [0.0, 1.0)
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Random f64 in [min, max)
    fn range(&mut self, min: f64, max: f64) -> f64 {
        min + self.next_f64() * (max - min)
    }
}

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    size: f64,
    r: f64,
    g: f64,
    b: f64,
    life: f64, // 1.0 → 0.0
}

pub struct ExplosionState {
    particles: Vec<Particle>,
    start_time: Option<Instant>,
    last_tick: Option<Instant>,
}

impl ExplosionState {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            particles: Vec::new(),
            start_time: None,
            last_tick: None,
        }))
    }

    /// Spawn particles at the given pixel coordinates
    pub fn trigger(&mut self, cx: f64, cy: f64) {
        let mut rng = Rng::new();
        self.particles.clear();
        let now = Instant::now();
        self.start_time = Some(now);
        self.last_tick = Some(now);

        // Bright pixel colors for the explosion
        let colors: [(f64, f64, f64); 8] = [
            (1.0, 0.3, 0.2), // red
            (1.0, 0.6, 0.1), // orange
            (1.0, 1.0, 0.2), // yellow
            (0.3, 1.0, 0.3), // green
            (0.2, 0.8, 1.0), // cyan
            (0.5, 0.4, 1.0), // purple
            (1.0, 0.4, 0.8), // pink
            (1.0, 1.0, 1.0), // white
        ];

        for _ in 0..PARTICLE_COUNT {
            let angle = rng.range(0.0, std::f64::consts::TAU);
            let speed = rng.range(80.0, 350.0);
            let ci = (rng.next_u64() as usize) % colors.len();
            let (r, g, b) = colors[ci];
            self.particles.push(Particle {
                x: cx,
                y: cy,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                size: rng.range(3.0, 7.0),
                r,
                g,
                b,
                life: 1.0,
            });
        }
    }

    /// Advance simulation. Returns true if still active.
    pub fn tick(&mut self) -> bool {
        let start = match self.start_time {
            Some(t) => t,
            None => return false,
        };

        let now = Instant::now();
        let elapsed = now.duration_since(start).as_secs_f64();
        let dt = self
            .last_tick
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(1.0 / 60.0);
        self.last_tick = Some(now);

        let progress = elapsed / (DURATION_MS / 1000.0);
        if progress >= 1.0 {
            self.particles.clear();
            self.start_time = None;
            self.last_tick = None;
            return false;
        }

        for p in &mut self.particles {
            p.vx *= (0.97_f64).powf(dt * 60.0); // frame-rate independent friction
            p.vy += GRAVITY * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life = (1.0 - progress).max(0.0);
        }

        true
    }

    /// Draw particles onto a Cairo context
    pub fn draw(&self, cr: &gtk4::cairo::Context) {
        if self.start_time.is_none() {
            return;
        }
        for p in &self.particles {
            let alpha = p.life * p.life; // quadratic fade-out
            cr.set_source_rgba(p.r, p.g, p.b, alpha);
            // Pixelated look: snap to integer coordinates, square shape
            let x = (p.x as i32) as f64;
            let y = (p.y as i32) as f64;
            let s = (p.size as i32) as f64;
            cr.rectangle(x, y, s, s);
            let _ = cr.fill();
        }
    }
}
