use crate::cmd::{Cmd, CmdResult};
use crate::event::Event;
use crate::model::Model;
use crate::renderer::Renderer;
use crate::terminal::{Terminal, TerminalOptions};

use crossterm::event::EventStream;
use futures_util::StreamExt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct ProgramBuilder<M: Model> {
    model: M,
    alt_screen: bool,
    mouse_support: bool,
    fps: u32,
}

impl<M: Model> ProgramBuilder<M>
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
        Program::run_inner(
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

pub struct Program;

impl Program {
    pub async fn run<M: Model>(model: M) -> io::Result<()>
    where
        M::Msg: From<Event>,
    {
        Self::run_inner(model, TerminalOptions::default(), 60).await
    }

    async fn run_inner<M: Model>(mut model: M, options: TerminalOptions, fps: u32) -> io::Result<()>
    where
        M::Msg: From<Event>,
    {
        let mut terminal = Terminal::new(&options)?;
        terminal.enter()?;

        let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<M::Msg>();
        let quit_flag = Arc::new(AtomicBool::new(false));

        if let Some(cmd) = model.init() {
            Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
        }

        let mut event_stream = EventStream::new();
        let mut renderer = Renderer::new(fps);

        let view = model.view();
        renderer.render(&mut terminal, &view)?;
        // Place the cursor on the first frame too, so the input is focused
        // immediately (not only after the first key event).
        match model.cursor() {
            Some((col, row)) => terminal.show_cursor_at(col, row)?,
            None => terminal.hide_cursor()?,
        }

        loop {
            if quit_flag.load(Ordering::Relaxed) {
                break;
            }

            // Terminal events (keystrokes) render immediately for responsive
            // input echo; internal messages (e.g. streaming deltas) stay
            // frame-throttled to avoid flicker.
            let mut immediate = false;
            tokio::select! {
                event = event_stream.next() => {
                    match event {
                        Some(Ok(ct_event)) => {
                            immediate = true;
                            // A resize shifts every row — force a full clear+redraw.
                            if matches!(ct_event, crossterm::event::Event::Resize(_, _)) {
                                renderer.invalidate();
                            }
                            let ev: Event = ct_event.into();
                            let msg: M::Msg = ev.into();
                            if let Some(cmd) = model.update(msg) {
                                Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
                            }
                        }
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                Some(msg) = msg_rx.recv() => {
                    if let Some(cmd) = model.update(msg) {
                        Self::dispatch_cmd(cmd, msg_tx.clone(), quit_flag.clone());
                    }
                }
            }

            if quit_flag.load(Ordering::Relaxed) {
                break;
            }

            let view = model.view();
            if immediate {
                renderer.render(&mut terminal, &view)?;
            } else {
                renderer.render_if_changed(&mut terminal, &view)?;
            }
            // Place the real terminal cursor at the model's insertion point (or
            // hide it). Done after rendering so it sits on top of the content.
            match model.cursor() {
                Some((col, row)) => terminal.show_cursor_at(col, row)?,
                None => terminal.hide_cursor()?,
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
                                CmdResult::Quit => {
                                    quit2.store(true, Ordering::Relaxed);
                                }
                                CmdResult::Msg(m) => {
                                    let _ = tx2.send(m);
                                }
                                CmdResult::Batch(inner_cmds) => {
                                    for ic in inner_cmds {
                                        let tx3 = tx2.clone();
                                        let quit3 = quit2.clone();
                                        tokio::spawn(async move {
                                            let r = ic.await;
                                            match r {
                                                CmdResult::Quit => {
                                                    quit3.store(true, Ordering::Relaxed);
                                                }
                                                CmdResult::Msg(m) => {
                                                    let _ = tx3.send(m);
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
                CmdResult::None => {}
            }
        });
    }
}
