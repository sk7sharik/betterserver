use std::sync::{Mutex, Arc};

use crate::{server::{Server, real_peers, Peer}, packet::Packet, states::game::Game};

pub(crate) trait Map: Send + Sync
{
    fn timer_sec(&self, server: &Server) -> f32 {
        180.0 * self.player_time_multiplier(server)
    }

    fn player_time_multiplier(&self, server: &Server) -> f32 {
        (real_peers!(server).count() as f32).max(3.0) / 3.0
    }

    fn ring_count(&self) -> usize {
        25
    }

    fn ring_time_sec(&self, server: &Server) -> f32 {
        5.0 - self.player_time_multiplier(server) * 0.5
    }

    fn spawn_red_rings(&self) -> bool {
        true
    }

    fn init(&mut self, server: &mut Server, game: &mut Game);
    fn tick(&mut self, server: &mut Server, game: &mut Game);
    fn got_tcp_packet(&mut self, server: &mut Server, game: &mut Game, peer: Arc<Mutex<Peer>>, packet: &mut Packet) {}

    fn name(&self) -> &str;
    fn index(&self) -> usize;
}