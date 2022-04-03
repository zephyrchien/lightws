use std::time::Duration;

use tokio::net::{TcpStream, TcpListener};

use lightws::endpoint::Endpoint;
use lightws::role::{Client, Server};

use log::debug;

const ADDR: &str = "127.0.0.1:10000";
const HOST: &str = "www.example.com";
const PATH: &str = "/ws";

#[tokio::test]
async fn async_handshake() {
    env_logger::init();

    let lis = TcpListener::bind(ADDR).await.unwrap();

    let t1 = tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        let (tcp, _) = lis.accept().await.unwrap();
        debug!("server: tcp accepted!");
        let _ = Endpoint::<_, Server>::accept_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("server: websocket accepted!");
    });

    let t2 = tokio::spawn(async {
        let mut buf = vec![0u8; 1024];
        debug!("client: sleep 500ms..");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let tcp = TcpStream::connect(ADDR).await.unwrap();
        debug!("client: tcp connected!");
        let _ = Endpoint::<_, Client>::connect_async(tcp, &mut buf, HOST, PATH)
            .await
            .unwrap();
        debug!("client: websocket connected!");
    });

    let _ = tokio::join!(t1, t2);
}
