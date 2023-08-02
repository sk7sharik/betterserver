use crate::{map::Map, states::game::Game, server::Server};

pub(crate) struct RavineMist
{

}

impl Map for RavineMist
{
    fn name(&self) -> &str {
        "Ravine Mist"
    }
    
    fn index(&self) -> usize {
        1
    }

    fn init(&mut self, server: &mut Server, game: &mut Game) {
        todo!()
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game) {
        todo!()
    }
}

impl RavineMist
{
    pub fn new() -> RavineMist {
        RavineMist { }
    }
}