use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr, UdpSocket};
use std::num::Wrapping;
use std::ops::AddAssign;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use log::{trace, debug, info, warn};
use num_derive::FromPrimitive;
use rand::{thread_rng, Rng};

use crate::packet::PacketType;
use crate::states::lobby::Lobby;

use super::packet::Packet;
use super::state::State;

macro_rules! check_state {
    ($next_state: expr, $server: expr) => {
        if $next_state.is_some() {
            let mut state = $next_state.unwrap();
            let next_state = state.init($server);

            if next_state.is_some() {
                let state = next_state.unwrap();
                log::info!("Changing state to [{}].", state.name());
                *$server.state.lock().unwrap() = state;
            }
            else {
                log::info!("Changing state to [{}].", state.name());
                *$server.state.lock().unwrap() = state;
            }
        }
    };
}

// Peers that aren't in queue
macro_rules! real_peers {
    ($server: expr) => {
        $server.peers.read().unwrap().values().filter(|x| !x.lock().unwrap().in_queue)
    };
}

macro_rules! real_peers_mut {
    ($server: expr) => {
        $server.peers.write().unwrap().values().filter(|x| !x.lock().unwrap().in_queue)
    };
}

macro_rules! assert_or_disconnect {
    ($expr: expr, $peer: expr) => {
        if (!$expr) {
            $peer.disconnect(format!("assert_or_disconnect failed: \"{}\"!", stringify!($expr)).as_str());
            return None;
        }
    };
}

pub(crate) use real_peers;
pub(crate) use real_peers_mut;
pub(crate) use assert_or_disconnect;


pub(crate) struct Server
{
    pub peers: Arc<RwLock<HashMap<u16, Arc<Mutex<Peer>>>>>,
    pub state: Arc<Mutex<Box<dyn State>>>,

    tcp_listener: Arc<Mutex<TcpListener>>,
    udp_socket: Arc<Mutex<UdpSocket>>,
    id_count: Arc<Mutex<Wrapping<u16>>>,
    next_state: Arc<Mutex<Option<Mutex<Box<Box<dyn State + 'static>>>>>>
}

impl Server {
    pub fn start(tcp_addr: &str, udp_addr: &str) -> Arc<Mutex<Server>> 
    {
        let server = Arc::new(Mutex::new(Server {
            peers: Arc::new(RwLock::new(HashMap::new())), 
            state: Arc::new(Mutex::new(Box::new(Lobby::new()))), // Default state - Lobby

            tcp_listener: Arc::new(Mutex::new(TcpListener::bind(tcp_addr).unwrap())), 
            udp_socket: Arc::new(Mutex::new(UdpSocket::bind(udp_addr).unwrap())), 
            id_count: Arc::new(Mutex::new(Wrapping(0))),
            next_state: Arc::new(Mutex::new(None))
        }));

        // Tick thread
        let server_clone = server.clone();
        let state_clone = server.lock().unwrap().state.clone();
        thread::spawn(move || {
            Server::tick_worker(state_clone, server_clone);
        });

        // TCP thread
        let listener_clone = server.lock().unwrap().tcp_listener.clone();
        let server_clone = server.clone();
        thread::spawn(move || {
            Server::tcp_worker(server_clone, listener_clone);
        });

        // UDP thread
        let listener_clone = server.lock().unwrap().udp_socket.clone();
        let server_clone = server.clone();
        let state = server.lock().unwrap().state.clone();

        thread::spawn(move || {
            Server::udp_worker(server_clone, state, listener_clone);
        });

        info!("Server listening at (TCP {}, UDP {})", tcp_addr, udp_addr);
        server
    }

    fn udp_worker(server: Arc<Mutex<Server>>, state: Arc<Mutex<Box<dyn State>>>, listener: Arc<Mutex<UdpSocket>>)
    {
        loop {
            let mut buf = [0; 256];
            let (size, src) = match listener.lock().unwrap().recv_from(&mut buf)
            {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to read from UDP connection: {}", err);
                    continue;
                }
            };

            let mut packet = Packet::from(&buf, size);
            Server::got_udp_packet(&mut server.lock().unwrap(), state.clone(), &src, &mut packet);
        }
    }

    fn tcp_worker(server: Arc<Mutex<Server>>, listener: Arc<Mutex<TcpListener>>) 
    {
        for stream in listener.lock().unwrap().incoming() {
            // If failed to open a stream, ignore
            let stream = match stream {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to open stream: {}", err);
                    continue;
                }
            };

            let stream = Arc::new(Mutex::new(stream));
            let addr = stream.lock().unwrap().peer_addr().unwrap();
            let state = server.lock().unwrap().state.clone();

            // Generate ID
            let mut _id: u16 = 1;
            {
                let server = server.lock().unwrap();
                let mut id_count = server.id_count.lock().unwrap();
                id_count.add_assign(1);

                if id_count.0 == 0 {
                    id_count.0 = 1;
                }

                _id = id_count.0;
            }

            trace!("New connection from {:?} (ID {})", addr, _id);
            let stream_clone = match stream.lock().unwrap().try_clone() {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to open stream for {:?}: {}", addr, err);
                    continue;
                }
            };

            // Create new peer
            let peer = Peer { 
                id: _id.clone(), 
                stream: stream_clone, 
                addr, 
                nickname: String::new(),
                udid: String::new(),
                exe_chance: thread_rng().gen_range(0..5),
                timer: 0, 
                lobby_icon: 0, 
                pet: 0,
                pending: true,
                in_queue: false,
                ready: false,
                player: None
            };

            server.lock().unwrap().peers.write().unwrap().insert(_id, Arc::new(Mutex::new(peer)));
            
            // Listen for messages
            let peer = server.lock().unwrap().peers.write().unwrap().get(&_id).unwrap().clone();
            let peers = server.lock().unwrap().peers.clone();
            let server = server.clone();
            info!("{:?} connected. (ID {})", peer.lock().unwrap().addr(), _id);
            Server::connected(&mut server.lock().unwrap(), state.clone(), peer);

            thread::spawn(move || {

                let mut in_buffer = [0; 256];

                let mut pak_buffer: Vec<u8> = Vec::new();
                let mut pak_start = false;
                let mut pak_size: usize = 0;

                loop {
                    // Reading incoming messages
                    let mut read: usize = 0;
                    match stream.lock().unwrap().read(&mut in_buffer)
                    {
                        Ok(sz) => read = sz,
                        Err(err) => {
                            trace!("{:?} disconnected (ID {}): {}", addr, _id, err);
                            
                            let peer = server.lock().unwrap().peers.write().unwrap().get(&_id).unwrap().clone();
                            Server::disconnected(&mut server.lock().unwrap(), state.clone(), peer);
                            peers.write().unwrap().remove(&_id);
                            break;
                        }
                    }

                    // Check connection close
                    if read <= 0 {
                        trace!("{:?} disconnected (ID {})", addr, _id);

                        let peer = server.lock().unwrap().peers.write().unwrap().get(&_id).unwrap().clone();
                        Server::disconnected(&mut server.lock().unwrap(), state.clone(), peer);
                        peers.write().unwrap().remove(&_id);
                        break;
                    }

                    while read > 0 {
                        if !pak_start {

                            pak_buffer.clear();
                            pak_size = in_buffer[0] as usize;
                        
                            debug!("Packet {}", pak_size);

                            if read - 1 >= pak_size {
                                debug!("Packet ok");

                                let data = &in_buffer[1..];
                                let mut pak = Packet::from(data, pak_size);
                                let peer = server.lock().unwrap().peers.write().unwrap().get(&_id).unwrap().clone();
                                Server::got_tcp_packet(&mut server.lock().unwrap(), state.clone(), peer, &mut pak);

                                read -= pak_size + 1;
                            } else {
                                debug!("Packet split");
                                pak_start = true;

                                // Read everything that we got left
                                for i in &in_buffer[1..] {
                                    pak_buffer.push(*i);
                                }

                                pak_size -= read - 1;
                                read = 0;
                            }
                        }
                        else {
                            debug!("Packet part arrived");
                            
                            for i in &in_buffer[1..] {
                                pak_buffer.push(*i);
                                pak_size -= 1;
                                read -= 1;

                                if pak_size <= 0 {
                                    debug!("Packet ok");

                                    let mut pak = Packet::from(&pak_buffer, pak_size);
                                    let peer = server.lock().unwrap().peers.write().unwrap().get(&_id).unwrap().clone();
                                    Server::got_tcp_packet(&mut server.lock().unwrap(), state.clone(), peer, &mut pak);
                                    pak_start = false;
                                    pak_buffer.clear();
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    fn tick_worker(state: Arc<Mutex<Box<dyn State>>>, server: Arc<Mutex<Server>>) 
    {
        loop {
            let next_state = state.lock().unwrap().tick(&mut server.lock().unwrap());
            check_state!(next_state, &mut server.lock().unwrap());
            
            thread::sleep(Duration::from_millis(15));
        }
    }

    pub fn udp_send(&mut self, recv: &SocketAddr, packet: &mut Packet) 
    {
        match self.udp_socket.lock().unwrap().send_to(&packet.buf(), recv)
        {
            Ok(_) => {},
            Err(err) => {
                warn!("Failed to send packet to {}: {}", recv, err);
            }
        }
    }

    pub fn udp_multicast(&mut self, recvs: &Vec<SocketAddr>, packet: &mut Packet) 
    {
        for recv in recvs {
            match self.udp_socket.lock().unwrap().send_to(&packet.buf(), recv)
            {
                Ok(_) => {},
                Err(err) => {
                    warn!("Failed to send packet to {}: {}", recv, err);
                }
            }
        }
    }

    pub fn multicast(&mut self, packet: &mut Packet) {
        for i in self.peers.write().unwrap().iter_mut() {
            i.1.lock().unwrap().send(packet);
            debug!("Sent packet to {}", *i.0);
        }
    }

    pub fn multicast_real(&mut self, packet: &mut Packet) {
        for i in self.peers.write().unwrap().iter_mut() {
            if i.1.lock().unwrap().in_queue {
                continue;
            }

            i.1.lock().unwrap().send(packet);
            debug!("Sent packet to {}", *i.0);
        }
    }

    pub fn multicast_except(&mut self, packet: &mut Packet, id: u16) {
        for i in self.peers.write().unwrap().iter_mut() {
            if *i.0 == id {
                continue;
            }

            i.1.lock().unwrap().send(packet);
            debug!("Sent packet to {}", *i.0);
        }
    }

    pub fn multicast_real_except(&mut self, packet: &mut Packet, id: u16) {
        for i in self.peers.write().unwrap().iter_mut() {
            if i.1.lock().unwrap().in_queue {
                continue;
            }

            if *i.0 == id {
                continue;
            }

            i.1.lock().unwrap().send(packet);
            debug!("Sent packet to {}", *i.0);
        }
    }

    fn connected(server: &mut Server, state: Arc<Mutex<Box<dyn State>>>, peer: Arc<Mutex<Peer>>) 
    {
        let next_state = state.lock().unwrap().connect(server, peer);
        check_state!(next_state, server);
    }

    fn disconnected(server: &mut Server, state: Arc<Mutex<Box<dyn State>>>, peer: Arc<Mutex<Peer>>) 
    {
        let next_state = state.lock().unwrap().disconnect(server, peer);
        check_state!(next_state, server);
    }

    fn got_tcp_packet(server: &mut Server, state: Arc<Mutex<Box<dyn State>>>, peer: Arc<Mutex<Peer>>, packet: &mut Packet) 
    {  
        let next_state = state.lock().unwrap().got_tcp_packet(server, peer, packet);
        check_state!(next_state, server);
    }

    fn got_udp_packet(server: &mut Server, state: Arc<Mutex<Box<dyn State>>>, addr: &SocketAddr, packet: &mut Packet)
    {
        let next_state = state.lock().unwrap().got_udp_packet(server, addr, packet);
        check_state!(next_state, server);
    }
}

pub(crate) struct Peer {
    pub timer: u16,
    pub nickname: String,
    pub udid: String,
    pub lobby_icon: u8,
    pub pet: i8,
    pub pending: bool,
    pub ready: bool,
    pub in_queue: bool,
    pub exe_chance: u8,

    /* Player */
    pub player: Option<Player>,

    id: u16,

    stream: TcpStream,
    addr: SocketAddr
}

impl Peer {
    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn send(&mut self, packet: &mut Packet) -> bool {

        match self.stream.write(&packet.buf()) {
            Ok(_) => {},
            Err(err) => {
                warn!("Couldn't write to a stream: {}", err);
                return false;
            }
        }
        true
    }

    pub fn disconnect(&mut self, reason: &str) -> bool {
        let mut pak = Packet::new(PacketType::SERVER_PLAYER_FORCE_DISCONNECT);
        pak.wstr(reason);
        self.send(&mut pak);

        match self.stream.shutdown(std::net::Shutdown::Both) {
            Ok(_) => {},
            Err(error) => {
                warn!("Failed to disconnect: {}", error);
                return false;
            }
        }

        trace!("Disconnected {} because \"{}\"", self.addr, reason);
        true
    }

}

#[derive(PartialEq)]
#[derive(Copy, Clone)]
#[derive(FromPrimitive)]
#[derive(Debug)]
pub(crate) enum SurvivorCharacter
{
    None = -1,

    Exe = 0,
    Tails = 1,
    Knuckles = 2,
    Eggman = 3,
    Amy = 4,
    Cream = 5,
    Sally = 6,
}

#[derive(PartialEq)]
#[derive(Copy, Clone)]
#[derive(FromPrimitive)]
#[derive(Debug)]
pub(crate) enum ExeCharacter
{
    None = -1,

    // why exe characters start at 0 :skull:
    Original = 0,
    Chaos,
    Exetior,
    Exeller
}

pub(crate) struct Player 
{
    pub ch1: SurvivorCharacter,
    pub ch2: ExeCharacter,
    pub exe: bool,

    pub revival_times: u8,
    pub death_timer: f32,
    pub escaped: bool,
    pub alive: bool,
    pub red_ring: bool,
    pub can_demonize: bool,
    pub invisible: bool
}

impl Player 
{
    pub fn new() -> Player 
    {
        Player 
        { 
            ch1: SurvivorCharacter::None, 
            ch2: ExeCharacter::None, 
            exe: false,
            
            revival_times: 0, 
            death_timer: 0.0, 
            escaped: false, 
            alive: true, 
            red_ring: false, 
            can_demonize: true, 
            invisible: false 
        }
    }
}