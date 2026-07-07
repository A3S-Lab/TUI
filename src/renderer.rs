use crate::style::split_lines_preserving_trailing_blank;
use crate::terminal::{terminal_row, Terminal};
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

    /// Force the next render to be a full clear + redraw (e.g. after a resize,
    /// where row positions shift and a diff would leave artifacts).
    pub fn invalidate(&mut self) {
        self.first_render = true;
    }

    pub fn render(&mut self, terminal: &mut Terminal, view: &str) -> io::Result<()> {
        if self.first_render {
            terminal.draw(view)?;
            self.first_render = false;
        } else {
            let new_lines: Vec<String> = view_lines(view).into_iter().map(str::to_string).collect();
            self.diff_render(terminal, &new_lines)?;
        }
        self.last_lines = view_lines(view).into_iter().map(str::to_string).collect();
        self.last_render = Instant::now();
        Ok(())
    }

    pub fn render_if_changed(&mut self, terminal: &mut Terminal, view: &str) -> io::Result<()> {
        if !self.is_changed(view) {
            return Ok(());
        }
        if !self.is_frame_due() {
            return Ok(());
        }
        self.render(terminal, view)
    }

    pub fn is_changed(&self, view: &str) -> bool {
        let new_lines = view_lines(view);
        new_lines.len() != self.last_lines.len()
            || new_lines
                .iter()
                .zip(self.last_lines.iter())
                .any(|(a, b)| *a != b.as_str())
    }

    pub fn is_frame_due(&self) -> bool {
        self.last_render.elapsed() >= self.frame_duration
    }

    pub fn time_until_next_frame(&self) -> Duration {
        self.frame_duration
            .saturating_sub(self.last_render.elapsed())
    }

    fn diff_render(&self, terminal: &mut Terminal, new_lines: &[String]) -> io::Result<()> {
        let max_rows = new_lines.len().max(self.last_lines.len());

        for row in 0..max_rows {
            let new_line = new_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let old_line = self.last_lines.get(row).map(|s| s.as_str()).unwrap_or("");

            if new_line != old_line {
                let Some(row) = terminal_row(row) else {
                    break;
                };
                terminal.draw_line(row, new_line)?;
            }
        }

        terminal.flush()
    }
}

fn view_lines(view: &str) -> Vec<&str> {
    if view.is_empty() {
        Vec::new()
    } else {
        split_lines_preserving_trailing_blank(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_detects_changed_view_lines() {
        let mut renderer = Renderer::new(60);
        renderer.last_lines = vec!["one".to_string(), "two".to_string()];

        assert!(!renderer.is_changed("one\ntwo"));
        assert!(renderer.is_changed("one\nthree"));
        assert!(renderer.is_changed("one\ntwo\nthree"));
    }

    #[test]
    fn renderer_detects_trailing_blank_view_rows() {
        let mut renderer = Renderer::new(60);
        renderer.last_lines = vec!["one".to_string()];

        assert!(renderer.is_changed("one\n"));

        renderer.last_lines = vec!["one".to_string(), String::new()];
        assert!(!renderer.is_changed("one\n"));
        assert!(renderer.is_changed("one"));
    }

    #[test]
    fn renderer_keeps_empty_view_as_zero_rows() {
        assert!(view_lines("").is_empty());
        assert_eq!(view_lines("one\n"), vec!["one", ""]);
    }

    #[test]
    fn renderer_reports_frame_deadline() {
        let mut renderer = Renderer::new(60);

        renderer.last_render = Instant::now();
        assert!(!renderer.is_frame_due());
        assert!(renderer.time_until_next_frame() <= renderer.frame_duration);

        renderer.last_render = Instant::now() - renderer.frame_duration - Duration::from_millis(1);
        assert!(renderer.is_frame_due());
        assert_eq!(renderer.time_until_next_frame(), Duration::ZERO);
    }
}
