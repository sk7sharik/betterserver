use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::ops::AddAssign;
use std::sync::{Mutex, Arc};

use log::{debug, info};
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{thread_rng, Rng};

use crate::map::Map;
use crate::maps::hideandseek2::HideAndSeek2;
use crate::maps::ravinemist::RavineMist;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers, assert_or_disconnect};
use crate::states::characterselect::CharacterSelect;

use super::lobby::Lobby;

pub(crate) struct MapVote 
{
    timer: u16,
    
    map_list: Vec<Arc<dyn Map>>,
    vote_maps: Vec<Arc<dyn Map>>,
    voted_peers: Vec<u16>,
    votes: HashMap<u8, u8>
}

impl State for MapVote
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        // we should have at least one map
        //TODO: finish
        if self.map_list.len() <= 0 {
            return Some(Box::new(Lobby::new()));
        }
        
        for i in 0..3 {
            self.votes.insert(i, 0);
        }

        if self.map_list.len() >= 3 {
            for _i in 0..3 {
                let map = self.map_list[thread_rng().gen_range(0..self.map_list.len())].clone();
                self.vote_maps.push(map);
            }
        }
        else {
            let mut last: Arc<dyn Map> = self.map_list[0].clone(); 
            for map in &self.map_list {
                self.vote_maps.push(map.clone());
                last = map.clone();
            }

            for _i in 0..(3-self.vote_maps.len()) {
                self.vote_maps.push(last.clone());
            }
        }

        let mut packet = Packet::new(PacketType::SERVER_VOTE_MAPS);
        for map in &self.vote_maps {
            packet.wu8(map.index() as u8);
        }

        server.multicast_real(&mut packet);
        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        if self.timer <= 0 {
            let winner = self.vote_winner();
            info!("[MapVote] Map [{}] won!", winner.name());

            return Some(Box::new(CharacterSelect::new(winner)));
        }

        if self.timer % 60 == 0 {
            let mut packet = Packet::new(PacketType::SERVER_VOTE_TIME_SYNC);
            packet.wu8((self.timer / 60) as u8);
            server.multicast(&mut packet); // works as hearbeat (bonus)

            debug!("[MapVote] Timer {} frames.", self.timer);
        }

        self.timer -= 1;
        None
    }

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        None
    }

    fn disconnect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        let id = peer.lock().unwrap().id();
        let mut packet = Packet::new(PacketType::SERVER_PLAYER_LEFT);
        packet.wu16(id);
        server.multicast_except(&mut packet, id);

        if real_peers!(server).count() <= 2 {
            return Some(Box::new(Lobby::new()));
        }

        None
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Option<Box<dyn State>> 
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
                self.handle_identity(server, &mut peer.lock().unwrap(), packet, false);
            },

            PacketType::CLIENT_VOTE_REQUEST => {
                let map = packet.ru8();

                // Sanity checks
                assert_or_disconnect!(map < 3, &mut peer.lock().unwrap());
                assert_or_disconnect!(!self.voted_peers.contains(&id), &mut peer.lock().unwrap());
                debug!("[MapVote] {} (ID {}) voted for [{}].", peer.lock().unwrap().nickname, id, self.vote_maps[map as usize].name());

                self.votes.get_mut(&map).unwrap().add_assign(1);
                self.voted_peers.push(id);

                let mut packet = Packet::new(PacketType::SERVER_VOTE_SET);
                packet.wu8(self.votes[&0]);
                packet.wu8(self.votes[&1]);
                packet.wu8(self.votes[&2]);
                server.multicast_real(&mut packet);

                self.check_votes(server);
            }

            _ => {}
        }

        None
    }

    fn name(&self) -> &str { "MapVote" }
}

impl MapVote
{
    pub fn new() -> MapVote 
    {
        MapVote 
        {  
            timer: 20 * 60,
            map_list: Vec::from([
                Arc::new(HideAndSeek2::new()) as Arc<dyn Map>,
                Arc::new(RavineMist::new()),
            ]),

            vote_maps: Vec::new(),
            voted_peers: Vec::new(),
            votes: HashMap::new(),
        }
    }

    fn check_votes(&mut self, server: &mut Server) 
    {
        if self.voted_peers.len() < real_peers!(server).count() {
            return;
        }

        if self.timer > 3 * 60 {
            self.timer = 3 * 60; 
        }
    }

    fn vote_winner(&mut self) -> Arc<dyn Map> 
    {
        let mut votes = 0;

        for vote in &self.votes {
            if *vote.1 >= votes {
                votes = *vote.1;
            }
        }

        let mut indexes = Vec::new();
        for vote in &self.votes {
            if *vote.1 == votes {
                indexes.push(*vote.0);
            }
        }

        // map indexes with this amount of votes
        let index = *indexes.choose(&mut thread_rng()).unwrap() as usize;
        self.vote_maps[index].clone()
    }
} 