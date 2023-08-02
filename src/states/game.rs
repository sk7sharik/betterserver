use std::any::Any;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Mutex, Arc, RwLock};

use log::{info, warn, debug};
use rand::{thread_rng, Rng};

use crate::entities::ring::Ring;
use crate::entities::tailsprojectile::TailsProjectile;
use crate::map::Map;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers};
use crate::entity::Entity;

use super::lobby::Lobby;

pub(crate) struct Game
{
    pub map: Arc<Mutex<dyn Map>>,
    pub entities: Arc<Mutex<HashMap<u16, Box<dyn Entity>>>>,
    pub rings: Vec<bool>,

    started: bool,
    timer: u16,
    ring_timer: u16,
    recp: HashMap<u16, SocketAddr>,
    entity_id: u16,
    entity_destroy_queue: Vec<u16>
}

macro_rules! find_entities {
    ($entities: expr, $id: expr) => {
        $entities.iter().filter(|x| x.1.id() == $id)
    };
}
pub(crate) use find_entities;

macro_rules! find_entities_mut {
    ($entities: expr, $id: expr) => {
        $entities.iter_mut().filter(|x| x.1.id() == $id)
    };
}
pub(crate) use find_entities_mut;

impl State for Game
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        let mut packet = Packet::new(PacketType::SERVER_LOBBY_GAME_START);
        server.multicast_real(&mut packet);

        self.timer = (self.map.lock().unwrap().timer_sec(&server) * 60.0) as u16;
        self.ring_timer = (self.map.lock().unwrap().ring_time_sec(&server) * 60.0) as u16;
        self.rings = vec![false; self.map.lock().unwrap().ring_count()];
        info!("Waiting for players...");
        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        if !self.started {
            return None;
        }

        let map = self.map.clone();
        map.lock().unwrap().tick(server, self);

        // Handle entities
        self.entity_check_destroy(server);

        let entities_clone = self.entities.clone();
        for ent in entities_clone.lock().unwrap().iter_mut() {
            let packet = ent.1.tick(server, self, ent.0);

            if packet.is_some() {
                server.udp_multicast(&self.recp,&mut packet.unwrap());
            }
        }

        if self.timer % self.ring_timer == 0 {
            self.spawn(server, Box::new(Ring::new()));
        }

        self.timer -= 1;
        if self.timer % 60 == 0 {
            let mut packet = Packet::new(PacketType::SERVER_GAME_TIME_SYNC);
            packet.wu16(self.timer);
            server.multicast(&mut packet);
            debug!("Timer tick");
        }


        if self.timer <= 0 {

        }

        None
    }

    fn connect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        None
    }

    fn disconnect(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
    {
        if peer.lock().unwrap().in_queue {
            return None;
        }

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
        let passtrough = packet.ru8() != 0; //TODO: get rid of
        let tp = packet.rpk();
        debug!("TCP Recv {:?}", tp);

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

            // Ring collected
            PacketType::CLIENT_RING_COLLECTED => {
                let id = packet.ru8() as usize;
                let uid = packet.ru16();
                
                for entity in find_entities!(self.entities.clone().lock().unwrap(), "ring") {
                    let ring = entity.1.as_any().downcast_ref::<Ring>().unwrap();
                    
                    if ring.id != id || ring.uid != uid {
                        continue;
                    }

                    let mut packet = Packet::new(PacketType::SERVER_RING_COLLECTED);
                    packet.wu8(ring.red as u8);
                    peer.lock().unwrap().send(&mut packet);

                    self.queue_destroy(entity.0);
                }
            },

            // Spawn projectile
            PacketType::CLIENT_TPROJECTILE => {
                if find_entities!(self.entities.lock().unwrap(), "tproj").count() > 0 {
                    peer.lock().unwrap().disconnect("Projectile abusing.");
                    return None;
                }

                self.spawn(server, Box::new(TailsProjectile {
                    owner: id,
                    x: packet.ru16() as i32,
                    y: packet.ru16() as i32,
                    dir: packet.ri8(),
                    dmg: packet.ru8(),
                    exe: packet.ru8() != 0,
                    charge: packet.ru8(),
                    timer: 5 * 60
                }));
            },

            // Destroy projectile
            PacketType::CLIENT_TPROJECTILE_HIT => {
                for proj in find_entities!(self.entities.clone().lock().unwrap(), "tproj") {
                    self.queue_destroy(proj.0);
                }
            },

            _ => {
                if passtrough {
                    server.multicast_real_except(packet, id);
                }
            }
        }

        self.entity_check_destroy(server);
        None
    }

    fn got_udp_packet(&mut self, server: &mut Server, addr: &SocketAddr, packet: &mut Packet) -> Option<Box<dyn State>> 
    {
        let pid = packet.ru16();
        let tp = packet.rpk();

        if !self.started {
            if !self.recp.contains_key(&pid) {
                match server.peers.read().unwrap().get(&pid) {
                    Some(res) => {
                        let peer = res.lock().unwrap();
                        info!("{} (ID {}) is UDP ready!", peer.nickname, peer.id());
                    },

                    None => {
                        warn!("Suspicious UDP: peer with ID {} doesn't exist", pid);
                        return None;
                    }
                };

                self.recp.insert(pid, *addr);
            }

            if self.recp.len() >= real_peers!(server).count() {
                self.started = true;

                let map_clone = self.map.clone();
                map_clone.lock().unwrap().init(server, self);

                let mut packet = Packet::new(PacketType::SERVER_GAME_PLAYERS_READY);
                server.multicast_real(&mut packet);

                info!("Game started! (Timer is {} frames)", self.timer);
            }

            return None;
        }

        match tp {
            PacketType::CLIENT_PLAYER_DATA => {

                let pak = &packet.raw()[3..];
                server.udp_multicast_except(&self.recp, &mut Packet::headless(PacketType::CLIENT_PLAYER_DATA, pak, pak.len()), addr);
            },

            PacketType::CLIENT_PING => {
                let ping = packet.ru64();
                let calc = packet.ru16();

                let mut packet = Packet::new(PacketType::SERVER_PONG);
                packet.wu64(ping);
                server.udp_send(addr, &mut packet);

                let mut packet = Packet::new(PacketType::SERVER_GAME_PING);
                packet.wu16(pid);
                packet.wu16(calc);
                server.udp_multicast_except(&self.recp, &mut packet, addr);

            },

            _ => {}
        }

        self.entity_check_destroy(server);
        None
    }

    fn name(&self) -> &str { "Game" }
}

impl Game
{
    pub fn new(map: Arc<Mutex<dyn Map>>) -> Game
    {
        Game 
        { 
            map,
            timer: 0,
            started: false,
            entities: Arc::new(Mutex::new(HashMap::new())),
            rings: Vec::new(),
            recp: HashMap::new(),
            ring_timer: 0,

            entity_id: 0,
            entity_destroy_queue: Vec::new()
        }
    }

    pub fn spawn(&mut self, server: &mut Server, entity: Box<dyn Entity>) 
    {
        self.entity_id += 1;
        let id = self.entity_id;

        let mut entity = entity;
        let packet = entity.spawn(server, self, &id);

        if packet.is_some() {
            server.multicast_real(&mut packet.unwrap());
        }

        info!("Spawned entity (type {}, ID {})", entity.id(), id);
        self.entities.lock().unwrap().insert(id, entity);
    }

    pub fn spawn_quiet(&mut self, server: &mut Server, entity: Box<dyn Entity>) 
    {
        self.entity_id += 1;
        let id = self.entity_id;

        let mut entity = entity;
        entity.spawn(server, self, &id);

        info!("Spawned entity (quiet) (type {}, ID {})", entity.id(), id);
        self.entities.lock().unwrap().insert(id, entity);
    }

    pub fn queue_destroy(&mut self, id: &u16)
    {
        self.entity_destroy_queue.push(*id);
        debug!("Queued destruction of entity (ID {})", *id);
    }

    fn entity_check_destroy(&mut self, server: &mut Server)
    {
        for id in &self.entity_destroy_queue.clone()
        {
            let mut entity = match self.entities.lock().unwrap().remove(&id)
            {
                Some(res) => res,
                None => {
                    warn!("Failed to remove entity (ID{})", id);
                    return;
                }
            };

            let packet = entity.destroy(server, self, id);
            if packet.is_some() {
                server.multicast_real(&mut packet.unwrap());
            }

            info!("Destroyed entity (type {}, ID {})", entity.id(), self.entity_id);
        }

        if self.entity_destroy_queue.len() > 0 {
            self.entity_destroy_queue.clear();
        }
    }

}