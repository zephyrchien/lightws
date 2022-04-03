use std::net::{TcpStream, TcpListener};
use std::time::Duration;
use std::thread;

use lightws::endpoint::Endpoint;
use lightws::role::{Client, Server};

use log::debug;

const ADDR: &str = "127.0.0.1:10000";
const HOST: &str = "www.example.com";
const PATH: &str = "/ws";

#[test]
fn sync_handshake() {
    env_logger::init();

    let lis = TcpListener::bind(ADDR).unwrap();

    let t1 = thread::spawn(move || {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis.accept().unwrap();
        debug!("server: tcp accepted!");
        let _ = Endpoint::<_, Server>::accept_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("server: websocket accepted!");
    });

    let t2 = thread::spawn(|| {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        thread::sleep(Duration::from_millis(500));
        let tcp = TcpStream::connect(ADDR).unwrap();
        debug!("client: tcp connected!");
        let _ = Endpoint::<_, Client>::connect_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("client: websocket connected!");
    });

    t1.join().unwrap();
    t2.join().unwrap();
}
