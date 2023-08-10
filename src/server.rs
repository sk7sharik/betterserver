use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, SocketAddr, UdpSocket};
use std::num::Wrapping;
use std::ops::AddAssign;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

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
        $server.peers.clone().read().unwrap().values().filter(|x| { let peer = x.lock().unwrap(); return !peer.in_queue && !peer.pending})
    };
}

macro_rules! assert_or_disconnect {
    ($expr: expr, $peer: expr) => {
        if (!$expr) {
            $peer.disconnect(format!("assert_or_disconnect failed: \'{}\'! ", stringify!($expr)).as_str());
            return Ok(());
        }
    };
}

pub(crate) use real_peers;
pub(crate) use assert_or_disconnect;
pub(crate) struct Server
{
    pub udp_port: u16,
    pub name: String,
    pub peers: Arc<RwLock<HashMap<u16, Arc<Mutex<Peer>>>>>,
    pub state: Arc<Mutex<Box<dyn State>>>,
    pub udp_socket: Arc<Mutex<UdpSocket>>,
    
    id_count: Arc<Mutex<Wrapping<u16>>>,
    next_state: Arc<Mutex<Option<Mutex<Box<Box<dyn State + 'static>>>>>>
}

impl Server {
    pub fn start(udp_port: u16, name: String) -> Arc<Mutex<Server>> 
    {
        let server = Arc::new(Mutex::new(Server {
            udp_port: udp_port,
            name: name.clone(),
            
            peers: Arc::new(RwLock::new(HashMap::new())), 
            state: Arc::new(Mutex::new(Box::new(Lobby::new()))), // Default state - Lobby

            udp_socket: Arc::new(Mutex::new(UdpSocket::bind(format!("0.0.0.0:{}", udp_port)).unwrap())), 
            id_count: Arc::new(Mutex::new(Wrapping(0))),
            next_state: Arc::new(Mutex::new(None))
        }));
        

        // Tick thread
        let server_clone = server.clone();
        let state_clone = server.lock().unwrap().state.clone();
        let _ = thread::Builder::new().name(name.clone()).spawn(move || {
            Server::tick_worker(state_clone, server_clone);
        });

        // UDP thread
        let listener_clone = server.lock().unwrap().udp_socket.clone();
        let server_clone = server.clone();
        let state = server.lock().unwrap().state.clone();

        let _ = thread::Builder::new().name(name.clone()).spawn(move || {
            info!("Listening at {}/udp", udp_port);
            Server::udp_worker(server_clone, state, listener_clone);
        });

        server
    }

    pub fn peer_redirect(server: Arc<Mutex<Server>>, stream: TcpStream) -> bool
    {
        let name = server.lock().unwrap().name.clone();
        let _ = thread::Builder::new().name(name).spawn(move || {
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
                    return false;
                }
            };

            // Create new peer
            let peer = Peer { 
                id: _id.clone(), 
                stream: stream_clone, 
                addr, 
                nickname: String::new(),
                udid: String::new(),
                exe_chance: thread_rng().gen_range(2..5),
                timer: 0, 
                lobby_icon: 0, 
                pet: 0,
                pending: true,
                in_queue: true,
                ready: false,
                player: None
            };

            // Listen for messages
            let peer = Arc::new(Mutex::new(peer));
            let peers = server.lock().unwrap().peers.clone();
            let server = server.clone();
            info!("{:?} connected. (ID {})", peer.lock().unwrap().addr(), _id);
            Server::connected(&mut server.lock().unwrap(), state.clone(), peer.clone());

            let mut in_buffer = [0; 256];

            let mut pak_buffer: Vec<u8> = Vec::new();
            let mut pak_size: usize = 0; 
            let mut pak_exsize: usize = 0;

            loop {
                // Reading incoming messages
                let mut read: usize = 0;
                match stream.lock().unwrap().read(&mut in_buffer)
                {
                    Ok(sz) => read = sz,
                    Err(err) => {
                        info!("{:?} disconnected (ID {}): {}", addr, _id, err);
                        
                        peers.write().unwrap().remove(&_id);
                        Server::disconnected(&mut server.lock().unwrap(), state.clone(), peer.clone());
                        break true;
                    }
                }

                // Check connection close
                if read <= 0 {
                    info!("{:?} disconnected (ID {})", addr, _id);

                    peers.write().unwrap().remove(&_id);
                    Server::disconnected(&mut server.lock().unwrap(), state.clone(), peer.clone());
                    break true;
                }

                let mut pos = 0;
                while pos < read {
                    if pak_size <= 0 {
                        pak_size = in_buffer[pos] as usize;
                        pak_exsize = pak_size.clone();

                        pak_buffer.clear();
                        pos += 1;
                    }
                    else {
                        pak_buffer.push(in_buffer[pos]);
                        pak_size -= 1;
                        pos += 1;

                        if pak_size <= 0 {
                            debug!("Packet ok (len {})", pak_exsize);

                            let mut pak = Packet::from(&pak_buffer, pak_buffer.len());
                            Server::got_tcp_packet(&mut server.lock().unwrap(), state.clone(), peer.clone(), &mut pak);
                            pak_buffer.clear();
                        }
                    }
                }
            }
        });

        true
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

            if size <= 0 {
                continue;
            }

            let mut packet = Packet::from(&buf, size);
            Server::got_udp_packet(&mut server.lock().unwrap(), state.clone(), &src, &mut packet);
        }
    }

    fn tick_worker(state: Arc<Mutex<Box<dyn State>>>, server: Arc<Mutex<Server>>) 
    {
        loop {
            let last_update = Instant::now();
            { 
                let server = &mut server.lock().unwrap();
                let next_state = state.lock().unwrap().tick(server);
                check_state!(next_state, server);
            }
            
            while last_update.elapsed().as_nanos() < (15u128 * 1000000u128) {
                thread::sleep(Duration::from_nanos(100000u64));
            }
        }
    }

    pub fn udp_send(&mut self, recv: &SocketAddr, packet: &mut Packet) 
    {
        match self.udp_socket.lock().unwrap().send_to(&packet.raw(), recv)
        {
            Ok(_) => {},
            Err(err) => {
                warn!("Failed to send packet to {}: {}", recv, err);
            }
        }
    }

    pub fn udp_multicast(&mut self, recvs: &HashMap<u16, SocketAddr>, packet: &mut Packet) 
    {
        for recv in recvs.values() {
            match self.udp_socket.lock().unwrap().send_to(packet.raw(), recv)
            {
                Ok(_) => {},
                Err(err) => {
                    warn!("Failed to send packet to {}: {}", recv, err);
                }
            }
        }
    }

    pub fn udp_multicast_except(&mut self, recvs: &HashMap<u16, SocketAddr>, packet: &mut Packet, except: &SocketAddr) 
    {
        for recv in recvs.values() {
            if *recv == *except {
                continue;
            }

            match self.udp_socket.lock().unwrap().send_to(packet.raw(), recv)
            {
                Ok(_) => {},
                Err(err) => {
                    warn!("Failed to send packet to {}: {}", recv, err);
                }
            }
        }
    }

    pub fn multicast(&mut self, packet: &mut Packet) {
        for i in self.peers.read().unwrap().iter() {
            i.1.lock().unwrap().send(packet);
        }
    }

    pub fn multicast_real(&mut self, packet: &mut Packet) {
        for i in self.peers.read().unwrap().iter() {
            if i.1.lock().unwrap().in_queue {
                continue;
            }

            i.1.lock().unwrap().send(packet);
        }
    }

    pub fn multicast_except(&mut self, packet: &mut Packet, id: u16) {
        for i in self.peers.read().unwrap().iter() {
            if *i.0 == id {
                continue;
            }

            i.1.lock().unwrap().send(packet);
        }
    }

    pub fn multicast_real_except(&mut self, packet: &mut Packet, id: u16) {
        for i in self.peers.read().unwrap().iter() {
            if i.1.lock().unwrap().in_queue {
                continue;
            }

            if *i.0 == id {
                continue;
            }

            i.1.lock().unwrap().send(packet);
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
        let result = state.lock().unwrap().got_tcp_packet(server, peer.clone(), packet);

        if !result.is_ok() {
            peer.lock().unwrap().disconnect(format!("got_tcp_packet failed: {}", result.err().unwrap()).as_str());
        }
    }

    fn got_udp_packet(server: &mut Server, state: Arc<Mutex<Box<dyn State>>>, addr: &SocketAddr, packet: &mut Packet)
    {
        let result = state.lock().unwrap().got_udp_packet(server, addr, packet);
        
        if !result.is_ok() {
            warn!("got_udp_packet failed: {}", result.err().unwrap());
        }
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

        match self.stream.write(&packet.sized()) {
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

        info!("Disconnected {} because \"{}\"", self.addr, reason);
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
    pub death_timer: i32,
    pub escaped: bool,
    pub dead: bool,
    pub red_ring: bool,
    pub revival: PlayerRevival
}

pub(crate) struct PlayerRevival
{
    pub progress: f64,
    pub initiators: Vec<u16>
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
            death_timer: 0,
            escaped: false, 
            dead: false, 
            red_ring: false,
            revival: PlayerRevival { progress: 0.0, initiators: Vec::new() }
        }
    }
}