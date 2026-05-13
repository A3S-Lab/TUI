//! Commands for side effects in the TEA update cycle.
//!
//! Commands are async futures that produce messages. They allow the update function
//! to trigger side effects (timers, I/O, etc.) without blocking the event loop.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// An async command that produces a [`CmdResult`] when completed.
pub type Cmd<M> = Pin<Box<dyn Future<Output = CmdResult<M>> + Send>>;

#[doc(hidden)]
pub enum CmdResult<M> {
    Msg(M),
    Batch(Vec<Cmd<M>>),
    Quit,
    None,
}

/// Create a command from an async closure that produces a message.
pub fn cmd<M, F, Fut>(f: F) -> Cmd<M>
where
    M: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = M> + Send + 'static,
{
    Box::pin(async move { CmdResult::Msg(f().await) })
}

/// Create a command that immediately produces a message.
pub fn msg<M: Send + 'static>(m: M) -> Cmd<M> {
    Box::pin(async move { CmdResult::Msg(m) })
}

/// Run multiple commands concurrently.
pub fn batch<M: Send + 'static>(cmds: Vec<Cmd<M>>) -> Cmd<M> {
    Box::pin(async move { CmdResult::Batch(cmds) })
}

/// Run multiple commands in sequence.
pub fn sequence<M: Send + 'static>(cmds: Vec<Cmd<M>>) -> Cmd<M> {
    Box::pin(async move { CmdResult::Batch(cmds) })
}

/// Produce a message after a delay.
pub fn tick<M: Send + 'static>(duration: Duration, m: M) -> Cmd<M> {
    Box::pin(async move {
        tokio::time::sleep(duration).await;
        CmdResult::Msg(m)
    })
}

/// Quit the program.
pub fn quit<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::Quit })
}

/// A no-op command that does nothing.
pub fn none<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::None })
}
