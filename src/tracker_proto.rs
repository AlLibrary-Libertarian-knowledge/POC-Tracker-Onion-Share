use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncedFile {
    pub file_id: Uuid,
    pub name: String,
    pub size: u64,
    pub link: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerLocation {
    pub node_id: String,
    pub onion: String,
    pub file_id: Uuid,
    pub link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFile {
    pub name: String,
    pub size: u64,
    pub link: String,
    pub content_hash: String,
    pub peer_count: usize,
    pub peers: Vec<PeerLocation>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct NetworkLobby {
    pub online_nodes: usize,
    pub files: Vec<NetworkFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsClientMessage {
    Announce {
        node_id: String,
        onion: String,
        files: Vec<AnnouncedFile>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsServerMessage {
    Lobby { lobby: NetworkLobby },
}
