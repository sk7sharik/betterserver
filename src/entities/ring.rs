use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct Ring
{
    pub red: bool,
    pub id: usize
}

impl Entity for Ring
{
    fn spawn(&mut self, _server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        if game.map.lock().unwrap().spawn_red_rings() {
            self.red = thread_rng().gen_bool(1.0 / 10.0);
        }
        
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(0);
        packet.wu8(self.id as u8);
        packet.wu16(*id);
        packet.wu8(self.red as u8);
        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(1);
        packet.wu8(self.id as u8);
        packet.wu16(*id);
        Some(packet)
    }

    fn id(&self) -> &str {
        "ring"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}