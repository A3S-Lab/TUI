use crate::cmd::Cmd;

pub trait Model: Send + 'static {
    type Msg: Send + 'static;

    fn init(&mut self) -> Option<Cmd<Self::Msg>> {
        None
    }

    fn update(&mut self, msg: Self::Msg) -> Option<Cmd<Self::Msg>>;

    fn view(&self) -> String;
}
