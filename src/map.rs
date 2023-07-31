use std::sync::{Mutex, Arc};

use crate::{server::{Server, real_peers, Peer}, packet::Packet, states::game::Game};

pub(crate) trait Map: Send + Sync
{
    fn player_time_multiplier(&self, server: &Server) -> u16 {
        ((real_peers!(server).count() - 1) * 20) as u16
    }

    fn ring_time(&self) -> u16 {
        5 * 60
    }

    fn spawn_red_rings(&self) -> bool {
        true
    }

    fn init(&mut self, server: &mut Server, game: &mut Game) 
    {
        
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game) 
    {
        
    }

    fn got_tcp_packet(&mut self, server: &mut Server, game: &mut Game, peer: Arc<Mutex<Peer>>, packet: &mut Packet) 
    {
        
    }

    fn name(&self) -> &str;
    fn index(&self) -> usize;
}