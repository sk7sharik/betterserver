use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rand::{thread_rng, Rng};

use crate::map::Map;
use crate::maps::hideandseek2::HideAndSeek2;
use crate::maps::ravinemist::RavineMist;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers, assert_or_disconnect};

use super::lobby::Lobby;

pub(crate) struct CharacterSelect
{
    map: Arc<dyn Map>,
    exe: u16
}

impl State for CharacterSelect
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        self.exe = self.choose_exe(server);

        let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXE);
        packet.wu16(self.exe);
        packet.wu16(self.map.index() as u16);
        server.multicast_real(&mut packet);

        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        None
    }

    fn connect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>> 
    {
        None
    }

    fn disconnect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>> 
    {
        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_LEFT);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);

        if real_peers!(server).count() <= 2 {
            return Some(Box::new(Lobby::new()));
        }

        None
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Option<Box<dyn State>> 
    {
        let _passtrough = packet.ru8(); //TODO: get rid of
        let tp = packet.rpk();

        if !peer.lock().unwrap().pending {
            peer.lock().unwrap().timer = 0;
        }

        let id = peer.lock().unwrap().id();
        match tp
        {
            // Peer's identity
            PacketType::IDENTITY => {
                self.handle_identity(server, &mut peer.lock().unwrap(), packet, false);
            },

            _ => {}
        }

        None
    }

    fn name(&self) -> &str {
        "Character Select"
    }
}

impl CharacterSelect
{
    pub fn new(map: Arc<dyn Map>) -> CharacterSelect
    {
        CharacterSelect { exe: 0, map }
    }

    fn choose_exe(&mut self, server: &mut Server) -> u16 
    {
        let mut chances: HashMap<u16, u32> = HashMap::new();
        let mut weight: u32 = 0;

        for peer in real_peers!(server) {
            let peer = peer.lock().unwrap();
            weight += peer.exe_chance as u32;

            chances.insert(peer.id(), weight);
        }

        let rnd: f32 = thread_rng().gen_range(0f32..weight as f32);
        for chance in chances {
            if chance.1 as f32 >= rnd {
                return chance.0;
            }
        }

        0
    }
}