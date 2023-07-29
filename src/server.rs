use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::num::Wrapping;
use std::ops::AddAssign;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::states::lobby::Lobby;

use super::packet::Packet;
use super::state::State;

pub(crate) struct Server
{
    pub peers: Arc<Mutex<HashMap<u16, Peer>>>,
    pub state: Arc<Mutex<Box<dyn State>>>,

    listener: Arc<Mutex<TcpListener>>,
    id_count: Arc<Mutex<Wrapping<u16>>>
}

impl Server {
    pub fn start(addr: &str) -> Arc<Mutex<Server>> {
        let server = Arc::new(Mutex::new(Server {
            peers: Arc::new(Mutex::new(HashMap::new())), 
            state: Arc::new(Mutex::new(Box::new(Lobby::new()))), // Default state - Lobby

            listener: Arc::new(Mutex::new(TcpListener::bind(addr).unwrap())), 
            id_count: Arc::new(Mutex::new(Wrapping(0)))
        }));

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

        println!("Server listening at {}", addr);
        server
    }

    fn peer_worker(server: Arc<Mutex<Server>>, listener: Arc<Mutex<TcpListener>>) {
        for stream in listener.lock().unwrap().incoming() {
            // If failed to open a stream, ignore
            let stream = match stream {
                Ok(res) => res,
                Err(_) => {
                    continue;
                }
            };

            let stream = Arc::new(Mutex::new(stream));
            let addr = stream.lock().unwrap().peer_addr().unwrap();
            let state = server.lock().unwrap().state.clone();

            // Generate ID
            server.lock().unwrap().id_count.lock().unwrap().add_assign(1);
            let id = *server.lock().unwrap().id_count.lock().unwrap();
            println!("New connection from {:?} (ID {})", addr, id);

            // Create new peer
            let mut peer = Peer { id: id.0.clone(), stream: stream.clone(), addr };
            Server::connected(&mut server.lock().unwrap(), &mut state.lock().unwrap(),&mut peer);
            server.lock().unwrap().peers.lock().unwrap().insert(id.0, peer);
            
            // Listen for messages
            let peers = server.lock().unwrap().peers.clone();
            let server = server.clone();

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
                            println!("Connection closed from {:?} due to: {}", addr, err);
                            Server::disconnected(&mut server.lock().unwrap(), &mut state.lock().unwrap(), &mut peers.lock().unwrap().get_mut(&id.0).unwrap());
                            peers.lock().unwrap().remove(&id.0);
                            break;
                        }
                    }

                    // Check connection close
                    if read <= 0 {
                        println!("Connection closed from {:?}", addr);
                        Server::disconnected(&mut server.lock().unwrap(), &mut state.lock().unwrap(), &mut peers.lock().unwrap().get_mut(&id.0).unwrap());
                        peers.lock().unwrap().remove(&id.0);
                        break;
                    }

                    while read > 0 {
                        if !pak_start {

                            pak_buffer.clear();
                            pak_size = in_buffer[0] as usize;
                        
                            println!("Packet {}", pak_size);

                            if read - 1 >= pak_size {
                                println!("Packet ok");

                                let data = &in_buffer[1..];
                                let mut pak = Packet::from(data, pak_size);
                                Server::got_packet(&mut server.lock().unwrap(), &mut state.lock().unwrap(), &mut peers.lock().unwrap().get_mut(&id.0).unwrap(), &mut pak);

                                read -= pak_size + 1;
                            } else {
                                println!("Packet split");
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
                            println!("Packet part arrived");
                            
                            for i in &in_buffer[1..] {
                                pak_buffer.push(*i);
                                pak_size -= 1;
                                read -= 1;

                                if pak_size <= 0 {
                                    println!("Packet ok");

                                    let mut pak = Packet::from(&pak_buffer, pak_size);
                                    Server::got_packet(&mut server.lock().unwrap(), &mut state.lock().unwrap(), &mut peers.lock().unwrap().get_mut(&id.0).unwrap(), &mut pak);

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

    fn tick_worker(state: Arc<Mutex<Box<dyn State>>>, server: Arc<Mutex<Server>>) {
        loop {
            state.lock().unwrap().tick(&mut server.lock().unwrap());
            thread::sleep(Duration::from_millis(15));
        }
    }

    fn connected(server: &mut Server, state: &mut Box<dyn State>, peer: &mut Peer) {
        state.connect(server, peer);
    }

    fn disconnected(server: &mut Server, state: &mut Box<dyn State>, peer: &mut Peer) {
        state.disconnect(server, peer);
    }

    fn got_packet(server: &mut Server, state: &mut Box<dyn State>, peer: &mut Peer, packet: &mut Packet) {  
        state.got_tcp_packet(server, peer, packet);
    }
}

pub(crate) struct Peer {
    id: u16,
    stream: Arc<Mutex<TcpStream>>,
    addr: SocketAddr,
}

impl Peer {
    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn send(&self, mut packet : Packet) -> bool {
        let mut stream = match self.stream.lock() {
            Ok(stream) => stream,
            Err(err) => {
                println!("Couldn't write to a stream: {}", err);
                return false;
            }
        };

        match stream.write(packet.buf()) {
            Ok(_) => {},
            Err(err) => {
                println!("Couldn't write to a stream: {}", err);
                return false;
            }
        }
        true
    }
}


pub(crate) struct TcpPlayer {
    //TODO: fill this shit
}