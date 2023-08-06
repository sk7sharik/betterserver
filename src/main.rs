use std::{net::TcpListener, sync::{Mutex, Arc}, ops::AddAssign};

use chrono::Utc;
use config::CONFIG;
use log::{LevelFilter, error, warn, info};
use log4rs::{append::{console::ConsoleAppender, file::FileAppender}, encode::pattern::PatternEncoder, Config, config::{Appender, Root}};
use server::Server;

mod config;
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
    let console = ConsoleAppender::builder().encoder(Box::new(PatternEncoder::default())).build();
    let logfile = FileAppender::builder()
    .encoder(Box::new(PatternEncoder::default()))
    .build(format!("logs/{}.log", Utc::now().format("%Y-%m-%d %H-%M-%S")))
    .unwrap();

    let config = Config::builder()
    .appender(Appender::builder().build("console", Box::new(console)))
    .appender(Appender::builder().build("logfile", Box::new(logfile)))
    .build(Root::builder().appender("console").appender("logfile").build(LevelFilter::Debug))
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
            return servers.last().unwrap().clone();
        }

        port.add_assign(1);
        let server = Server::start(*port);
        servers.push(server.clone());

        info!("Growing servers...");
        server
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
            error!("Failed to bind socket: {}", err);
            return;
        }
    };

    servers.push(Server::start(CONFIG.server.udp_port));
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