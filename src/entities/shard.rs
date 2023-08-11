use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct Shard
{
    pub x: u16,
    pub y: u16,
    pub spawned: bool
}

impl Entity for Shard
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {        
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(self.spawned as u8);
        packet.wu16(*id);
        packet.wu16(self.x);
        packet.wu16(self.y);
        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        None
    }

    fn id(&self) -> &str {
        "shard"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}