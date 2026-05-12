use crate::terminal::Terminal;
use std::io;
use std::time::{Duration, Instant};

pub struct Renderer {
    last_view: String,
    last_render: Instant,
    frame_duration: Duration,
}

impl Renderer {
    pub fn new(fps: u32) -> Self {
        let fps = fps.clamp(1, 120);
        Self {
            last_view: String::new(),
            last_render: Instant::now() - Duration::from_secs(1),
            frame_duration: Duration::from_secs_f64(1.0 / fps as f64),
        }
    }

    pub fn render(&mut self, terminal: &mut Terminal, view: &str) -> io::Result<()> {
        terminal.draw(view)?;
        self.last_view = view.to_string();
        self.last_render = Instant::now();
        Ok(())
    }

    pub fn render_if_changed(
        &mut self,
        terminal: &mut Terminal,
        view: &str,
    ) -> io::Result<()> {
        if view == self.last_view {
            return Ok(());
        }
        if self.last_render.elapsed() < self.frame_duration {
            return Ok(());
        }
        self.render(terminal, view)
    }
}
