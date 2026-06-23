//! Animation system for smooth transitions and timed effects.
//!
//! Provides [`Transition`] for animating between states with easing functions.

use std::time::{Duration, Instant};

/// Easing function type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Slow start, fast end.
    EaseIn,
    /// Fast start, slow end.
    EaseOut,
    /// Slow start and end, fast middle.
    EaseInOut,
}

impl Easing {
    fn apply(&self, t: f64) -> f64 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
        }
    }
}

/// A value that transitions smoothly from one state to another.
///
/// ```rust
/// use a3s_tui::animation::{Transition, Easing};
/// use std::time::Duration;
///
/// let mut t = Transition::new(0.0);
/// t.animate_to(100.0, Duration::from_millis(300), Easing::EaseOut);
/// // Call t.tick() each frame to advance the animation
/// ```
#[derive(Debug, Clone)]
pub struct Transition {
    from: f64,
    to: f64,
    current: f64,
    duration: Duration,
    easing: Easing,
    started_at: Option<Instant>,
}

impl Transition {
    /// Create a new transition at the given initial value.
    pub fn new(value: f64) -> Self {
        Self {
            from: value,
            to: value,
            current: value,
            duration: Duration::from_millis(200),
            easing: Easing::Linear,
            started_at: None,
        }
    }

    /// Start animating to a new target value.
    pub fn animate_to(&mut self, target: f64, duration: Duration, easing: Easing) {
        self.from = self.current;
        self.to = target;
        self.duration = duration;
        self.easing = easing;
        self.started_at = Some(Instant::now());
    }

    /// Advance the animation. Call once per frame.
    pub fn tick(&mut self) {
        let Some(started) = self.started_at else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed >= self.duration {
            self.current = self.to;
            self.started_at = None;
        } else {
            let t = elapsed.as_secs_f64() / self.duration.as_secs_f64();
            let eased = self.easing.apply(t);
            self.current = self.from + (self.to - self.from) * eased;
        }
    }

    /// Get the current interpolated value.
    pub fn value(&self) -> f64 {
        self.current
    }

    /// Whether the animation is still running.
    pub fn is_animating(&self) -> bool {
        self.started_at.is_some()
    }

    /// Jump immediately to a value without animation.
    pub fn set(&mut self, value: f64) {
        self.from = value;
        self.to = value;
        self.current = value;
        self.started_at = None;
    }
}

/// A looping frame-based animation (e.g., for spinners, progress indicators).
///
/// ```rust
/// use a3s_tui::animation::FrameAnimation;
/// use std::time::Duration;
///
/// let anim = FrameAnimation::new(
///     vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
///     Duration::from_millis(80),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct FrameAnimation {
    frames: Vec<String>,
    interval: Duration,
    current: usize,
    last_tick: Option<Instant>,
    active: bool,
}

impl FrameAnimation {
    /// Create a new frame animation with the given frames and interval.
    pub fn new(frames: Vec<impl Into<String>>, interval: Duration) -> Self {
        Self {
            frames: frames.into_iter().map(|f| f.into()).collect(),
            interval,
            current: 0,
            last_tick: None,
            active: true,
        }
    }

    /// Advance to the next frame if enough time has passed.
    pub fn tick(&mut self) {
        if !self.active || self.frames.is_empty() {
            return;
        }
        let now = Instant::now();
        match self.last_tick {
            None => {
                self.last_tick = Some(now);
            }
            Some(last) if now.duration_since(last) >= self.interval => {
                self.current = (self.current + 1) % self.frames.len();
                self.last_tick = Some(now);
            }
            _ => {}
        }
    }

    /// Get the current frame content.
    pub fn frame(&self) -> &str {
        &self.frames[self.current]
    }

    /// Get the current frame index.
    pub fn index(&self) -> usize {
        self.current
    }

    pub fn start(&mut self) {
        self.active = true;
        self.last_tick = None;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn reset(&mut self) {
        self.current = 0;
        self.last_tick = None;
    }
}

/// Predefined spinner frame sets.
pub mod spinners {
    use super::FrameAnimation;
    use std::time::Duration;

    pub fn dots() -> FrameAnimation {
        FrameAnimation::new(
            vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            Duration::from_millis(80),
        )
    }

    pub fn line() -> FrameAnimation {
        FrameAnimation::new(vec!["-", "\\", "|", "/"], Duration::from_millis(130))
    }

    pub fn bounce() -> FrameAnimation {
        FrameAnimation::new(
            vec!["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"],
            Duration::from_millis(120),
        )
    }

    pub fn pulse() -> FrameAnimation {
        FrameAnimation::new(
            vec!["█", "▓", "▒", "░", "▒", "▓"],
            Duration::from_millis(150),
        )
    }

    pub fn arrow() -> FrameAnimation {
        FrameAnimation::new(
            vec!["←", "↖", "↑", "↗", "→", "↘", "↓", "↙"],
            Duration::from_millis(100),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_linear() {
        assert_eq!(Easing::Linear.apply(0.0), 0.0);
        assert_eq!(Easing::Linear.apply(0.5), 0.5);
        assert_eq!(Easing::Linear.apply(1.0), 1.0);
    }

    #[test]
    fn easing_ease_in() {
        assert_eq!(Easing::EaseIn.apply(0.0), 0.0);
        assert_eq!(Easing::EaseIn.apply(1.0), 1.0);
        assert!(Easing::EaseIn.apply(0.5) < 0.5);
    }

    #[test]
    fn easing_ease_out() {
        assert_eq!(Easing::EaseOut.apply(0.0), 0.0);
        assert_eq!(Easing::EaseOut.apply(1.0), 1.0);
        assert!(Easing::EaseOut.apply(0.5) > 0.5);
    }

    #[test]
    fn transition_immediate_set() {
        let mut t = Transition::new(10.0);
        assert_eq!(t.value(), 10.0);
        assert!(!t.is_animating());
        t.set(50.0);
        assert_eq!(t.value(), 50.0);
        assert!(!t.is_animating());
    }

    #[test]
    fn transition_animate_starts() {
        let mut t = Transition::new(0.0);
        t.animate_to(100.0, Duration::from_millis(100), Easing::Linear);
        assert!(t.is_animating());
    }

    #[test]
    fn transition_completes_after_duration() {
        let mut t = Transition::new(0.0);
        t.animate_to(100.0, Duration::from_millis(1), Easing::Linear);
        std::thread::sleep(Duration::from_millis(5));
        t.tick();
        assert_eq!(t.value(), 100.0);
        assert!(!t.is_animating());
    }

    #[test]
    fn frame_animation_cycles() {
        let mut anim = FrameAnimation::new(vec!["a", "b", "c"], Duration::from_millis(1));
        assert_eq!(anim.frame(), "a");
        anim.tick(); // initializes last_tick
        std::thread::sleep(Duration::from_millis(5));
        anim.tick();
        assert_eq!(anim.frame(), "b");
        std::thread::sleep(Duration::from_millis(5));
        anim.tick();
        assert_eq!(anim.frame(), "c");
        std::thread::sleep(Duration::from_millis(5));
        anim.tick();
        assert_eq!(anim.frame(), "a"); // wraps
    }

    #[test]
    fn frame_animation_stop_start() {
        let mut anim = FrameAnimation::new(vec!["x", "y"], Duration::from_millis(1));
        anim.stop();
        assert!(!anim.is_active());
        anim.tick();
        std::thread::sleep(Duration::from_millis(5));
        anim.tick();
        assert_eq!(anim.frame(), "x"); // didn't advance
        anim.start();
        anim.tick();
        std::thread::sleep(Duration::from_millis(5));
        anim.tick();
        assert_eq!(anim.frame(), "y");
    }

    #[test]
    fn spinners_create_valid_animations() {
        let dots = spinners::dots();
        assert_eq!(dots.frame(), "⠋");
        let line = spinners::line();
        assert_eq!(line.frame(), "-");
    }
}
