use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use log::{info, debug};
use num_traits::FromPrimitive;
use rand::seq::IteratorRandom;
use rand::{thread_rng, Rng};

use crate::map::Map;
use crate::maps::hideandseek2::HideAndSeek2;
use crate::maps::ravinemist::RavineMist;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers, assert_or_disconnect, Player, SurvivorCharacter, ExeCharacter};

use super::game::Game;
use super::lobby::Lobby;

pub(crate) struct CharacterSelect
{
    heartbeat_timer: u8,

    map: Arc<Mutex<dyn Map>>,
    exe: u16
}

impl State for CharacterSelect
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        self.exe = self.choose_exe(server);

        let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXE);
        packet.wu16(self.exe);
        packet.wu16(self.map.lock().unwrap().index() as u16);
        server.multicast_real(&mut packet);

        for peer in real_peers!(server) {
            let mut peer = peer.lock().unwrap();
            peer.player = Some(Player::new()); // new player
            peer.player.as_mut().unwrap().exe = self.exe == peer.id();
            peer.timer = 30;
        }

        let peers = server.peers.read().unwrap();
        let peer = peers.get(&self.exe).unwrap().lock().unwrap();
        info!("{} (ID {}) (c. {}%) is EXE!", peer.nickname, self.exe, peer.exe_chance);
        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        if self.heartbeat_timer >= 60 {
            server.multicast(&mut Packet::new(PacketType::SERVER_HEARTBEAT));

            for peer in real_peers!(server) {
                let mut peer = peer.lock().unwrap();
                let player = peer.player.as_ref().unwrap();
                
                if player.exe && player.ch1 != SurvivorCharacter::None {
                    continue;
                }

                if player.ch2 != ExeCharacter::None {
                    continue;
                }

                peer.timer -= 1;
                if peer.timer <= 0 {
                    peer.disconnect("AFK or timeout.");
                    continue;
                }

                let mut packet = Packet::new(PacketType::SERVER_CHAR_TIME_SYNC);
                packet.wu8(peer.timer as u8);
                peer.send(&mut packet);
            }

            self.heartbeat_timer = 0;
        }

        self.heartbeat_timer += 1;
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

        if real_peers!(server).count() <= 1 {
            return Some(Box::new(Lobby::new()));
        }

        // exe left
        if id == self.exe {
            return Some(Box::new(Lobby::new()));
        }

        self.check_remaining(server)
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Option<Box<dyn State>> 
    {
        let passtrough = packet.ru8() != 0;
        let tp = packet.rpk();
        
        debug!("Got packet {:?}", tp);

        let id = peer.lock().unwrap().id();
        match tp
        {
            // Peer's identity
            PacketType::IDENTITY => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                self.handle_identity(server, &mut peer.lock().unwrap(), packet, false);
            },

            PacketType::CLIENT_REQUEST_CHARACTER => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                assert_or_disconnect!(self.exe != id, &mut peer.lock().unwrap());
                
                let char: SurvivorCharacter = match FromPrimitive::from_u8(packet.ru8())
                {
                    Some(res) => res,
                    None => {
                        peer.lock().unwrap().disconnect("Invalid survivor character requested!");      
                        return None;
                    }
                };

                // Ignore if he already have a character
                if peer.lock().unwrap().player.as_ref().unwrap().ch1 != SurvivorCharacter::None {
                    return None;
                }

                // Set player's character if everything is OK
                let can_have = !real_peers!(server).any(|x| x.lock().unwrap().player.as_ref().unwrap().ch1 == char);
                if can_have {
                    peer.lock().unwrap().player.as_mut().unwrap().ch1 = char;
                    
                    let mut packet = Packet::new(PacketType::SERVER_LOBBY_CHARACTER_CHANGE);
                    packet.wu16(id);
                    packet.wu8(char as u8);
                    server.multicast_real_except(&mut packet, id);
                }

                let mut packet = Packet::new(PacketType::SERVER_LOBBY_CHARACTER_RESPONSE);
                packet.wu8(char as u8);
                packet.wu8(can_have as u8);
                peer.lock().unwrap().send(&mut packet);

                info!("{} (ID {}) chooses [{:?}]", peer.lock().unwrap().nickname, id, char);
                return self.check_remaining(server);
            },

            PacketType::CLIENT_REQUEST_EXECHARACTER => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                assert_or_disconnect!(self.exe == id, &mut peer.lock().unwrap());

                let char: ExeCharacter = match FromPrimitive::from_u8(packet.ru8() - 1)
                {
                    Some(res) => res,
                    None => {
                        peer.lock().unwrap().disconnect("Invalid exe character requested!");      
                        return None;
                    }
                };

                // Ignore if he already have a character
                if peer.lock().unwrap().player.as_ref().unwrap().ch2 != ExeCharacter::None {
                    return None;
                }

                // Set player's character
                peer.lock().unwrap().player.as_mut().unwrap().ch2 = char;

                let mut packet = Packet::new(PacketType::SERVER_LOBBY_CHARACTER_CHANGE);
                packet.wu16(id);
                packet.wu8(char as u8);
                server.multicast_real_except(&mut packet, id);

                let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXECHARACTER_RESPONSE);
                packet.wu8(char as u8);
                peer.lock().unwrap().send(&mut packet);

                info!("{} (ID {}) chooses [{:?}]", peer.lock().unwrap().nickname, id, char);
                return self.check_remaining(server);
            }

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
    pub fn new(map: Arc<Mutex<dyn Map>>) -> CharacterSelect
    {
        CharacterSelect { exe: 0, heartbeat_timer: 0, map }
    }

    fn choose_exe(&mut self, server: &mut Server) -> u16 
    {
        loop {
            for peer in real_peers!(server) {
                let peer = peer.lock().unwrap();
                
                if thread_rng().gen_ratio(peer.exe_chance as u32, 100u32) {
                    return peer.id();
                }
            }
        }
    }

    fn check_remaining(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        if !real_peers!(server).any(|x| {
            let peer = x.lock().unwrap();
            let player = peer.player.as_ref().unwrap();
            
            if player.exe {
                return player.ch2 == ExeCharacter::None
            }
            else {
                return player.ch1 == SurvivorCharacter::None;
            }
         }) {
            return Some(Box::new(Game::new(self.map.clone())));
        }

        None
    }
}