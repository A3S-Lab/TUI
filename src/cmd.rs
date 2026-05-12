use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type Cmd<M> = Pin<Box<dyn Future<Output = CmdResult<M>> + Send>>;

#[doc(hidden)]
pub enum CmdResult<M> {
    Msg(M),
    Batch(Vec<Cmd<M>>),
    Quit,
    None,
}

pub fn cmd<M, F, Fut>(f: F) -> Cmd<M>
where
    M: Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = M> + Send + 'static,
{
    Box::pin(async move { CmdResult::Msg(f().await) })
}

pub fn msg<M: Send + 'static>(m: M) -> Cmd<M> {
    Box::pin(async move { CmdResult::Msg(m) })
}

pub fn batch<M: Send + 'static>(cmds: Vec<Cmd<M>>) -> Cmd<M> {
    Box::pin(async move { CmdResult::Batch(cmds) })
}

pub fn sequence<M: Send + 'static>(cmds: Vec<Cmd<M>>) -> Cmd<M> {
    Box::pin(async move { CmdResult::Batch(cmds) })
}

pub fn tick<M: Send + 'static>(duration: Duration, m: M) -> Cmd<M> {
    Box::pin(async move {
        tokio::time::sleep(duration).await;
        CmdResult::Msg(m)
    })
}

pub fn quit<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::Quit })
}

pub fn none<M: Send + 'static>() -> Cmd<M> {
    Box::pin(async move { CmdResult::None })
}
