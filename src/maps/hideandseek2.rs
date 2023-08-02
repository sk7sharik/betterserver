use crate::{map::Map, states::game::Game, server::Server};

pub(crate) struct HideAndSeek2
{

}

impl Map for HideAndSeek2
{
    fn name(&self) -> &str {
        "Hide and Seek 2"
    }

    fn index(&self) -> usize {
        0
    }

    fn init(&mut self, server: &mut Server, game: &mut Game) {
        todo!()
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game) {
        todo!()
    }
}

impl HideAndSeek2
{
    pub fn new() -> HideAndSeek2 {
        HideAndSeek2 { }
    }
}