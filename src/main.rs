#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;

use futures::stream::StreamExt;
use tokio::net::TcpListener;
use tokio_util::codec::Decoder;

mod codec;
use codec::ChunkCodec;

#[tokio::main]
async fn main() {
    env_logger::init();

    let addr = "127.0.0.1:1983";
    let mut listener = TcpListener::bind(addr).await.unwrap();
    info!("TCP listener binded at {}", addr);

    let server = {
        async move {
            let mut incoming = listener.incoming();
            while let Some(conn) = incoming.next().await {
                debug!("connection {:?}", conn);
                match conn {
                    Err(e) => error!("TCP connection accept failed: {:?}", e),
                    Ok(sock) => {
                        debug!("a TCP client has connected");
                        tokio::spawn(async move {
                            let mut chunks = ChunkCodec::new(7).framed(sock);
                            while let Some(result) = chunks.next().await {
                                match result {
                                    // TODO: broadcast the message using ZeroMQ
                                    Ok(message) => trace!("message = {:?}", message),
                                    Err(err) => eprintln!("TCP socket decode error: {:?}", err),
                                }
                            }
                            debug!("a TCP client closed the connection");
                        });
                    }
                }
            }
        }
    };

    println!("Server running on {}", addr);

    // Start the server and block this async fn until `server` spins down.
    server.await;
}
