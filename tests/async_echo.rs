use std::time::Duration;

use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use lightws::endpoint::Endpoint;
use lightws::role::{Client, Server};

use log::debug;

const ADDR: &str = "127.0.0.1:10000";
const HOST: &str = "www.example.com";
const PATH: &str = "/ws";
const ECHO_DATA: &[u8] = b"ECHO ECHO ECHO!";

macro_rules! gets {
    ($b: expr) => {
        std::str::from_utf8($b).unwrap()
    };
}

#[tokio::test]
async fn async_echo() {
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

    let _ = tokio::join!(t1, t2);
}
