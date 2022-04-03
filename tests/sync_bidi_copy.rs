use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::time::Duration;
use std::thread;

use lightws::endpoint::Endpoint;
use lightws::role::{Client, Server};

use log::debug;

const ADDR1: &str = "127.0.0.1:10000";
const ADDR2: &str = "127.0.0.1:20000";
const HOST: &str = "www.example.com";
const PATH: &str = "/ws";
const ECHO_DATA: &[u8] = b"ECHO ECHO ECHO!";

macro_rules! gets {
    ($b: expr) => {
        std::str::from_utf8($b).unwrap()
    };
}

// addr0(client) <=> addr1(relay) <=> addr2(server)
#[test]
fn sync_bidi_copy() {
    env_logger::init();

    let lis1 = TcpListener::bind(ADDR1).unwrap();
    let lis2 = TcpListener::bind(ADDR2).unwrap();

    let relay = thread::spawn(move || {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis1.accept().unwrap();
        debug!("relay: tcp accepted!");
        let mut ws_local_read =
            Endpoint::<_, Server>::accept_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("relay: websocket accepted!");

        let tcp = TcpStream::connect(ADDR2).unwrap();
        debug!("relay: tcp connected!");
        let mut ws_remote_read =
            Endpoint::<_, Client>::connect_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("relay: websocket connected!");

        let mut ws_local_write = ws_local_read.try_clone().unwrap();
        let mut ws_remote_write = ws_remote_read.try_clone().unwrap();

        let t1 = thread::spawn(move || {
            let _ = std::io::copy(&mut ws_local_read, &mut ws_remote_write);
            debug!("relay: client close, shutdown");
            ws_remote_write
                .as_ref()
                .shutdown(std::net::Shutdown::Both)
                .unwrap();
        });

        let t2 = thread::spawn(move || {
            let _ = std::io::copy(&mut ws_remote_read, &mut ws_local_write);
            debug!("relay: server close");
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });

    let server = thread::spawn(move || {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis2.accept().unwrap();
        debug!("server: tcp accepted!");
        let mut ws = Endpoint::<_, Server>::accept_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("server: websocket accepted!");

        loop {
            let n = ws.read(&mut buf).unwrap();
            if n == 0 && ws.is_read_end() {
                debug!("server: close");
                break;
            }
            debug!("server: echo..");
            let _ = ws.write(&buf[..n]).unwrap();
        }
    });

    let client = thread::spawn(|| {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        thread::sleep(Duration::from_millis(500));
        let tcp = TcpStream::connect(ADDR1).unwrap();
        debug!("client: tcp connected!");
        let mut ws = Endpoint::<_, Client>::connect_sync(tcp, &mut buf, HOST, PATH).unwrap();
        debug!("client: websocket connected!");

        debug!("client: sleep 500ms..");
        thread::sleep(Duration::from_millis(500));

        for i in 1..=5 {
            debug!("client: send[{}]..", i);
            let n = ws.write(ECHO_DATA).unwrap();
            assert_eq!(n, ECHO_DATA.len());

            let n = ws.read(&mut buf).unwrap();
            debug!("client: receive message: {}", gets!(&buf[..n]));
            assert_eq!(n, ECHO_DATA.len());
            assert_eq!(&buf[..n], ECHO_DATA);
        }

        debug!("client: close");
    });

    relay.join().unwrap();
    server.join().unwrap();
    client.join().unwrap();
}
