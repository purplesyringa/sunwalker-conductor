use crate::{invoker, message};
use anyhow::{anyhow, Context, Result};
use futures_util::stream::StreamExt;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite;

pub struct Conductor {}

impl Conductor {
    pub async fn accept_invoker_connection(&self, socket: TcpStream) {
        let result: Result<()> = try {
            let mut stream = tokio_tungstenite::accept_async(socket)
                .await
                .context("Failure during websocket handshake")?;

            let mut invoker_object = None;

            while let Some(message) = stream.next().await {
                let message = message.context("Failed to read message from the invoker")?;
                match message {
                    tungstenite::Message::Close(_) => break,
                    tungstenite::Message::Binary(buf) => {
                        let message = rmp_serde::from_slice(&buf)
                            .context("Failed to parse buffer as msgpack format")?;
                        match invoker_object {
                            None => {
                                if let message::i2c::Message::Handshake(handshake) = message {
                                    invoker_object = Some(invoker::Invoker::new(handshake));
                                } else {
                                    Err(anyhow!("The first message of the invoker was not a handshake, but {message:?}"))?;
                                }
                            }
                            Some(ref invoker_object) => {
                                invoker_object.handle_message(message).await?;
                            }
                        }
                    }
                    tungstenite::Message::Ping(_) => (),
                    _ => {
                        println!("Message of unknown type received from the invoker: {message:?}")
                    }
                };
            }
        };

        if let Err(e) = result {
            println!("Invoker connection errored: {e:?}");
        }
    }
}
