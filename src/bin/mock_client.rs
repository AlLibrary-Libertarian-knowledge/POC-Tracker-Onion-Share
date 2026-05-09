use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{SinkExt, StreamExt};
use serde_json;
use uuid::Uuid;
use onion_poc::tracker_proto::{WsClientMessage, AnnouncedFile};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = "ws://127.0.0.1:8080/ws";
    println!("Connecting to {}...", url);
    let (mut ws_stream, _) = connect_async(url).await?;
    println!("Connected!");

    let announce = WsClientMessage::Announce {
        node_id: "test-node-id".to_string(),
        onion: "test-onion.onion".to_string(),
        files: vec![
            AnnouncedFile {
                file_id: Uuid::new_v4(),
                name: "test-file.txt".to_string(),
                size: 1024,
                link: "onion://test-onion.onion/s/test-uuid".to_string(),
                content_hash: "test-hash-123".to_string(),
            }
        ],
    };

    let text = serde_json::to_string(&announce)?;
    ws_stream.send(Message::Text(text.into())).await?;
    println!("Announce sent! Keeping connection alive for 10 seconds...");

    // Keep connection alive to be seen by the tracker
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    Ok(())
}
