use log::info;
use rand::{thread_rng, Rng};

use crate::{entity::Entity, states::game::Game, server::Server, packet::{Packet, PacketType}};

#[derive(PartialEq)]
#[derive(Copy, Clone)]
pub(crate) enum SlugState 
{
    NoneRight,
    NoneLeft,
    RingRight,
    RingLeft,
    RedRingRight,
    RedRingLeft    
}

pub(crate) enum SlugRing
{
    None,
    Ring,
    RedRing
}

pub(crate) struct Slug
{
    pub x: i32,
    pub y: i32,
    pub state: SlugState,
    pub ring: SlugRing,
    pub pair: usize,
    pub realX: i32
}

impl Entity for Slug
{
    fn spawn(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        info!("Slug at ({}, {})", self.x, self.y);
        let rng = thread_rng().gen_range(0..100);

        if rng < 50 {
            self.ring = SlugRing::None;
            self.state = SlugState::NoneRight;
        }
        else if rng >= 50 && rng < 90 {
            self.ring = SlugRing::Ring;
            self.state = SlugState::RingRight;
        }
        else {
            self.ring = SlugRing::RedRing;
            self.state = SlugState::RedRingRight;
        }

        // randomize
        for _i in 0..thread_rng().gen_range(2..4) {
            self.swap();
        }

        let mut packet = Packet::new(PacketType::SERVER_RMZSLIME_STATE);
        packet.wu8(0u8);
        packet.wu16(*id);
        packet.wu16((self.x + self.realX) as u16);
        packet.wu16(self.y as u16);
        packet.wu8(self.state as u8);

        Some(packet)
    }

    fn tick(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        self.tick();

        let mut packet = Packet::new(PacketType::SERVER_RMZSLIME_STATE);
        packet.wu8(1u8);
        packet.wu16(*id);
        packet.wu16((self.x + self.realX) as u16);
        packet.wu16(self.y as u16);
        packet.wu8(self.state as u8);

        Some(packet)
    }

    fn destroy(&mut self, _server: &mut Server, _game: &mut Game, id: &u16) -> Option<Packet> 
    {
        let mut packet = Packet::new(PacketType::SERVER_RMZSLIME_STATE);
        packet.wu8(2u8);
        packet.wu16(*id);

        Some(packet)
    }

    fn id(&self) -> &str {
        "slug"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Slug
{
    fn tick(&mut self) {
        match self.state {
            SlugState::NoneLeft | SlugState::RingLeft | SlugState::RedRingLeft => {
                self.realX -= 1;

                if (self.realX) < -100 {
                    self.swap();
                }
            },

            SlugState::NoneRight | SlugState::RingRight | SlugState::RedRingRight => {
                self.realX += 1;

                if (self.realX) > 100 {
                    self.swap();
                }
            },
        }
    }

    fn swap(&mut self) {
        match self.state {
            SlugState::NoneLeft => {
                self.state = SlugState::NoneRight;
            },
            
            SlugState::NoneRight => {
                self.state = SlugState::NoneLeft;
            },

            SlugState::RingLeft => {
                self.state = SlugState::RingRight;
            },
            
            SlugState::RingRight => {
                self.state = SlugState::RingLeft;
            },
            
            SlugState::RedRingLeft => {
                self.state = SlugState::RedRingRight;
            },
            
            SlugState::RedRingRight => {
                self.state = SlugState::RedRingLeft;
            },
        }
    }
}