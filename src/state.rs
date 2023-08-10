use std::{sync::{Arc, Mutex}, net::SocketAddr};

use log::debug;

use crate::{server::{Server, Peer}, packet::{Packet, PacketType}, states::lobby::BUILD_VER};

pub(crate) trait State: Send + Sync
{
    fn init(&mut self, _server: &mut Server) -> Option<Box<dyn State>>;
    fn tick(&mut self, _server: &mut Server) -> Option<Box<dyn State>>;

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>;
    fn disconnect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>;
    fn got_tcp_packet(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>, _packet: &mut Packet) -> Result<(), &'static str>;
    fn got_udp_packet(&mut self, _server: &mut Server, _addr: &SocketAddr, _packet: &mut Packet) -> Result<(), &'static str> { Ok(()) }

    fn handle_identity(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet, accept: bool) -> Result<bool, &'static str>
    {
        {
            let mut peer = peer.lock().unwrap();

            if !peer.pending {
                peer.disconnect("Second identity attempt.");
                return Ok(false);
            }

            let build_ver = packet.ru16()?;
            if build_ver != BUILD_VER {
                peer.disconnect("Version mismatch.");
                return Ok(false);
            }

            let nickname = packet.rstr()?;
            if nickname.len() > 15 {
                peer.nickname = nickname.chars().take(15).collect();
            }
            else {
                peer.nickname = nickname;
            }
            
            peer.lobby_icon = packet.ru8()?;
            peer.pet = packet.ri8()?;
            let os_type = packet.ru8()?;
            peer.udid = packet.rstr()?;
            debug!("Identity of \"{}\" (ID {}):", peer.nickname, peer.id());
            debug!("OS: {} UDID: {}", os_type, peer.udid);
            
            // We're in game, so put the player in queue
            peer.in_queue = !accept;
            peer.pending = false;
            
            let mut packet = Packet::new(PacketType::SERVER_IDENTITY_RESPONSE);
            packet.wu8(accept as u8);
            packet.wu16(server.udp_port);
            packet.wu16(peer.id());
            peer.send(&mut packet);
        }
        
        let id = peer.lock().unwrap().id();
        server.peers.write().unwrap().insert(id, peer);
        debug!("Added ID {} to peer list!", id);
        Ok(true)
    }

    fn name(&self) -> &str { "default" }
}