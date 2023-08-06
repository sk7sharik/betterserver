use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct CreamRing
{
    pub red: bool,
    pub x: i16,
    pub y: i16
}

impl Entity for CreamRing
{
    fn spawn(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(2);
        packet.wi16(self.x);
        packet.wi16(self.y);
        packet.wu8(255);
        packet.wu16(*id);
        packet.wu8(self.red as u8);
        Some(packet)
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(1);
        packet.wu8(255);
        packet.wu16(*id);
        Some(packet)
    }

    fn id(&self) -> &str {
        "creamring"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}