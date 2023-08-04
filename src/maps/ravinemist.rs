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

    fn ring_count(&self) -> usize {
        27
    }
}

impl RavineMist
{
    pub fn new() -> RavineMist {
        RavineMist { }
    }
}