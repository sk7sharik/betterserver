use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct BlackRing;
impl Entity for BlackRing
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_BRING_STATE);
        packet.wu8(0);
        packet.wu16(*id);
        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_BRING_STATE);
        packet.wu8(1);
        packet.wu16(*id);
        Some(packet)
    }

    fn id(&self) -> &str {
        "blackring"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}