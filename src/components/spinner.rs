use std::time::Duration;

use crate::cmd::{self, Cmd};
use crate::element::{Element, TextElement};
use crate::style::Color;

pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    title: String,
    active: bool,
    color: Color,
}

#[derive(Debug, Clone)]
pub struct SpinnerTick;

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: vec![
                "\u{28cb}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}",
                "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}",
            ],
            current: 0,
            title: String::new(),
            active: true,
            color: Color::Cyan,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_frames(mut self, frames: Vec<&'static str>) -> Self {
        self.frames = frames;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn tick(&mut self) {
        if self.active {
            self.current = (self.current + 1) % self.frames.len();
        }
    }

    pub fn tick_cmd<M: From<SpinnerTick> + Send + 'static>() -> Cmd<M> {
        cmd::tick(Duration::from_millis(80), SpinnerTick.into())
    }

    pub fn start(&mut self) {
        self.active = true;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn element<Msg>(&self) -> Element<Msg> {
        if self.active {
            let text = format!("{} {}", self.frames[self.current], self.title);
            Element::Text(TextElement::new(text).fg(self.color))
        } else {
            Element::Text(TextElement::new(&self.title))
        }
    }

    pub fn view(&self) -> String {
        if self.active {
            format!("{} {}", self.frames[self.current], self.title)
        } else {
            self.title.clone()
        }
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}
