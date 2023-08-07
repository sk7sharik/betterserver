use std::sync::{Mutex, Arc};
use log::{debug, info};
use rand::{thread_rng, Rng};
use crate::{state::State, server::{Server, Peer, real_peers, assert_or_disconnect}, packet::{Packet, PacketType, self}};

use super::mapvote::MapVote;

pub(crate) const BUILD_VER: u16 = 101;

pub(crate) struct Lobby
{
    heartbeat_timer: u8,
    countdown: bool,
    countdown_timer: u16,
}

impl State for Lobby
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>>
    {
        for peer in server.peers.write().unwrap().values_mut() {
            let mut peer = peer.lock().unwrap();
            peer.ready = false;

            if peer.in_queue {
                let mut packet = Packet::new(PacketType::SERVER_IDENTITY_RESPONSE);
                packet.wu8(true as u8);
                packet.wu16(server.udp_port);
                packet.wu16(peer.id());
                peer.send(&mut packet);

                peer.in_queue = false;
            }
            else {
                let mut packet = Packet::new(PacketType::SERVER_GAME_BACK_TO_LOBBY);
                peer.send(&mut packet);
            }

            peer.exe_chance += thread_rng().gen_range(2..10);
            if peer.exe_chance > 99 {
                peer.exe_chance = 99;
            }

            // Send new exe chance
            let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXE_CHANCE);
            packet.wu8(peer.exe_chance);
            peer.send(&mut packet);
        }

        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>>
    {
        {
            self.heartbeat_timer += 1;
            if self.heartbeat_timer >= 60 {
                for peer in server.peers.write().unwrap().iter_mut() {
                    let mut peer = peer.1.lock().unwrap();
                
                    if peer.ready {
                        peer.timer = 0;
                        continue;
                    }
                
                    peer.timer += 1;
                    if peer.timer >= 30 || (peer.pending && peer.timer >= 2) {
                        debug!("[Lobby] Disconnecting... {} (ID {})", peer.nickname, peer.id());
                        peer.disconnect("AFK or timeout");
                        continue;
                   }
                }          
            
                // Heartbeat
                server.multicast_real(&mut Packet::new(PacketType::SERVER_HEARTBEAT));
                self.heartbeat_timer = 0;
            }
        }

        // Handle countdown
        if self.countdown {
            self.countdown_timer -= 1;

            // Set state to map vote
            if self.countdown_timer <= 0 {
                return Some(Box::new(MapVote::new()));
            }

            if self.countdown_timer % 60 == 0 {
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_COUNTDOWN);
                packet.wu8(self.countdown as u8);
                packet.wu8((self.countdown_timer / 60) as u8);
                server.multicast_real(&mut packet);
            }
        }

        None
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Option<Box<dyn State>>
    {
        let passtrough = packet.ru8() != 0; //TODO: get rid of
        let tp = packet.rpk();

        debug!("Got packet {:?}", tp);

        if !peer.lock().unwrap().pending {
            peer.lock().unwrap().timer = 0;
        }

        let id = peer.lock().unwrap().id();
        match tp 
        {
            // Peer's identity
            PacketType::IDENTITY => {
                if self.handle_identity(server, peer.clone(), packet, true) {
                    self.accept_player(&mut peer.lock().unwrap());
                    self.share_player(server, &mut peer.lock().unwrap());
                    self.check_ready(server);
                }
            },

            // Peer requests player list
            PacketType::CLIENT_LOBBY_PLAYERS_REQUEST => {
                {
                    let peer =  &mut peer.lock().unwrap();
                    assert_or_disconnect!(!peer.pending, peer);
                    assert_or_disconnect!(!passtrough, peer);
                }
                
                for plr in server.peers.read().unwrap().iter() {
                    let plr = plr.1.lock().unwrap();

                    if plr.pending {
                        continue;
                    }
                    
                    if plr.id() == id {
                        continue;
                    }

                    let mut packet = Packet::new(PacketType::SERVER_LOBBY_PLAYER);
                    packet.wu16(plr.id());
                    packet.wu8(plr.ready as u8);
                    packet.wstr(&plr.nickname);
                    packet.wu8(plr.lobby_icon);
                    packet.wi8(plr.pet);
                    peer.lock().unwrap().send(&mut packet);
                }

                let mut packet = Packet::new(PacketType::SERVER_LOBBY_CORRECT);
                peer.lock().unwrap().send(&mut packet);
                self.send_message(&mut peer.lock().unwrap(), "type .help for more info");
            },

            // Peer's ready state changed
            PacketType::CLIENT_LOBBY_READY_STATE => {
                {
                    let peer =  &mut peer.lock().unwrap();
                    assert_or_disconnect!(!peer.pending, peer);
                    assert_or_disconnect!(!passtrough, peer);
                }

                let ready = packet.ru8() != 0;
                
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_READY_STATE);
                packet.wu16(id);
                packet.wu8(ready as u8);
                server.multicast_real_except(&mut packet, id);

                peer.lock().unwrap().ready = ready;
                self.check_ready(server);
            },

            // Peer's chat message
            PacketType::CLIENT_CHAT_MESSAGE => {
                {
                    let peer =  &mut peer.lock().unwrap();
                    assert_or_disconnect!(!peer.pending, peer);
                    assert_or_disconnect!(!passtrough, peer);
                }

                // Remulitcast the message
                server.multicast_real_except(packet, id);

                let _id = packet.ru16(); //TODO: get rid of
                let msg = packet.rstr();

                info!("[{}]: {}", peer.lock().unwrap().nickname, msg);
            },

            _ => {
                {
                    let peer =  &mut peer.lock().unwrap();
                    assert_or_disconnect!(!peer.pending, peer);
                }

                debug!("Unrecognized packet {:?}", tp);
            }
        }

        None
    }

    fn connect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        //TODO: queue
        if server.peers.read().unwrap().len() > 7 {
            peer.lock().unwrap().disconnect("Server is full: 7/7.");
            return None;
        }

        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_JOINED);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);
        None
    }

    fn disconnect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_LEFT);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);
        
        if real_peers!(server).count() <= 1 {
            if self.countdown {
                self.countdown = false;
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_COUNTDOWN);
                packet.wu8(self.countdown as u8);
                packet.wu8(0);
                server.multicast(&mut packet);
            }
            return None;
        }

        None
    }


    fn name(&self) -> &str {
        "Lobby"
    }
}

impl Lobby
{
    pub fn new() -> Lobby 
    {
        Lobby { heartbeat_timer: 0, countdown: false, countdown_timer: 0 }
    }

    fn share_player(&mut self, server: &mut Server, peer: &mut Peer) 
    {
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_INFO);
        packet.wu16(peer.id());
        packet.wstr(&peer.nickname);
        packet.wu8(peer.lobby_icon);
        packet.wi8(peer.pet);

        server.multicast_except(&mut packet, peer.id());
    }

    fn accept_player(&mut self, peer: &mut Peer) 
    {
        let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXE_CHANCE);
        packet.wu8(peer.exe_chance);
        peer.send(&mut packet);

        peer.pending = false;
    }

    fn send_message(&mut self, peer: &mut Peer, message: &str)
    {
        let mut packet = Packet::new(PacketType::CLIENT_CHAT_MESSAGE);
        packet.wu16(0);
        packet.wstr(message);
        peer.send(&mut packet);
    }

    fn multicast_message(&mut self, server: &mut Server, message: &str)
    {
        let mut packet = Packet::new(PacketType::CLIENT_CHAT_MESSAGE);
        packet.wu16(0);
        packet.wstr(message);
        server.multicast(&mut packet);
    }

    fn check_ready(&mut self, server: &mut Server) 
    {
        let count = real_peers!(server).count();
        if count <= 1 {
            if self.countdown {
                self.countdown = false;
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_COUNTDOWN);
                packet.wu8(self.countdown as u8);
                packet.wu8(0);
                server.multicast_real(&mut packet);
            }
            return;
        }

        let mut ready_count = 0;
        {
            for peer in real_peers!(server) {
                let peer = peer.lock().unwrap();
                if peer.ready {
                    ready_count += 1;
                }
            }
        }

        if ready_count == count {
            self.countdown = true;
            self.countdown_timer = 60 * 5;

            println!("done");
            let mut packet = Packet::new(PacketType::SERVER_LOBBY_COUNTDOWN);
            packet.wu8(self.countdown as u8);
            packet.wu8((self.countdown_timer / 60) as u8);
            server.multicast_real(&mut packet);
        } else {
            if self.countdown {
                self.countdown = false;

                println!("done");
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_COUNTDOWN);
                packet.wu8(self.countdown as u8);
                packet.wu8((self.countdown_timer / 60) as u8);
                server.multicast_real(&mut packet);
            }
        }
    }
}