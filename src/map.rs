use std::sync::{Mutex, Arc};

use crate::{server::{Server, real_peers, Peer}, packet::Packet, states::game::Game};

pub(crate) trait Map: Send + Sync
{
    fn timer(&self, server: &Server) -> f32 {
        (180.0 + 100.0 * self.player_time_multiplier(server)) * 60.0
    }

    fn player_time_multiplier(&self, server: &Server) -> f32 {
        (real_peers!(server).count() as f32) / 7.0
    }

    fn ring_count(&self) -> usize {
        1
    }

    fn ring_time(&self, server: &Server) -> f32 {
        300.0 - (60.0 * self.player_time_multiplier(server) * 0.25)
    }

    fn spawn_red_rings(&self) -> bool {
        true
    }

    fn bring_spawn(&self) -> bool {
        true
    }

    fn bring_activate_time(&self) -> u16 {
       (60 - 10) * 60
    }
 
    fn init(&mut self, _server: &mut Server, _game: &mut Game) {}
    fn tick(&mut self, _server: &mut Server, _game: &mut Game) {}
    fn got_tcp_packet(&mut self, _server: &mut Server, _game: &mut Game, _peer: Arc<Mutex<Peer>>, _packet: &mut Packet) -> Result<(), &'static str> { Ok(()) }

    fn name(&self) -> &str;
    fn index(&self) -> usize;
}