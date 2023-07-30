use std::sync::{Mutex, Arc};

use log::{debug, info};

use crate::{state::State, server::{Server, Peer}, packet::{Packet, PacketType, self}};

const BUILD_VER: u16 = 100;

pub(crate) struct Lobby
{
    timer: u8
}

impl State for Lobby
{
    fn tick(&mut self, server: &mut Server) 
    {
        if server.peers.read().unwrap().len() <= 0 {
            return;
        }

        if self.timer >= 60 {
            for peer in server.peers.write().unwrap().iter_mut() {
                let mut peer = peer.1.lock().unwrap();

                if peer.ready {
                    peer.timer = 0;
                    continue;
                }

                peer.timer += 1;

                if peer.timer >= 30 || (peer.pending && peer.timer >= 5) {
                    peer.disconnect("AFK or timeout");
                    continue;
               }
            }          

            // Heartbeat
            server.multicast(&mut Packet::new(PacketType::SERVER_HEARTBEAT));

            self.timer = 0;
            debug!("Heartbeat + Check");
        }

        self.timer += 1
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) 
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
                let mut peer = peer.lock().unwrap();
                let build_ver = packet.ru16();

                if build_ver != BUILD_VER {
                    peer.disconnect("Version mismatch.");
                    return;
                }

                peer.pending = false; // now he isnt pending
                peer.nickname = packet.rstr();
                peer.lobby_icon = packet.ru8();
                peer.pet = packet.ri8();
                let os_type = packet.ru8();
                peer.udid = packet.rstr();

                debug!("Identity of \"{}\" (ID {}):", peer.nickname, peer.id());
                debug!("OS: {} UDID: {}", os_type, peer.udid);
                self.share_player(server, &mut peer);
                self.accept_player(&mut peer);
            },

            // Peer requests player list
            PacketType::CLIENT_LOBBY_PLAYERS_REQUEST => {
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
                let ready = packet.ru8() != 0;
                
                let mut packet = Packet::new(PacketType::SERVER_LOBBY_READY_STATE);
                packet.wu16(id);
                packet.wu8(ready as u8);
                server.multicast_except(&mut packet, id);

                peer.lock().unwrap().ready = ready;
            },

            // Peer's chat message
            PacketType::CLIENT_CHAT_MESSAGE => {
                // Remulitcast the message
                server.multicast_except(packet, id);

                let _id = packet.ru16(); //TODO: get rid of
                let msg = packet.rstr();

                info!("[{}]: {}", peer.lock().unwrap().nickname, msg);
            },

            _ => {
                debug!("Unrecognized packet {:?}", tp);
            }
        }
    }

    fn connect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) 
    {
        //TODO: queue
        if server.peers.read().unwrap().len() >= 7 {
            peer.lock().unwrap().disconnect("Server is full: 7/7.");
            return;
        }

        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_JOINED);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);
    }

    fn disconnect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) 
    {
        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_LEFT);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);
    }

}

impl Lobby
{
    pub fn new() -> Lobby 
    {
        Lobby { timer: 0 }
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
        let mut packet = Packet::new(PacketType::SERVER_IDENTITY_RESPONSE);
        packet.wu8(1);
        packet.wu16(peer.id());
        peer.send(&mut packet);

        let mut packet = Packet::new(PacketType::SERVER_LOBBY_EXE_CHANCE);
        packet.wu8(1);
        peer.send(&mut packet);
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
}