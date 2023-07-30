use std::sync::{Mutex, Arc};

use crate::packet::Packet;
use crate::state::State;
use crate::server::{Server, Peer};

pub(crate) struct MapVote 
{

}

impl State for MapVote
{
    fn init(&mut self, _server: &mut Server) 
    {

    }

    fn tick(&mut self, _server: &mut Server) 
    {

    }

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) 
    {

    }

    fn disconnect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) 
    {

    }

    fn got_tcp_packet(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>, _packet: &mut Packet) 
    {

    }

    fn name(&self) -> &str { "MapVote" }
}

impl MapVote
{
    pub fn new() -> MapVote 
    {
        MapVote {  }
    }
}