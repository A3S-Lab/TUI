use crate::cmd::{Cmd, CmdResult};
use crate::event::Event;
use crate::model::Model;
use crate::renderer::Renderer;
use crate::terminal::{Terminal, TerminalOptions};

use crossterm::event::EventStream;
use futures_util::StreamExt;
use std::io;
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
        Program::run_inner(self.model, TerminalOptions {
            alt_screen: self.alt_screen,
            mouse_support: self.mouse_support,
            raw_mode: true,
        }, self.fps).await
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

    async fn run_inner<M: Model>(
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

        if let Some(cmd) = model.init() {
            Self::spawn_cmd(cmd, msg_tx.clone());
        }

        let mut event_stream = EventStream::new();
        let mut renderer = Renderer::new(fps);

        let view = model.view();
        renderer.render(&mut terminal, &view)?;

        let mut should_quit = false;

        while !should_quit {
            tokio::select! {
                event = event_stream.next() => {
                    match event {
                        Some(Ok(ct_event)) => {
                            let ev: Event = ct_event.into();
                            let msg: M::Msg = ev.into();
                            if let Some(cmd) = model.update(msg) {
                                should_quit = Self::process_cmd(cmd, &msg_tx).await;
                            }
                        }
                        Some(Err(_)) => break,
                        None => break,
                    }
                }
                Some(msg) = msg_rx.recv() => {
                    if let Some(cmd) = model.update(msg) {
                        should_quit = Self::process_cmd(cmd, &msg_tx).await;
                    }
                }
            }

            if !should_quit {
                let view = model.view();
                renderer.render_if_changed(&mut terminal, &view)?;
            }
        }

        terminal.exit()?;
        std::mem::forget(terminal);
        Ok(())
    }

    async fn process_cmd<M: Send + 'static>(
        cmd: Cmd<M>,
        tx: &mpsc::UnboundedSender<M>,
    ) -> bool {
        let result = cmd.await;
        match result {
            CmdResult::Quit => true,
            CmdResult::Msg(m) => {
                let _ = tx.send(m);
                false
            }
            CmdResult::Batch(cmds) => {
                for c in cmds {
                    Self::spawn_cmd(c, tx.clone());
                }
                false
            }
            CmdResult::None => false,
        }
    }

    fn spawn_cmd<M: Send + 'static>(
        cmd: Cmd<M>,
        tx: mpsc::UnboundedSender<M>,
    ) {
        tokio::spawn(async move {
            let result = cmd.await;
            match result {
                CmdResult::Msg(m) => {
                    let _ = tx.send(m);
                }
                CmdResult::Batch(cmds) => {
                    for c in cmds {
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            let r = c.await;
                            if let CmdResult::Msg(m) = r {
                                let _ = tx2.send(m);
                            }
                        });
                    }
                }
                CmdResult::Quit | CmdResult::None => {}
            }
        });
    }
}
