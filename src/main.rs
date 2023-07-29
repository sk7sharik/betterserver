use server::Server;

mod packet;
mod server;
mod state;
mod states;

fn main() 
{
    let server = Server::start("0.0.0.0:7606");

    loop{}
}
