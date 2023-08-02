use std::{sync::{Arc, Mutex}, net::SocketAddr};

use log::debug;

use crate::{server::{Server, Peer}, packet::{Packet, PacketType}, states::lobby::BUILD_VER};

pub(crate) trait State: Send + Sync
{
    fn init(&mut self, _server: &mut Server) -> Option<Box<dyn State>>;
    fn tick(&mut self, _server: &mut Server) -> Option<Box<dyn State>>;

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>;
    fn disconnect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>;
    fn got_tcp_packet(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>, _packet: &mut Packet) -> Option<Box<dyn State>>;
    fn got_udp_packet(&mut self, _server: &mut Server, _addr: &SocketAddr, _packet: &mut Packet) -> Option<Box<dyn State>> { None }

    fn handle_identity(&mut self, _server: &mut Server, peer: &mut Peer, packet: &mut Packet, accept: bool) 
    {
        if !peer.pending {
            peer.disconnect("Second identity attempt.");
            return
        }

        let build_ver = packet.ru16();
        if build_ver != BUILD_VER {
            peer.disconnect("Version mismatch.");
            return;
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
        peer.in_queue = !accept;
        peer.pending = false;
        
        let mut packet = Packet::new(PacketType::SERVER_IDENTITY_RESPONSE);
        packet.wu8(accept as u8);
        packet.wu16(peer.id());
        peer.send(&mut packet);
    }

    fn name(&self) -> &str { "default" }
}