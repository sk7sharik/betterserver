use std::sync::{Mutex, Arc};

use log::debug;

use crate::map::Map;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers};
use crate::states::lobby::BUILD_VER;

use super::lobby::Lobby;

pub(crate) struct Game
{
    pub map: Arc<dyn Map>
}

impl State for Game
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
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
        if peer.lock().unwrap().in_queue {
            return None;
        }
        
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

        // let id = peer.lock().unwrap().id();
        match tp
        {
            // Peer's identity
            PacketType::IDENTITY => {
                let mut peer = peer.lock().unwrap();
                let build_ver = packet.ru16();

                if build_ver != BUILD_VER {
                    peer.disconnect("Version mismatch.");
                    return None;
                }

                let nickname = packet.rstr();

                if nickname.len() > 15 {
                    peer.nickname = nickname[..16].to_string();
                }
                else {
                    peer.nickname = nickname;
                }

                peer.lobby_icon = packet.ru8();
                peer.pet = packet.ri8();
                let os_type = packet.ru8();
                peer.udid = packet.rstr();

                debug!("Identity of \"{}\" (ID {}):", peer.nickname, peer.id());
                debug!("OS: {} UDID: {}", os_type, peer.udid);

                // We're in game, so put the player in queue
                peer.in_queue = true;
                peer.pending = false;
                
                let mut packet = Packet::new(PacketType::SERVER_IDENTITY_RESPONSE);
                packet.wu8(false as u8);
                packet.wu16(peer.id());
                peer.send(&mut packet);
            },

            _ => {}
        }

        None
    }

    fn name(&self) -> &str { "Game" }
}

impl Game
{
    pub fn new(map: Arc<dyn Map>) -> Game
    {
        Game { map }
    }
}