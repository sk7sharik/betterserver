use crate::{server::{Server, Peer}, packet::Packet};

pub(crate) trait State : Send + Sync
{
    fn tick(&self, server: &mut Server) {}

    fn connect(&self, server: &mut Server, peer: &mut Peer) {}
    fn disconnect(&self, server: &mut Server, peer: &mut Peer) {}
    fn got_tcp_packet(&self, server: &mut Server, peer: &mut Peer, packet: &mut Packet) {}
}