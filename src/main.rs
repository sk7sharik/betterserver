use std::{thread, time::Duration};

use chrono::Local;
use log::LevelFilter;
use log4rs::{append::{console::ConsoleAppender, file::FileAppender}, encode::pattern::PatternEncoder, Config, config::{Appender, Root}};
use server::Server;

mod packet;
mod server;
mod entities;
mod map;
mod maps;
mod state;
mod states;
mod entity;

fn init_logger()
{
    let console = ConsoleAppender::builder().encoder(Box::new(PatternEncoder::default())).build();
    let logfile = FileAppender::builder()
    .encoder(Box::new(PatternEncoder::default()))
    .build(format!("log/{}.log", Local::now()))
    .unwrap();

    let config = Config::builder()
    .appender(Appender::builder().build("console", Box::new(console)))
    .appender(Appender::builder().build("logfile", Box::new(logfile)))
    .build(Root::builder().appender("console").appender("logfile").build(LevelFilter::Debug))
    .unwrap();
    
    log4rs::init_config(config).unwrap();
}

fn main() 
{
    init_logger();

    let _server = Server::start("0.0.0.0:7606", "0.0.0.0:8606");
    let dur = Duration::from_secs(2);
    
    loop {
        thread::sleep(dur);
    }
}