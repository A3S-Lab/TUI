use crate::terminal::Terminal;
use std::io;
use std::time::{Duration, Instant};

pub struct Renderer {
    last_lines: Vec<String>,
    last_render: Instant,
    frame_duration: Duration,
    first_render: bool,
}

impl Renderer {
    pub fn new(fps: u32) -> Self {
        let fps = fps.clamp(1, 120);
        Self {
            last_lines: Vec::new(),
            last_render: Instant::now() - Duration::from_secs(1),
            frame_duration: Duration::from_secs_f64(1.0 / fps as f64),
            first_render: true,
        }
    }

    pub fn render(&mut self, terminal: &mut Terminal, view: &str) -> io::Result<()> {
        if self.first_render {
            terminal.draw(view)?;
            self.first_render = false;
        } else {
            let new_lines: Vec<String> = view.lines().map(|l| l.to_string()).collect();
            self.diff_render(terminal, &new_lines)?;
        }
        self.last_lines = view.lines().map(|l| l.to_string()).collect();
        self.last_render = Instant::now();
        Ok(())
    }

    pub fn render_if_changed(
        &mut self,
        terminal: &mut Terminal,
        view: &str,
    ) -> io::Result<()> {
        let new_lines: Vec<&str> = view.lines().collect();
        let same = new_lines.len() == self.last_lines.len()
            && new_lines
                .iter()
                .zip(self.last_lines.iter())
                .all(|(a, b)| *a == b.as_str());

        if same {
            return Ok(());
        }
        if self.last_render.elapsed() < self.frame_duration {
            return Ok(());
        }
        self.render(terminal, view)
    }

    fn diff_render(&self, terminal: &mut Terminal, new_lines: &[String]) -> io::Result<()> {
        let max_rows = new_lines.len().max(self.last_lines.len());

        for row in 0..max_rows {
            let new_line = new_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let old_line = self.last_lines.get(row).map(|s| s.as_str()).unwrap_or("");

            if new_line != old_line {
                terminal.draw_line(row as u16, new_line)?;
            }
        }

        terminal.flush()
    }
}
