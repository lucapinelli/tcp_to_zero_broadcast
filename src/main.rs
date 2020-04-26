#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;

use futures::stream::StreamExt;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::codec::Decoder;

mod codec;
use codec::ChunkCodec;

mod config;
use crate::config::Settings;

mod zero;
use zero::Broadcast;

#[tokio::main]
async fn main() {
    env_logger::init();

    let conf = Settings::new().unwrap();

    info!("{:?}", conf);

    let mut listener = TcpListener::bind(&conf.tcp.endpoint).await.unwrap();
    info!("TCP listener binded at {}", &conf.tcp.endpoint);

    let broadcast = Broadcast::new(&conf.zero.pub_endpoint).unwrap();
    let broadcast = Arc::new(Mutex::new(broadcast));

    let settings = Arc::new(conf);
    let server = {
        async move {
            let mut incoming = listener.incoming();
            while let Some(conn) = incoming.next().await {
                debug!("connection {:?}", conn);
                let broadcast = Arc::clone(&broadcast);
                let settings = Arc::clone(&settings);
                match conn {
                    Err(e) => error!("TCP connection accept failed: {:?}", e),
                    Ok(stream) => {
                        debug!("a TCP client has connected");
                        tokio::spawn(async move {
                            on_connection(stream, broadcast, settings).await;
                        });
                    }
                }
            }
        }
    };

    // start the server and block this async fn until `server` spins down.
    server.await;
}

async fn on_connection(stream: TcpStream, broadcast: Arc<Mutex<Broadcast>>, conf: Arc<Settings>) {
    let decoder = ChunkCodec::new(conf.tcp.message_termination_byte);
    let mut chunks = decoder.framed(stream);
    while let Some(result) = chunks.next().await {
        match result {
            Ok(message) => {
                trace!("received TCP message = {:?}", message);
                broadcast
                    .lock()
                    .await
                    .send(&conf.zero.pub_topic, &message)
                    .unwrap_or_else(|e| {
                        error!("An error occurred sending the message {}: {}", message, e)
                    });
            }
            Err(err) => {
                eprintln!("TCP socket decode error: {:?}", err);
            }
        }
    }
    debug!("a TCP client closed the connection");
}
