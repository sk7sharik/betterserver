use std::{net::TcpListener, sync::{Mutex, Arc}, ops::AddAssign};

use chrono::Utc;
use config::CONFIG;
use log::{LevelFilter, error, warn, info};
use log4rs::{append::{console::ConsoleAppender, file::FileAppender}, encode::pattern::PatternEncoder, Config, config::{Appender, Root}};
use server::Server;

mod config;
mod timer;
mod server;
mod packet;
mod map;
mod entity;
mod state;
mod maps;
mod entities;
mod states;

fn init_logger()
{
    let mut level = LevelFilter::Info;

    if CONFIG.debug {
        level = LevelFilter::Debug;
    }

    let console = ConsoleAppender::builder()
    .encoder(Box::new(PatternEncoder::new("[{h({l})} {T} {d(%Y-%m-%d %H:%M:%S)}] {m} {n}")))
    .build();

    let logfile = FileAppender::builder()
    .encoder(Box::new(PatternEncoder::new("[{h({l})} {T} {d(%Y-%m-%d %H:%M:%S)}] {m} {n}")))
    .build(format!("logs/{}.log", Utc::now().format("%Y-%m-%d %H-%M-%S")))
    .unwrap();

    let config = Config::builder()
    .appender(Appender::builder().build("console", Box::new(console)))
    .appender(Appender::builder().build("logfile", Box::new(logfile)))
    .build(Root::builder().appender("console").appender("logfile").build(level))
    .unwrap();
    
    log4rs::init_config(config).unwrap();
}

fn find_free_server(servers: &mut Vec<Arc<Mutex<Server>>>, port: &mut u16) -> Arc<Mutex<Server>>
{
    if CONFIG.server.grow {
        for server in servers.iter() {
            if server.lock().unwrap().peers.read().unwrap().len() < 7 {
                return server.clone();
            }
        }
        
        if servers.len() >= CONFIG.server.grow_limit as usize {
            warn!("Couldn't allocate new server: FULL ({}/{})!", servers.len(), CONFIG.server.grow_limit);
            return servers.last().unwrap().clone();
        }

        info!("Allocating new sub-server... ({}/{})", servers.len() + 1, CONFIG.server.grow_limit);
        port.add_assign(1);

        let server = Server::start(*port, format!("server{}", servers.len()));
        servers.push(server.clone());
        return server;
    }
    else {
        return servers.get(0).unwrap().clone();
    }
}

fn main()
{
    init_logger();

    let mut servers: Vec<Arc<Mutex<Server>>> = Vec::new();
    let mut port = CONFIG.server.udp_port;
    let listner = match TcpListener::bind(format!("0.0.0.0:{}", CONFIG.server.tcp_port))
    {
        Ok(res) => res,
        Err(err) => {
            error!("Failed to bind listener: {}", err);
            return;
        }
    };

    info!("Listening for connections on {}/tcp", CONFIG.server.tcp_port);
    for stream in listner.incoming()
    {
        // If failed to open a stream, ignore
        let stream = match stream {
            Ok(res) => res,
            Err(err) => {
                warn!("Failed to open stream: {}", err);
                continue;
            }
        };

        let server = find_free_server(&mut servers, &mut port);
        Server::peer_redirect(server, stream);
    }
}