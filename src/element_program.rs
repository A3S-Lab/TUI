//! Element-based program runner with Flexbox layout and incremental rendering.
//!
//! This module provides [`ElementProgramBuilder`] for running applications that
//! use the [`ElementModel`] trait with automatic layout computation and diff-based
//! terminal rendering.

use crate::cmd::{Cmd, CmdResult};
use crate::diff::DiffRenderer;
use crate::event::Event;
use crate::layout_engine::LayoutEngine;
use crate::model::ElementModel;
use crate::paint;
use crate::terminal::{Terminal, TerminalOptions};

use crossterm::event::EventStream;
use futures_util::StreamExt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Builder for configuring and running an element-based TUI program.
///
/// ```rust,no_run
/// # use a3s_tui::{ElementProgramBuilder, ElementModel, Element, Event, cmd::Cmd};
/// # struct App;
/// # enum Msg {}
/// # impl From<Event> for Msg { fn from(_: Event) -> Self { todo!() } }
/// # impl ElementModel for App {
/// #     type Msg = Msg;
/// #     fn update(&mut self, _: Msg) -> Option<Cmd<Msg>> { None }
/// #     fn view(&self) -> Element<Msg> { todo!() }
/// # }
/// # async fn run() -> std::io::Result<()> {
/// ElementProgramBuilder::new(App)
///     .with_alt_screen()
///     .with_fps(30)
///     .run()
///     .await
/// # }
/// ```
pub struct ElementProgramBuilder<M: ElementModel> {
    model: M,
    alt_screen: bool,
    mouse_support: bool,
    fps: u32,
}

impl<M: ElementModel> ElementProgramBuilder<M>
where
    M::Msg: From<Event>,
{
    pub fn new(model: M) -> Self {
        Self {
            model,
            alt_screen: true,
            mouse_support: false,
            fps: 60,
        }
    }

    pub fn with_alt_screen(mut self) -> Self {
        self.alt_screen = true;
        self
    }

    pub fn without_alt_screen(mut self) -> Self {
        self.alt_screen = false;
        self
    }

    pub fn with_mouse_support(mut self) -> Self {
        self.mouse_support = true;
        self
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps.clamp(1, 120);
        self
    }

    pub async fn run(self) -> io::Result<()> {
        ElementProgram::run_inner(
            self.model,
            TerminalOptions {
                alt_screen: self.alt_screen,
                mouse_support: self.mouse_support,
                raw_mode: true,
            },
            self.fps,
        )
        .await
    }
}

pub struct ElementProgram;

impl ElementProgram {
    pub async fn run<M: ElementModel>(model: M) -> io::Result<()>
    where
        M::Msg: From<Event>,
    {
        Self::run_inner(model, TerminalOptions::default(), 60).await
    }

    #[allow(unused_assignments)]
    async fn run_inner<M: ElementModel>(
        mut model: M,
        options: TerminalOptions,
        fps: u32,
    ) -> io::Result<()>
    where
        M::Msg: From<Event>,
    {
        let mut terminal = Terminal::new(&options)?;
        terminal.enter()?;

        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<M::Msg>();
        let quit_flag = Arc::new(AtomicBool::new(false));
        let frame_duration = Duration::from_secs_f64(1.0 / fps as f64);
        let mut last_render = Instant::now();
        let mut dirty = false;

        if let Some(cmd) = model.init() {
            Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
        }

        let mut event_stream = EventStream::new();
        let mut layout_engine = LayoutEngine::new();
        let mut diff_renderer = DiffRenderer::new();

        let (width, height) = Terminal::size().unwrap_or((80, 24));

        {
            let element = model.view();
            let layout = layout_engine.compute(&element, width, height);
            let grid = paint::paint(&element, &layout, width, height);
            diff_renderer.render(&mut terminal, grid)?;
        }

        loop {
            if quit_flag.load(Ordering::Relaxed) {
                break;
            }

            tokio::select! {
                event = event_stream.next() => {
                    match event {
                        Some(Ok(ct_event)) => {
                            let ev: Event = ct_event.into();
                            let msg: M::Msg = ev.into();
                            if let Some(cmd) = model.update(msg) {
                                Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
                            }
                            dirty = true;
                        }
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                Some(msg) = msg_rx.recv() => {
                    if let Some(cmd) = model.update(msg) {
                        Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
                    }
                    dirty = true;
                }
            }

            if quit_flag.load(Ordering::Relaxed) {
                break;
            }

            if dirty && last_render.elapsed() >= frame_duration {
                let (w, h) = Terminal::size().unwrap_or((width, height));
                let element = model.view();
                let layout = layout_engine.compute(&element, w, h);
                let grid = paint::paint(&element, &layout, w, h);
                diff_renderer.render(&mut terminal, grid)?;
                last_render = Instant::now();
                dirty = false;
            }
        }

        terminal.exit()?;
        std::mem::forget(terminal);
        Ok(())
    }

    fn dispatch_cmd<M: Send + 'static>(
        cmd: Cmd<M>,
        tx: mpsc::UnboundedSender<M>,
        quit: Arc<AtomicBool>,
    ) {
        tokio::spawn(async move {
            let result = cmd.await;
            match result {
                CmdResult::Quit => {
                    quit.store(true, Ordering::Relaxed);
                }
                CmdResult::Msg(m) => {
                    let _ = tx.send(m);
                }
                CmdResult::Batch(cmds) => {
                    for c in cmds {
                        let tx2 = tx.clone();
                        let quit2 = quit.clone();
                        tokio::spawn(async move {
                            let r = c.await;
                            match r {
                                CmdResult::Quit => quit2.store(true, Ordering::Relaxed),
                                CmdResult::Msg(m) => {
                                    let _ = tx2.send(m);
                                }
                                _ => {}
                            }
                        });
                    }
                }
                CmdResult::None => {}
            }
        });
    }
}
