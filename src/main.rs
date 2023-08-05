use std::{thread, time::Duration};

use chrono::Utc;
use config::CONFIG;
use gui::GUI;
use log::LevelFilter;
use log4rs::{append::{console::ConsoleAppender, file::FileAppender}, encode::pattern::PatternEncoder, Config, config::{Appender, Root}};
use server::Server;

mod config;
mod gui;
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

fn main() 
{
    init_logger();

    let _server = Server::start("0.0.0.0:7606", "0.0.0.0:8606");

    if CONFIG.gui {
        let mut gui = GUI::new();
        gui.run();
    }
    else {
        let dur = Duration::from_secs(2);
        loop {
            thread::sleep(dur);
        }
    }
}