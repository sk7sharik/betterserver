use std::sync::{Arc, Mutex};

use rand::{thread_rng, Rng};

use crate::{map::Map, states::game::{Game, find_entities}, server::{Server, Peer}, entities::slug::{Slug, SlugState, SlugRing}, packet::{Packet, PacketType}};

static SLUG_SPAWNS: [(i32, i32, bool); 11] = [
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

pub(crate) struct RavineMist
{
    slugs: [(i32, i32, bool); 11],
    slug_timer: u16
}

impl Map for RavineMist
{
    fn init(&mut self, server: &mut Server, game: &mut Game) 
    {
        self.slug_timer = thread_rng().gen_range(2..17) * 60;
    }

    fn tick(&mut self, server: &mut Server, game: &mut Game) 
    {
        self.slug_timer -= 1;

        if self.slug_timer <= 0 {
            let pair = self.find_free();
            
            game.spawn(server, Box::new(Slug { 
                x: self.slugs[pair].0,
                y: self.slugs[pair].1,
                pair: pair,
                state: SlugState::NoneRight,
                ring: SlugRing::None,
                realX: 0
            }));

            self.slug_timer = (15 * 60) + (thread_rng().gen_range(2..10) * 60);
        }
    }

    fn got_tcp_packet(&mut self, _server: &mut Server, game: &mut Game, peer: Arc<Mutex<Peer>>, packet: &mut Packet) {
        let _passtrough = packet.ru8() != 0; //TODO: get rid of
        let tp = packet.rpk();

        match tp {
            PacketType::CLIENT_RMZSLIME_HIT => {
                let eid = packet.ru16();
                let proj = packet.ru8() != 0;

                for entity in find_entities!(game.entities.clone().lock().unwrap(), "slug") {                    
                    if *entity.0 != eid {
                        continue;
                    }

                    let slug = entity.1.as_any().downcast_ref::<Slug>().unwrap();
                    self.slugs[slug.pair].2 = false;
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
            slug_timer: 0 
        }
    }

    fn find_free(&mut self) -> usize {
        let mut ind = 0;
        for point in self.slugs.iter_mut() {
            if !point.2 {
                point.2 = true;
                return ind;
            }

            ind += 1;
        }

        0
    }
}