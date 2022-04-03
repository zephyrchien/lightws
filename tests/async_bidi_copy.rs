use std::time::Duration;

use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
#[tokio::test]
async fn async_bidi_copy() {
    env_logger::init();

    let lis1 = TcpListener::bind(ADDR1).await.unwrap();
    let lis2 = TcpListener::bind(ADDR2).await.unwrap();

    let relay = tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis1.accept().await.unwrap();
        debug!("relay: tcp accepted!");
        let ws_local = Endpoint::<_, Server>::accept_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap()
            .guard();
        debug!("relay: websocket accepted!");

        let tcp = TcpStream::connect(ADDR2).await.unwrap();
        debug!("relay: tcp connected!");
        let ws_remote = Endpoint::<_, Client>::connect_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap()
            .guard();
        debug!("relay: websocket connected!");

        // or use tokio::io::bidirectional_copy ~
        let (mut ws_local_read, mut ws_local_write) = tokio::io::split(ws_local);
        let (mut ws_remote_read, mut ws_remote_write) = tokio::io::split(ws_remote);

        let t1 = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut ws_local_read, &mut ws_remote_write).await;
            debug!("relay: client close, shutdown");
            let _ = ws_remote_write.shutdown().await;
        });

        let t2 = tokio::spawn(async move {
            let _ = tokio::io::copy(&mut ws_remote_read, &mut ws_local_write).await;
            debug!("relay: server close");
        });

        let _ = tokio::join!(t1, t2);
    });

    let server = tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis2.accept().await.unwrap();
        debug!("server: tcp accepted!");
        let mut ws = Endpoint::<_, Server>::accept_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("server: websocket accepted!");

        loop {
            let n = ws.read(&mut buf).await.unwrap();
            if n == 0 && ws.is_read_end() {
                debug!("server: close");
                break;
            }
            debug!("server: echo..");
            let _ = ws.write(&buf[..n]).await.unwrap();
        }
    });

    let client = tokio::spawn(async {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let tcp = TcpStream::connect(ADDR1).await.unwrap();
        debug!("client: tcp connected!");
        let mut ws = Endpoint::<_, Client>::connect_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("client: websocket connected!");

        debug!("client: sleep 500ms..");
        tokio::time::sleep(Duration::from_millis(500)).await;

        for i in 1..=5 {
            debug!("client: send[{}]..", i);
            let n = ws.write(ECHO_DATA).await.unwrap();
            assert_eq!(n, ECHO_DATA.len());

            let n = ws.read(&mut buf).await.unwrap();
            debug!("client: receive message: {}", gets!(&buf[..n]));
            assert_eq!(n, ECHO_DATA.len());
            assert_eq!(&buf[..n], ECHO_DATA);
        }

        debug!("client: close");
    });

    let _ = tokio::join!(relay, server, client);
}
