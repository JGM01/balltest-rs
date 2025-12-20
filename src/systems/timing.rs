use std::time::{Duration, Instant};

pub struct TimeSystem {
    // Core
    pub sim_time: Duration,
    last_update: Instant,
    sim_accumulator: Duration,
    fps_timer: Instant,
    fps_frame_count: u32,
    pub current_fps: u32,

    // Constants
    sim_dt: Duration,
    fps_dt: Duration,

    // Flags/States
    paused: bool,
    scale: f32,
}

impl TimeSystem {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            sim_time: Duration::ZERO,
            last_update: now,
            sim_accumulator: Duration::ZERO,
            fps_timer: now,
            fps_frame_count: 0,
            current_fps: 0,
            sim_dt: Duration::from_millis(8), // 125 Hz
            fps_dt: Duration::from_secs(1),
            paused: false,
            scale: 1.0,
        }
    }

    pub fn sim_dt(&self) -> Duration {
        self.sim_dt
    }

    /// Advances time and returns:
    /// - number of fixed simulation steps to run
    /// - optional FPS update (once per fps_dt)
    /// - whether a redraw is justified
    /// - interpolation alpha for rendering (0.0–1.0)
    pub fn tick(&mut self, now: Instant) -> (u32, Option<u32>, bool, f32) {
        let mut frame_dt = now - self.last_update;
        self.last_update = now;

        if self.paused {
            self.sim_accumulator = Duration::ZERO;
            return (0, None, false, 0.0);
        }

        let max_frame_dt = self.sim_dt * 5;
        frame_dt = frame_dt.min(max_frame_dt);

        self.sim_accumulator += frame_dt.mul_f32(self.scale);

        let mut sim_steps = 0;
        while self.sim_accumulator >= self.sim_dt {
            self.sim_accumulator -= self.sim_dt;
            self.sim_time += self.sim_dt;
            sim_steps += 1;
        }

        self.fps_frame_count += 1;
        let mut fps_update = None;

        if now - self.fps_timer >= self.fps_dt {
            let elapsed = (now - self.fps_timer).as_secs_f32();
            let fps = (self.fps_frame_count as f32 / elapsed).round() as u32;

            self.current_fps = fps;
            self.fps_frame_count = 0;
            self.fps_timer = now;

            fps_update = Some(fps);
        }

        let alpha =
            (self.sim_accumulator.as_secs_f32() / self.sim_dt.as_secs_f32()).clamp(0.0, 1.0);

        let needs_redraw = sim_steps > 0 || fps_update.is_some();

        (sim_steps, fps_update, needs_redraw, alpha)
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;

        if !self.paused {
            let now = Instant::now();
            self.last_update = now;
            self.fps_timer = now;
            self.sim_accumulator = Duration::ZERO;
        }
    }

    pub fn next_wakeup(&self) -> Instant {
        self.last_update + self.sim_dt
    }
}
