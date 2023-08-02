use std::num::Wrapping;

use log::warn;
use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct EggmanTracker
{
    pub x: u16,
    pub y: u16,

    pub activated_by: u16
}

impl Entity for EggmanTracker
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_ETRACKER_STATE);
        packet.wu8(0);
        packet.wu16(*id);
        packet.wu16(self.x as u16);
        packet.wu16(self.y as u16);

        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_ETRACKER_STATE);
        packet.wu8(1);
        packet.wu16(*id);
        packet.wu16(self.activated_by);
        Some(packet)
    }

    fn id(&self) -> &str {
        "eggtrack"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}