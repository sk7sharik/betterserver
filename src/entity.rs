use std::any::Any;

use crate::{server::Server, states::game::Game, packet::Packet};

pub(crate) trait Entity: Send + Sync
{
    fn spawn(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet>;
    fn tick(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet>;
    fn destroy(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet>;

    fn id(&self) -> &str { "any" }
    
    fn as_any(&self) -> &dyn Any;
}