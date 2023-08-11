use std::{sync::{Arc, Mutex}, collections::HashMap};

use rand::{thread_rng, Rng, seq::SliceRandom};

use crate::{map::Map, states::game::{Game, find_entities}, server::{Server, Peer, real_peers}, entities::{slug::{Slug, SlugState, SlugRing}, shard::Shard}, packet::{Packet, PacketType}};

const SLUG_SPAWNS: [(i32, i32, bool); 11] = [
    (1901, 392, false),
    (2193, 392, false),
    (2468, 392, false),
    (1188, 860, false),
    (2577, 1952, false),
    (2564, 2264, false),
    (2782, 2264, false),
    (1441, 2264, false),
    (884, 2264, false),
    (988, 2004, false),
    (915, 2004, false),
];

const SHARD_SPAWNS: [(u16, u16); 12] = [
    (862, 248),
    (3078, 248),
    (292, 558),
    (2918, 558),
    (1100, 820),
    (980, 1188),
    (1870, 1252),
    (2180, 1508),
    (2920, 2216),
    (282, 2228),
    (1318, 1916),
    (3010, 1766)
];

pub(crate) struct RavineMist
{
    slugs: [(i32, i32, bool); 11],
    slug_timer: u16,

    shards_list: HashMap<u16, u8>
}

impl Map for RavineMist
{
    fn init(&mut self, server: &mut Server, game: &mut Game) 
    {
        self.slug_timer = thread_rng().gen_range(2..17) * 60;

        // Fill shards list
        for peer in real_peers!(server) {
            self.shards_list.insert(peer.lock().unwrap().id(), 0);
        }

        // Randomly shuffle and spawn shards
        let mut spawns = SHARD_SPAWNS.clone();
        spawns.shuffle(&mut thread_rng());
        for point in spawns.iter().take(7) {
            game.spawn(server, Box::new(Shard {
                x: point.0,
                y: point.1,
                spawned: false
            }));
        }
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game) 
    {
        self.slug_timer -= 1;

        if self.slugs.iter().any(|x| !x.2) && self.slug_timer <= 0 {

            let mut id: usize;
            loop {
                id = thread_rng().gen_range(0..self.slugs.len());

                if !self.slugs.get(id).unwrap().2 {
                    break;
                }
            }

            self.slugs[id].2 = true;
            game.spawn(server, Box::new(Slug {
                real_x: 0,
                x: self.slugs[id].0,
                y: self.slugs[id].1,
                id: id,
                state: SlugState::NoneLeft,
                ring: SlugRing::None
            }));

            self.slug_timer = (15 * 60) + (thread_rng().gen_range(2..10) * 60);
        }
    }

    fn got_tcp_packet(&mut self, _server: &mut Server, game: &mut Game, peer: Arc<Mutex<Peer>>, packet: &mut Packet) -> Result<(), &'static str> {
        let _passtrough = packet.ru8()? != 0; //TODO: get rid of
        let tp = packet.rpk()?;

        match tp {
            PacketType::CLIENT_RMZSLIME_HIT => {
                let eid = packet.ru16()?;
                let proj = packet.ru8()? != 0;

                for entity in find_entities!(game.entities.clone().lock().unwrap(), "slug") {                    
                    if *entity.0 != eid {
                        continue;
                    }

                    let slug = entity.1.as_any().downcast_ref::<Slug>().unwrap();
                    
                    self.slugs[slug.id].2 = false;
                    game.queue_destroy(entity.0);
                    
                    if proj {
                        break;
                    }

                    match slug.ring {
                        SlugRing::Ring => {
                            let mut packet = Packet::new(PacketType::SERVER_RMZSLIME_RINGBONUS);
                            packet.wu8(false as u8);
                            peer.lock().unwrap().send(&mut packet);
                        },

                        SlugRing::RedRing => {
                            let mut packet = Packet::new(PacketType::SERVER_RMZSLIME_RINGBONUS);
                            packet.wu8(true as u8);
                            peer.lock().unwrap().send(&mut packet);
                        },

                        _ => {}
                    }
                    break;
                }
            },

            _ => {}
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "Ravine Mist"
    }
    
    fn index(&self) -> usize {
        1
    }

    fn ring_count(&self) -> usize {
        27
    }
}

impl RavineMist
{
    pub fn new() -> RavineMist {
        RavineMist { 
            slugs: SLUG_SPAWNS.clone(),
            slug_timer: 0,
            shards_list: HashMap::new()
        }
    }
}