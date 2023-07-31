use server::Server;

mod packet;
mod server;
mod map;
mod maps;
mod state;
mod states;

fn main() 
{
    log4rs::init_file("logging_config.yaml", Default::default()).unwrap();

    let server = Server::start("0.0.0.0:7606", "0.0.0.0:8606");
    loop{}
}
