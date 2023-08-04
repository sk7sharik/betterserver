use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct ExellerClone
{
    pub owner_id: u16,
    pub dir: i8,

    pub x: u16,
    pub y: u16
}

impl Entity for ExellerClone
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_EXELLERCLONE_STATE);
        packet.wu8(0);
        packet.wu16(*id);
        packet.wu16(self.owner_id);
        packet.wu16(self.x);
        packet.wu16(self.y);
        packet.wi8(self.dir);

        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_EXELLERCLONE_STATE);
        packet.wu8(1);
        packet.wu16(*id);

        Some(packet)
    }

    fn id(&self) -> &str {
        "exclone"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}