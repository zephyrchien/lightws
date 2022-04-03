use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::time::Duration;
use std::thread;

use lightws::endpoint::Endpoint;
use lightws::role::{Client, Server};

use log::debug;

const ADDR: &str = "127.0.0.1:10000";
const HOST: &str = "www.example.com";
const PATH: &str = "/ws";
const PING_DATA: &[u8] = b"PING PING PING!";
const PONG_DATA: &[u8] = b"PONG PONG PONG!";

macro_rules! gets {
    ($b: expr) => {
        std::str::from_utf8($b).unwrap()
    };
}

#[test]
fn sync_read_write() {
    env_logger::init();

    let lis = TcpListener::bind(ADDR).unwrap();

    let t1 = thread::spawn(move || {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis.accept().unwrap();
        debug!("server: tcp accepted!");
        let mut ws = Endpoint::<_, Server>::accept_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("server: websocket accepted!");

        let n = ws.read(&mut buf).unwrap();
        debug!("server: receive message: {}", gets!(&buf[..n]));
        assert_eq!(n, PING_DATA.len());
        assert_eq!(&buf[..n], PING_DATA);

        debug!("server: send..");
        let n = ws.write(PONG_DATA).unwrap();
        assert_eq!(n, PONG_DATA.len());
    });

    let t2 = thread::spawn(|| {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        thread::sleep(Duration::from_millis(500));
        let tcp = TcpStream::connect(ADDR).unwrap();
        debug!("client: tcp connected!");
        let mut ws = Endpoint::<_, Client>::connect_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("client: websocket connected!");

        debug!("client: send..");
        let n = ws.write(PING_DATA).unwrap();
        assert_eq!(n, PING_DATA.len());

        let n = ws.read(&mut buf).unwrap();
        debug!("client: receive message: {}", gets!(&buf[..n]));
        assert_eq!(n, PONG_DATA.len());
        assert_eq!(&buf[..n], PONG_DATA);
    });

    t1.join().unwrap();
    t2.join().unwrap();
}
