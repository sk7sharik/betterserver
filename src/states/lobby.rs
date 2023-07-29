use crate::{state::State, server::{Server, Peer}, packet::Packet};

pub(crate) struct Lobby
{

}


impl State for Lobby
{
    fn tick(&self, server: &mut Server) {
        println!("tick!");
    }

    fn got_tcp_packet(&self, server: &mut Server, peer: &mut Peer, packet: &mut Packet) {

    }
}

impl Lobby
{
    pub fn new() -> Lobby {
        Lobby { }
    }
}