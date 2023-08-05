use log::warn;
use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct TailsProjectile
{
    pub owner: u16,
    pub x: i32,
    pub y: i32,
    pub dir: i8,
    pub dmg: u8,
    pub exe: bool,
    pub charge: u8,

    pub timer: u16
}

impl Entity for TailsProjectile
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_TPROJECTILE_STATE);
        packet.wu8(0);
        packet.wu16(self.x as u16);
        packet.wu16(self.y as u16);
        packet.wu16(self.owner);
        packet.wi8(self.dir);
        packet.wu8(self.dmg);
        packet.wu8(self.exe as u8);
        packet.wu8(self.charge);

        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        self.x += (self.dir * 12) as i32;
        
        self.timer -= 1;
        if self.timer <= 0 || self.x <= 0 {
            game.queue_destroy(id);
            return None;
        }

        let mut packet = Packet::new(PacketType::SERVER_TPROJECTILE_STATE);
        packet.wu8(1);
        packet.wu16(self.x as u16);
        packet.wu16(self.y as u16);

        Some(packet)
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, _id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_TPROJECTILE_STATE);
        packet.wu8(2);
        Some(packet)
    }

    fn id(&self) -> &str {
        "tproj"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}