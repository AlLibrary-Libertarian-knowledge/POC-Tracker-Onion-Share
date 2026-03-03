use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicFile {
    pub name: String,
    pub size: u64,
    pub link: String,
}

#[derive(Clone)]
struct Node {
    last_ping: Instant,
    files: Vec<PublicFile>,
}

type SharedState = Arc<Mutex<HashMap<String, Node>>>;

#[derive(Deserialize)]
struct PingReq {
    node_id: String,
    files: Vec<PublicFile>,
}

#[derive(Serialize)]
struct LobbyRes {
    online_nodes: usize,
    files: Vec<PublicFile>,
}

/// Registra que um node (cliente) está online e quais arquivos ele oferece publicamente.
async fn ping(State(state): State<SharedState>, Json(req): Json<PingReq>) -> Json<()> {
    let mut map = state.lock().unwrap();
    map.insert(
        req.node_id,
        Node {
            last_ping: Instant::now(),
            files: req.files,
        },
    );
    tracing::info!("Node atualizado. Total online: {}", map.len());
    Json(())
}

/// Retorna a lista de todos os usuários ativos nos últimos 2 minutos e seus arquivos.
async fn lobby(State(state): State<SharedState>) -> Json<LobbyRes> {
    let mut map = state.lock().unwrap();
    // Remove (evict) nodes que não enviaram ping nos últimos 2 minutos
    map.retain(|_, n| n.last_ping.elapsed() < Duration::from_secs(120));

    let online_nodes = map.len();
    let mut files = Vec::new();

    // Agrega arquivos de todos os nós vivos anonimamente
    for n in map.values() {
        files.extend(n.files.clone());
    }

    Json(LobbyRes {
        online_nodes,
        files,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let state: SharedState = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/ping", post(ping))
        .route("/lobby", get(lobby))
        .with_state(state);

    // O tracker roda localmente na 8080.
    // O tracker escuta em 0.0.0.0 na 8080 para aceitar conexões de outros containers Docker (Ex: o proxy Tor)
    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("✅ Tracker Server iniciado em http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
