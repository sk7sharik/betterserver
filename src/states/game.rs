use std::collections::HashMap;
use std::f32::consts::PI;
use std::net::SocketAddr;
use std::sync::{Mutex, Arc};

use log::{info, warn, debug};
use rand::{thread_rng, Rng};

use crate::entities::blackring::BlackRing;
use crate::entities::creamring::CreamRing;
use crate::entities::eggtrack::EggmanTracker;
use crate::entities::exclone::ExellerClone;
use crate::entities::ring::Ring;
use crate::entities::tailsprojectile::TailsProjectile;
use crate::map::Map;
use crate::packet::{Packet, PacketType};
use crate::state::State;
use crate::server::{Server, Peer, real_peers, assert_or_disconnect, SurvivorCharacter, PlayerRevival, ExeCharacter};
use crate::entity::Entity;
use crate::timer::Timer;

use super::lobby::Lobby;

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

enum Ending
{
    ExeWin,
    SurvWin,
    TimeOver
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) enum GameTimer
{
    Time,

    TailsProjectile,
    EggmanTracker,
    CreamRing,
    ExetiorRing,
}

pub(crate) struct Game
{
    pub map: Arc<Mutex<dyn Map>>,
    recp: HashMap<u16, SocketAddr>,

    // Rings
    pub rings: Vec<bool>,
    ring_time: u16,

    // Big ring
    big_ring_time: u16,
    big_ring_ready: bool,
    big_ring_spawn: bool,

    // Game state
    pub timer: Timer<GameTimer>,
    started: bool,
    end_timer: u16,
    
    // Entities
    pub entities: Arc<Mutex<HashMap<u16, Box<dyn Entity>>>>,
    entity_id: u16,
    entity_destroy_queue: Vec<u16>,

    // Player
    pub players_pos: HashMap<u16, (f32, f32)>
}

impl State for Game
{
    fn init(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        let mut packet = Packet::new(PacketType::SERVER_LOBBY_GAME_START);
        server.multicast_real(&mut packet);

        {
            let map = self.map.lock().unwrap();
            
            self.timer.set(GameTimer::Time, map.timer(&server) as u16);
            
            self.ring_time = map.ring_time(&server) as u16;
            self.rings = vec![false; map.ring_count()];
            self.big_ring_spawn = map.bring_spawn();
            self.big_ring_time = map.bring_activate_time();
        }

        info!("Waiting for players...");
        None
    }

    fn tick(&mut self, server: &mut Server) -> Option<Box<dyn State>> 
    {
        if self.end_timer > 0 {
            self.end_timer -= 1;

            if self.end_timer == 0 {
                return Some(Box::new(Lobby::new()));
            }

            return None;
        }

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

        self.do_timers(server);
        None
    }

    fn connect(&mut self, _server: &mut Server, _peer: Arc<Mutex<Peer>>) -> Option<Box<dyn State>>
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

        if peer.lock().unwrap().player.as_ref().unwrap().exe {
            self.end(server, Ending::SurvWin);
            return None;
        }

        if real_peers!(server).count() <= 1 {
            return Some(Box::new(Lobby::new()));
        }

        self.check_state(server);
        None
    }

    fn got_tcp_packet(&mut self, server: &mut Server, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Result<(), &'static str>
    {
        let passtrough = packet.ru8()? != 0; //TODO: get rid of
        let tp = packet.rpk()?;
        debug!("Got packet {:?}", tp);

        if !peer.lock().unwrap().pending {
            peer.lock().unwrap().timer = 0;
        }

        let id = peer.lock().unwrap().id();
        match tp
        {
            PacketType::IDENTITY => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                self.handle_identity(server, peer.clone(), packet, false)?;
            },

            PacketType::CLIENT_PLAYER_DEATH_STATE => {
                let peer_count = real_peers!(server).filter(|x| !x.lock().unwrap().player.as_ref().unwrap().exe).count();
                let demon_count = real_peers!(server).filter(|x| x.lock().unwrap().player.as_ref().unwrap().revival_times >= 2).count();
                
                // Sanity checks
                let mut dead = false;
                let mut revival_times = 0;
                {
                    let mut peer = peer.lock().unwrap();
                    let player = peer.player.as_mut().unwrap();
                    
                    assert_or_disconnect!(player.revival_times < 2, &mut peer);
                    player.dead = packet.ru8()? != 0;
                    player.revival_times = packet.ru8()?;
                    player.revival = PlayerRevival { progress: 0.0, initiators: Vec::new() };

                    dead = player.dead;
                    revival_times = player.revival_times;
                }
                
                let mut packet = Packet::new(PacketType::SERVER_PLAYER_DEATH_STATE);
                packet.wu16(id);
                packet.wu8(dead as u8);
                packet.wu8(revival_times);
                server.multicast_real(&mut packet);

                let mut packet = Packet::new(PacketType::SERVER_REVIVAL_STATUS);
                packet.wu8(false as u8);
                packet.wu16(id);
                server.multicast(&mut packet);

                if dead {
                    info!("RIP {} (ID {}).", peer.lock().unwrap().nickname, id);

                    if revival_times == 0 {
                        peer.lock().unwrap().player.as_mut().unwrap().death_timer = 30 * 60;
                    }

                    if revival_times == 1 || (revival_times == 0 && self.timer.get(GameTimer::Time) <= 3600 * 2) {
                        let mut packet = Packet::new(PacketType::SERVER_GAME_DEATHTIMER_END);
                        
                        if demon_count < peer_count / 2 {
                            peer.lock().unwrap().player.as_mut().unwrap().revival_times = 2;
                            packet.wu8(1);
                            info!("{} (ID {}) was demonized!", peer.lock().unwrap().nickname, id);
                        }
                        else {
                            packet.wu8(0);
                        }

                        peer.lock().unwrap().send(&mut packet);
                    }
                }
                else {
                    peer.lock().unwrap().player.as_mut().unwrap().death_timer = -1;
                }

                self.check_state(server);
            },

            PacketType::CLIENT_PLAYER_ESCAPED => {
                
                // Sanity checks
                {
                    let mut peer = peer.lock().unwrap();
                    assert_or_disconnect!(self.timer.get(GameTimer::Time) <= self.big_ring_time, &mut peer);
                    assert_or_disconnect!(!peer.player.as_ref().unwrap().exe, &mut peer);
                    assert_or_disconnect!(!peer.player.as_ref().unwrap().dead, &mut peer);
                    assert_or_disconnect!(!peer.player.as_ref().unwrap().red_ring, &mut peer);
                    assert_or_disconnect!(peer.player.as_ref().unwrap().revival_times < 2, &mut peer);
                    peer.player.as_mut().unwrap().escaped = true;
                }

                let mut packet = Packet::new(PacketType::SERVER_GAME_PLAYER_ESCAPED);
                packet.wu16(id);
                server.multicast_real(&mut packet);

                info!("{} (ID {}) escaped!", peer.lock().unwrap().nickname, id);
                self.check_state(server);
            },

            PacketType::CLIENT_REVIVAL_PROGRESS => {
                let pid = packet.ru16()?;
                let rings = packet.ru8()?;
                let mut sub_vec: Vec<u16> = Vec::new();

                for peer in real_peers!(server) {
                    if peer.lock().unwrap().id() != pid {
                        continue;
                    }

                    if peer.lock().unwrap().player.as_ref().unwrap().revival_times >= 2 {
                        break;
                    }

                    if peer.lock().unwrap().player.as_ref().unwrap().revival.progress <= 0.0 {
                        let mut packet = Packet::new(PacketType::SERVER_REVIVAL_STATUS);
                        packet.wu8(true as u8);
                        packet.wu16(pid);

                        server.multicast_real(&mut packet);
                    }

                    if !peer.lock().unwrap().player.as_ref().unwrap().revival.initiators.contains(&id) {
                        peer.lock().unwrap().player.as_mut().unwrap().revival.initiators.push(id.clone());
                    }

                    peer.lock().unwrap().player.as_mut().unwrap().revival.progress += 0.015 + (0.004 * rings as f64);
                    if peer.lock().unwrap().player.as_ref().unwrap().revival.progress >= 1.0 {
                        sub_vec = peer.lock().unwrap().player.as_ref().unwrap().revival.initiators.clone();
                        peer.lock().unwrap().player.as_mut().unwrap().revival = PlayerRevival { progress: 0.0, initiators: Vec::new() };

                        let mut packet = Packet::new(PacketType::SERVER_REVIVAL_STATUS);
                        packet.wu8(false as u8);
                        packet.wu16(pid);
                        server.multicast_real(&mut packet);

                        peer.lock().unwrap().send(&mut Packet::new(PacketType::SERVER_REVIVAL_REVIVED));

                        info!("{} (ID {}) was revived!", peer.lock().unwrap().nickname, id);
                    }
                    else {
                        let mut packet = Packet::new(PacketType::SERVER_REVIVAL_PROGRESS);
                        packet.wu16(pid);
                        packet.wf64(peer.lock().unwrap().player.as_ref().unwrap().revival.progress);
                        server.udp_multicast(&self.recp, &mut packet);
                    }

                    break;
                }

                for tid in sub_vec {
                    for peer in real_peers!(server) {
                        let mut peer = peer.lock().unwrap();

                        if peer.id() != tid {
                            continue;
                        }

                        peer.send(&mut Packet::new(PacketType::SERVER_REVIVAL_RINGSUB));
                        break;
                    } 
                }
            },

            // Spawn projectile
            PacketType::CLIENT_TPROJECTILE => {
                {
                    let mut peer = peer.lock().unwrap();
                    assert_or_disconnect!(!passtrough, peer);
                    assert_or_disconnect!(peer.player.as_ref().unwrap().ch1 == SurvivorCharacter::Tails, peer);
                    assert_or_disconnect!(self.timer.get(GameTimer::TailsProjectile) == 0, peer);
                    assert_or_disconnect!(find_entities!(self.entities.lock().unwrap(), "tproj").count() == 0, peer);
                }

                self.spawn(server, Box::new(TailsProjectile {
                    owner: id,
                    x: packet.ru16()? as i32,
                    y: packet.ru16()? as i32,
                    dir: packet.ri8()?,
                    dmg: packet.ru8()?,
                    exe: packet.ru8()? != 0,
                    charge: packet.ru8()?,
                    timer: 5 * 60
                }));
                
                self.timer.set(GameTimer::TailsProjectile, 10 * 60);
            },

            // Destroy projectile
            PacketType::CLIENT_TPROJECTILE_HIT => {
                for proj in find_entities!(self.entities.clone().lock().unwrap(), "tproj") {
                    self.queue_destroy(proj.0);
                }
            },

            PacketType::CLIENT_ETRACKER => {
                {
                    let mut peer = peer.lock().unwrap();
                    assert_or_disconnect!(!passtrough, peer);
                    assert_or_disconnect!(peer.player.as_ref().unwrap().ch1 == SurvivorCharacter::Eggman, peer);
                    assert_or_disconnect!(self.timer.get(GameTimer::EggmanTracker) == 0, peer);
                }

                self.spawn(server, Box::new(EggmanTracker {
                    x: packet.ru16()?,
                    y: packet.ru16()?,
                    activated_by: 0
                }));

                self.timer.set(GameTimer::TailsProjectile, 10 * 60);
            },

            PacketType::CLIENT_ETRACKER_ACTIVATED => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                let eid = packet.ru16()?;

                for entity in find_entities_mut!(self.entities.clone().lock().unwrap(), "eggtrack") {
                    if *entity.0 != eid {
                        continue;
                    }

                    let track = entity.1.as_any_mut().downcast_mut::<EggmanTracker>().unwrap();
                    track.activated_by = id;
                    self.queue_destroy(entity.0);
                    break;
                }
            },

            PacketType::CLIENT_CREAM_SPAWN_RINGS => {
                {
                    let mut peer = peer.lock().unwrap();
                    assert_or_disconnect!(!passtrough, peer);
                    assert_or_disconnect!(peer.player.as_ref().unwrap().ch1 == SurvivorCharacter::Cream, peer);
                    assert_or_disconnect!(self.timer.get(GameTimer::CreamRing) == 0, peer);
                }

                let x = packet.ru16()?;
                let y = packet.ru16()?;
                let red = packet.ru8()? != 0;

                // Sanity checks
                {
                    let mut peer = peer.lock().unwrap();
                    let player = peer.player.as_ref().unwrap();

                    assert_or_disconnect!(player.ch1 == SurvivorCharacter::Cream, &mut peer);

                    if red {
                        assert_or_disconnect!(player.revival_times >= 2, &mut peer);
                    }
                    else {
                        assert_or_disconnect!(player.revival_times < 2 && !player.dead, &mut peer);
                    }
                }

                if red {
                    for i in 0..2 {
                        self.spawn(server, Box::new(CreamRing {
                            x: (x as f32 + (PI * 2.5 - (i as f32 * PI)).sin() * 26.0) as i16 - 1,
                            y: (y as f32 + (PI * 2.5 - (i as f32 * PI)).cos() * 26.0) as i16,
                            red: true
                        }));
                    }
                }
                else {
                    for i in 0..3 {
                        self.spawn(server, Box::new(CreamRing {
                            x: (x as f32 + (PI * 2.5 + (i as f32 * (PI / 2.0))).sin() * 26.0) as i16,
                            y: (y as f32 + (PI * 2.5 + (i as f32 * (PI / 2.0))).cos() * 26.0) as i16,
                            red: false
                        }));
                    }
                }

                self.timer.set(GameTimer::CreamRing, 10 * 60);
            },

            PacketType::CLIENT_RING_COLLECTED => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());

                let rid = packet.ru8()? as usize;
                let eid = packet.ru16()?;
                
                for entity in find_entities!(self.entities.clone().lock().unwrap(), "ring") {
                    let ring = entity.1.as_any().downcast_ref::<Ring>().unwrap();
                    
                    if ring.id != rid || *entity.0 != eid {
                        continue;
                    }

                    let mut packet = Packet::new(PacketType::SERVER_RING_COLLECTED);
                    packet.wu8(ring.red as u8);
                    peer.lock().unwrap().send(&mut packet);

                    self.rings[ring.id] = false;
                    self.queue_destroy(entity.0);
                    break;
                }

                // Cream's ring
                if rid == 255 {
                    for entity in find_entities!(self.entities.clone().lock().unwrap(), "creamring") {
                        let ring = entity.1.as_any().downcast_ref::<CreamRing>().unwrap();
                        
                        if *entity.0 != eid {
                            continue;
                        }

                        let mut packet = Packet::new(PacketType::SERVER_RING_COLLECTED);
                        packet.wu8(ring.red as u8);
                        peer.lock().unwrap().send(&mut packet);

                        self.queue_destroy(entity.0);
                        break;
                    }
                }
            },

            PacketType::CLIENT_ERECTOR_BRING_SPAWN => {
                {
                    let mut peer = peer.lock().unwrap();
                    assert_or_disconnect!(!passtrough, peer);
                    assert_or_disconnect!(peer.player.as_ref().unwrap().ch2 == ExeCharacter::Exetior, peer);
                    assert_or_disconnect!(self.timer.get(GameTimer::ExetiorRing) == 0, peer);
                }

                let x = packet.ru16()?;
                let y = packet.ru16()?;
                let rid = self.spawn_quiet(server, Box::new(BlackRing {}));

                let mut packet = Packet::new(PacketType::SERVER_ERECTOR_BRING_SPAWN);
                packet.wu16(rid);
                packet.wu16(x);
                packet.wu16(y);
                server.multicast_real(&mut packet);

                self.timer.set(GameTimer::ExetiorRing, 10 * 60);
            },

            PacketType::CLIENT_BRING_COLLECTED => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());

                let eid = packet.ru16()?;
                for entity in find_entities!(self.entities.clone().lock().unwrap(), "blackring") {                    
                    if *entity.0 != eid {
                        continue;
                    }

                    let mut packet = Packet::new(PacketType::SERVER_BRING_COLLECTED);
                    peer.lock().unwrap().send(&mut packet);

                    self.queue_destroy(entity.0);
                    break;
                }
            },

            PacketType::CLIENT_ERECTOR_BALLS => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());

                let x = packet.rf32()?;
                let y = packet.rf32()?;

                let mut packet = Packet::new(PacketType::CLIENT_ERECTOR_BALLS);
                packet.wf32(x);
                packet.wf32(y);
                server.multicast_real(&mut packet);
            },

            PacketType::CLIENT_EXELLER_SPAWN_CLONE => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                assert_or_disconnect!(find_entities!(self.entities.clone().lock().unwrap(), "exclone").count() < 2, &mut peer.lock().unwrap());

                self.spawn(server, Box::new(ExellerClone {
                    owner_id: id,
                    x: packet.ru16()?,
                    y: packet.ru16()?,
                    dir: packet.ri8()?
                }));
            },

            PacketType::CLIENT_EXELLER_TELEPORT_CLONE => {
                assert_or_disconnect!(!passtrough, &mut peer.lock().unwrap());
                let cid = packet.ru16()?;
                
                for entity in find_entities!(self.entities.clone().lock().unwrap(), "exclone") {
                    if *entity.0 != cid {
                        continue;
                    }
                    
                    self.queue_destroy(entity.0);
                    break;
                }
            },

            _ => {                
                if passtrough {
                    server.multicast_real_except(packet, id);
                }
            }
        }
        
        packet.rewind(0);
        self.map.clone().lock().unwrap().got_tcp_packet(server, self, peer, packet)?;
        self.entity_check_destroy(server);
        Ok(())
    }

    fn got_udp_packet(&mut self, server: &mut Server, addr: &SocketAddr, packet: &mut Packet) -> Result<(), &'static str>
    {
        let pid = packet.ru16()?;
        let tp = packet.rpk()?;

        match tp {
            PacketType::CLIENT_PLAYER_DATA => {
                if self.started {
                    let pak = &packet.raw()[3..];
                    server.udp_multicast_except(&self.recp, &mut Packet::headless(PacketType::CLIENT_PLAYER_DATA, pak, pak.len()), addr);

                    match self.players_pos.get_mut(&pid)
                    {
                        Some(pos) => {
                            let _ = packet.ru16()?;
                            pos.0 = packet.rf32()?;
                            pos.1 = packet.rf32()?;
                        },

                        None => {
                            return Err("Player with specified id doesn't exist");
                        }
                    }
                }
            },

            PacketType::CLIENT_PING => {
                let ping = packet.ru64()?;
                let calc = packet.ru16()?;

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

        if !self.started {
            if !self.recp.contains_key(&pid) {
                match server.peers.read().unwrap().get(&pid) {
                    Some(res) => {
                        let peer = res.lock().unwrap();
                        info!("{} (ID {}) is UDP ready!", peer.nickname, peer.id());
                    },

                    None => {
                        warn!("Suspicious UDP: peer with ID {} doesn't exist", pid);
                        return Ok(());
                    }
                };

                self.recp.insert(pid, *addr);
                self.players_pos.insert(pid, (0.0, 0.0));
            }

            if self.recp.len() >= real_peers!(server).count() {
                self.started = true;

                self.map.clone().lock().unwrap().init(server, self);

                let mut packet = Packet::new(PacketType::SERVER_GAME_PLAYERS_READY);
                server.multicast_real(&mut packet);

                info!("Game started! (Timer is {} frames)", self.timer.get(GameTimer::Time));
            }

            return Ok(());
        }

        self.entity_check_destroy(server);
        Ok(())
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
            rings: Vec::new(),
            recp: HashMap::new(),
            started: false,

            big_ring_time: 0,
            big_ring_ready: false,
            big_ring_spawn: false,

            ring_time: 0,
            end_timer: 0,
            timer: Timer::<GameTimer>::new(),

            entities: Arc::new(Mutex::new(HashMap::new())),
            entity_id: 0,
            entity_destroy_queue: Vec::new(),

            players_pos: HashMap::new()
        }
    }

    pub fn spawn(&mut self, server: &mut Server, entity: Box<dyn Entity>) -> u16
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

        id
    }

    pub fn spawn_quiet(&mut self, server: &mut Server, entity: Box<dyn Entity>) -> u16
    {
        self.entity_id += 1;
        let id = self.entity_id;

        let mut entity = entity;
        entity.spawn(server, self, &id);

        info!("Spawned entity (quiet) (type {}, ID {})", entity.id(), id);
        self.entities.lock().unwrap().insert(id, entity);

        id
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
                    warn!("Failed to remove entity (ID {})", id);
                    return;
                }
            };

            let packet = entity.destroy(server, self, id);
            if packet.is_some() {
                server.multicast_real(&mut packet.unwrap());
            }

            info!("Destroyed entity (type {}, ID {})", entity.id(), *id);
        }

        if self.entity_destroy_queue.len() > 0 {
            self.entity_destroy_queue.clear();
        }
    }

    fn do_timers(&mut self, server: &mut Server)
    {
        let game_time = self.timer.get(GameTimer::Time);

        // Player death timer
        let mut packets = Vec::new();
        for peer in real_peers!(server)
        {
            let peer_count = real_peers!(server).filter(|x| !x.lock().unwrap().player.as_ref().unwrap().exe).count();
            let demon_count = real_peers!(server).filter(|x| x.lock().unwrap().player.as_ref().unwrap().revival_times >= 2).count();

            let id;
            let mut death_timer;
            {
                let mut peer = peer.lock().unwrap();
                id = peer.id();

                let player = peer.player.as_mut().unwrap();
                death_timer = player.death_timer;

                if !player.dead || player.escaped || player.revival_times >= 2 {
                    player.death_timer = -1;
                    continue;
                }
            }

            if death_timer > 0 {
                death_timer -= 1;

                if game_time <= 3600 * 2 {
                    death_timer = 0;
                }

                if death_timer == 0 {
                    let mut packet = Packet::new(PacketType::SERVER_GAME_DEATHTIMER_END);
                    
                    if demon_count < peer_count / 2 {
                        peer.lock().unwrap().player.as_mut().unwrap().revival_times = 2;
                        packet.wu8(1);
                        info!("{} (ID {}) was demonized!", peer.lock().unwrap().nickname, id);
                    }
                    else {
                        packet.wu8(0);
                        info!("RIP {} (ID {})!", peer.lock().unwrap().nickname, id);
                    }

                    peer.lock().unwrap().send(&mut packet);
                }

                if death_timer % 60 == 0 {
                    let mut packet = Packet::new(PacketType::SERVER_GAME_DEATHTIMER_TICK);
                    packet.wu16(id);
                    packet.wu8((death_timer / 60) as u8);
                    packets.push(packet);
                }
            }

            peer.lock().unwrap().player.as_mut().unwrap().death_timer = death_timer;
        }

        for packet in packets.iter_mut() {
            server.multicast_real(packet);
        }

        if self.big_ring_spawn {
            // Spawn big ring
            if game_time == 60 * 60 {
                let mut packet = Packet::new(PacketType::SERVER_GAME_SPAWN_RING);
                packet.wu8(false as u8);
                packet.wu8(thread_rng().gen_range(0..255));
                server.multicast_real(&mut packet);

                info!("Big ring spawned!");
            }

            // Activate big ring
            if game_time == self.big_ring_time {
                self.big_ring_ready = true;

                let mut packet = Packet::new(PacketType::SERVER_GAME_SPAWN_RING);
                packet.wu8(true as u8);
                packet.wu8(thread_rng().gen_range(0..255));
                server.multicast_real(&mut packet);

                info!("Big ring activate!");
            }
        }

        // Timer sync
        if game_time % 60 == 0 {
            let mut packet = Packet::new(PacketType::SERVER_GAME_TIME_SYNC);
            packet.wu16(game_time);
            server.multicast(&mut packet);
            debug!("Timer tick");
        }

        // Time over
        if game_time <= 0 {
            self.end(server, Ending::TimeOver);
            return;
        }

        // Timer tick
        self.timer.tick();
    }

    fn check_state(&mut self, server: &mut Server) 
    {
        let mut alive = 0;
        let mut escaped = 0;
        for peer in real_peers!(server) {
            let peer = peer.lock().unwrap();
            let player = peer.player.as_ref().unwrap();

            if player.exe {
                continue;
            }

            if player.escaped {
                escaped += 1;
            }

            if !player.dead {
                alive += 1;
            }
        }

        if alive == 0 && escaped == 0 {
            self.end(server, Ending::ExeWin);
            return;
        }

        let count = real_peers!(server).count() as i32;
        if (count - alive) + escaped >= count {
            if escaped == 0 {
                self.end(server, Ending::ExeWin);
            }
            else {
                self.end(server, Ending::SurvWin);
            }
        }
    }

    fn end(&mut self, server: &mut Server, ending: Ending) 
    {
        match ending {
            Ending::ExeWin => {
                info!("Exe won (killed everyone)!");

                let mut packet = Packet::new(PacketType::SERVER_GAME_EXE_WINS);
                server.multicast_real(&mut packet);
            },

            Ending::SurvWin => {
                info!("Survivors won!");

                let mut packet = Packet::new(PacketType::SERVER_GAME_SURVIVOR_WIN);
                server.multicast_real(&mut packet);
            },

            Ending::TimeOver => {
                info!("Exe won (time over)!");

                let mut packet = Packet::new(PacketType::SERVER_GAME_TIME_SYNC);
                packet.wu16(0);
                server.multicast(&mut packet);

                let mut packet = Packet::new(PacketType::SERVER_GAME_TIME_OVER);
                server.multicast_real(&mut packet);
            }
        }

        self.end_timer = 5 * 60;
    }

}