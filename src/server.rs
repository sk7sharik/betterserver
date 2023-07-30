use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::num::Wrapping;
use std::ops::AddAssign;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use log::{trace, debug, info, warn};
use rand::{thread_rng, Rng};

use crate::packet::PacketType;
use crate::states::lobby::Lobby;

use super::packet::Packet;
use super::state::State;

pub(crate) struct Server
{
    pub peers: Arc<RwLock<HashMap<u16, Arc<Mutex<Peer>>>>>,
    pub state: Arc<Mutex<Box<dyn State>>>,

    listener: Arc<Mutex<TcpListener>>,
    id_count: Arc<Mutex<Wrapping<u16>>>
}

impl Server {
    pub fn start(addr: &str) -> Arc<Mutex<Server>> 
    {
        let server = Arc::new(Mutex::new(Server {
            peers: Arc::new(RwLock::new(HashMap::new())), 
            state: Arc::new(Mutex::new(Box::new(Lobby::new()))), // Default state - Lobby

            listener: Arc::new(Mutex::new(TcpListener::bind(addr).unwrap())), 
            id_count: Arc::new(Mutex::new(Wrapping(0)))
        }));
        server.lock().unwrap().set_state(Box::new(Lobby::new()));

        // Tick thread
        let server_clone = server.clone();
        let state_clone = server.lock().unwrap().state.clone();
        thread::spawn(move || {
            Server::tick_worker(state_clone, server_clone);
        });

        // Worker thread
        let listener_clone = server.lock().unwrap().listener.clone();
        let server_clone = server.clone();
        thread::spawn(move || {
            Server::peer_worker(server_clone, listener_clone);
        });

        info!("Server listening at {}", addr);
        server
    }

    fn peer_worker(server: Arc<Mutex<Server>>, listener: Arc<Mutex<TcpListener>>) 
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
            let mut id: u16 = 1;
            {
                let server = server.lock().unwrap();
                let mut id_count = server.id_count.lock().unwrap();
                id_count.add_assign(1);

                if id_count.0 == 0 {
                    id_count.0 = 1;
                }

                id = id_count.0;
            }

            trace!("New connection from {:?} (ID {})", addr, id);
            let stream_clone = match stream.lock().unwrap().try_clone() {
                Ok(res) => res,
                Err(err) => {
                    warn!("Failed to open stream for {:?}: {}", addr, err);
                    continue;
                }
            };

            // Create new peer
            let peer = Peer { 
                id: id.clone(), 
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
                ready: false
            };

            server.lock().unwrap().peers.write().unwrap().insert(id, Arc::new(Mutex::new(peer)));
            
            // Listen for messages
            let peer = server.lock().unwrap().peers.write().unwrap().get(&id).unwrap().clone();
            let peers = server.lock().unwrap().peers.clone();
            let server = server.clone();
            Server::connected(&mut server.lock().unwrap(), &mut state.lock().unwrap(), peer);

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
                            trace!("Connection closed from {:?} due to: {}", addr, err);
                            
                            let peer = server.lock().unwrap().peers.write().unwrap().get(&id).unwrap().clone();
                            Server::disconnected(&mut server.lock().unwrap(), &mut state.lock().unwrap(), peer);
                            peers.write().unwrap().remove(&id);
                            break;
                        }
                    }

                    // Check connection close
                    if read <= 0 {
                        trace!("Connection closed from {:?}", addr);

                        let peer = server.lock().unwrap().peers.write().unwrap().get(&id).unwrap().clone();
                        Server::disconnected(&mut server.lock().unwrap(), &mut state.lock().unwrap(), peer);
                        peers.write().unwrap().remove(&id);
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
                                let peer = server.lock().unwrap().peers.write().unwrap().get(&id).unwrap().clone();
                                Server::got_packet(&mut server.lock().unwrap(), &mut state.lock().unwrap(), peer, &mut pak);

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
                                    let peer = server.lock().unwrap().peers.write().unwrap().get(&id).unwrap().clone();
                                    Server::got_packet(&mut server.lock().unwrap(), &mut state.lock().unwrap(), peer, &mut pak);
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
            state.lock().unwrap().tick(&mut server.lock().unwrap());
            thread::sleep(Duration::from_millis(15));
        }
    }

    pub fn multicast(&mut self, packet: &mut Packet) {
        for i in self.peers.write().unwrap().iter_mut() {
            i.1.lock().unwrap().send(packet);
        }
    }

    pub fn multicast_except(&mut self, packet: &mut Packet, id: u16) {
        for i in self.peers.write().unwrap().iter_mut() {
            if *i.0 == id {
                continue;
            }

            i.1.lock().unwrap().send(packet);
        }
    }

    pub fn set_state(&mut self, state: Box<dyn State>) {
        info!("Server state is now {}", state.name());
        let c_state = self.state.clone();
        *c_state.lock().unwrap() = state;
        c_state.lock().unwrap().init(self);
    }

    fn connected(server: &mut Server, state: &mut Box<dyn State>, peer: Arc<Mutex<Peer>>) 
    {
        state.connect(server, peer);
    }

    fn disconnected(server: &mut Server, state: &mut Box<dyn State>, peer: Arc<Mutex<Peer>>) 
    {
        state.disconnect(server, peer);
    }

    fn got_packet(server: &mut Server, state: &mut Box<dyn State>, peer: Arc<Mutex<Peer>>, packet: &mut Packet) 
    {  
        state.got_tcp_packet(server, peer, packet);
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

        match self.stream.write(packet.buf()) {
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


pub(crate) struct TcpPlayer {
    //TODO: fill this shit
}