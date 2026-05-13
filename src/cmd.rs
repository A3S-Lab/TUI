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

/// Alias for [`tick`] — produce a message after a delay.
pub fn delay<M: Send + 'static>(duration: Duration, m: M) -> Cmd<M> {
    tick(duration, m)
}

/// Run an async operation and map its result to a message.
///
/// Useful for I/O operations like HTTP requests or file reads.
pub fn perform<M, F, Fut>(f: F) -> Cmd<M>
where
    M: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = M> + Send + 'static,
{
    Box::pin(async move { CmdResult::Msg(f().await) })
}

/// Quit the program.
pub fn quit<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::Quit })
}

/// A no-op command that does nothing.
pub fn none<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn msg_produces_message() {
        let cmd = msg::<String>("hello".to_string());
        match cmd.await {
            CmdResult::Msg(m) => assert_eq!(m, "hello"),
            _ => panic!("expected Msg"),
        }
    }

    #[tokio::test]
    async fn quit_produces_quit() {
        let cmd = quit::<()>();
        assert!(matches!(cmd.await, CmdResult::Quit));
    }

    #[tokio::test]
    async fn none_produces_none() {
        let cmd = none::<()>();
        assert!(matches!(cmd.await, CmdResult::None));
    }

    #[tokio::test]
    async fn tick_delays_then_produces() {
        let start = std::time::Instant::now();
        let cmd = tick(Duration::from_millis(10), 42u32);
        match cmd.await {
            CmdResult::Msg(m) => {
                assert_eq!(m, 42);
                assert!(start.elapsed() >= Duration::from_millis(10));
            }
            _ => panic!("expected Msg"),
        }
    }

    #[tokio::test]
    async fn perform_runs_async() {
        let cmd = perform(|| async { 1 + 2 });
        match cmd.await {
            CmdResult::Msg(m) => assert_eq!(m, 3),
            _ => panic!("expected Msg"),
        }
    }

    #[tokio::test]
    async fn batch_produces_batch() {
        let cmds = vec![msg(1), msg(2)];
        let cmd = batch(cmds);
        match cmd.await {
            CmdResult::Batch(b) => assert_eq!(b.len(), 2),
            _ => panic!("expected Batch"),
        }
    }

    #[tokio::test]
    async fn cmd_from_closure() {
        let c = cmd(|| async { "result".to_string() });
        match c.await {
            CmdResult::Msg(m) => assert_eq!(m, "result"),
            _ => panic!("expected Msg"),
        }
    }
}
