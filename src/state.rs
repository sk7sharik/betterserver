use std::sync::{Arc, Mutex};

use crate::{server::{Server, Peer}, packet::Packet};

pub(crate) trait State : Send + Sync
{
    fn tick(&mut self, _server: &mut Server) {}

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) {}
    fn disconnect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) {}
    fn got_tcp_packet(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>, _packet: &mut Packet) {}
}