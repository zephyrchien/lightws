use std::time::Duration;

use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[tokio::test]
async fn async_read_write() {
    env_logger::init();

    let lis = TcpListener::bind(ADDR).await.unwrap();

    let t1 = tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis.accept().await.unwrap();
        debug!("server: tcp accepted!");
        let mut ws = Endpoint::<_, Server>::accept_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("server: websocket accepted!");
        let n = ws.read(&mut buf).await.unwrap();
        debug!("server: receive message: {}", gets!(&buf[..n]));
        assert_eq!(n, PING_DATA.len());
        assert_eq!(&buf[..n], PING_DATA);
        debug!("server: send..");
        let n = ws.write(PONG_DATA).await.unwrap();
        assert_eq!(n, PONG_DATA.len());
    });

    let t2 = tokio::spawn(async {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let tcp = TcpStream::connect(ADDR).await.unwrap();
        debug!("client: tcp connected!");
        let mut ws = Endpoint::<_, Client>::connect_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("client: websocket connected!");

        debug!("client: send..");
        let n = ws.write(PING_DATA).await.unwrap();
        assert_eq!(n, PING_DATA.len());
        let n = ws.read(&mut buf).await.unwrap();
        debug!("client: receive message: {}", gets!(&buf[..n]));
        assert_eq!(n, PONG_DATA.len());
        assert_eq!(&buf[..n], PONG_DATA);
    });

    let _ = tokio::join!(t1, t2);
}
