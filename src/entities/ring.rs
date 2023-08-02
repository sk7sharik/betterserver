use log::warn;
use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

pub(crate) struct Ring
{
    pub red: bool,
    pub id: usize,
    pub uid: u16
}

impl Entity for Ring
{
    fn spawn(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        self.uid = *id;
        self.id = match game.rings.iter().position(|x| !x)
        {
            Some(res) => res,
            None => {
                warn!("Couldn't find free space for a ring");
                return None;
            }
        };
        game.rings[self.id] = true;

        if game.map.lock().unwrap().spawn_red_rings() {
            self.red = thread_rng().gen_bool(1.0 / 10.0);
        }
        
        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(0);
        packet.wu8(self.id as u8);
        packet.wu16(self.uid);
        packet.wu8(self.red as u8);
        Some(packet)
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        None
    }

    fn destroy(&mut self, server: &mut Server, game: &mut Game, id: &u16) -> Option<Packet> 
    {
        game.rings[self.id] = false;

        let mut packet = Packet::new(PacketType::SERVER_RING_STATE);
        packet.wu8(1);
        packet.wu8(self.id as u8);
        packet.wu16(self.uid);
        Some(packet)
    }

    fn id(&self) -> &str {
        "ring"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Ring
{
    pub fn new() -> Ring {
        Ring { red: false, id: 0, uid: 0 }
    }
}