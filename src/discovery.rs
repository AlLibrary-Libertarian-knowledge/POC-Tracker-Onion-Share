use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use crate::config::AppConfig;
use crate::gui::shared::SharedStateRef;
use crate::tracker_proto::{AnnouncedFile, NetworkFile, NetworkLobby, PeerLocation};

const DISCOVERY_VERSION: u8 = 1;
const PEER_TTL_SECS: u64 = 120; // Aumentado para o Gossip (WAN é mais lento)
const ANNOUNCE_INTERVAL_SECS: u64 = 4;
const GOSSIP_INTERVAL_SECS: u64 = 45;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryAnnouncement {
    pub version: u8,
    pub node_id: String,
    pub onion: String,
    pub files: Vec<AnnouncedFile>,
    pub sent_at_unix: u64,
}

#[derive(Debug, Clone)]
pub struct SeenPeer {
    last_seen: Instant,
    onion: String,
    files: Vec<AnnouncedFile>,
}

type PeersMap = Arc<Mutex<HashMap<String, SeenPeer>>>;

pub async fn discovery_loop(shared: SharedStateRef) {
    let peers: PeersMap = Arc::new(Mutex::new(HashMap::new()));

    let s1 = shared.clone();
    let p1 = peers.clone();
    tokio::spawn(async move {
        if let Err(e) = gossip_loop(s1, p1).await {
            tracing::error!("gossip_loop error: {e}");
        }
    });

    loop {
        if let Err(err) = run_discovery_loop(shared.clone(), peers.clone()).await {
            tracing::warn!("discovery loop error: {err}");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

async fn run_discovery_loop(shared: SharedStateRef, peers: PeersMap) -> anyhow::Result<()> {
    let cfg = AppConfig::load();
    let group_ip: Ipv4Addr = cfg.discovery_multicast_addr.parse()?;
    let port = cfg.discovery_port;
    let target = SocketAddr::V4(SocketAddrV4::new(group_ip, port));

    let recv_std = std::net::UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)))?;
    recv_std.set_nonblocking(true)?;
    recv_std.join_multicast_v4(&group_ip, &Ipv4Addr::UNSPECIFIED)?;
    recv_std.set_multicast_loop_v4(true)?;
    let recv_socket = UdpSocket::from_std(recv_std)?;

    let send_std = std::net::UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)))?;
    send_std.set_nonblocking(true)?;
    send_std.set_multicast_loop_v4(true)?;
    send_std.set_multicast_ttl_v4(1)?;
    let send_socket = UdpSocket::from_std(send_std)?;

    let mut announce_tick = tokio::time::interval(Duration::from_secs(ANNOUNCE_INTERVAL_SECS));
    let mut cleanup_tick = tokio::time::interval(Duration::from_secs(5));
    let mut buf = vec![0u8; 65_536];

    loop {
        tokio::select! {
            _ = announce_tick.tick() => {
                if let Some(local_announce) = build_local_announce(&shared) {
                    let payload = serde_json::to_vec(&local_announce)?;
                    let _ = send_socket.send_to(&payload, target).await;
                    let mut p = peers.lock().await;
                    update_shared_lobby(&shared, "", Some(local_announce), &mut p);
                } else {
                    let mut p = peers.lock().await;
                    update_shared_lobby(&shared, "", None, &mut p);
                }
            }
            _ = cleanup_tick.tick() => {
                let local = build_local_announce(&shared);
                let fallback_node_id = local.as_ref().map(|a| a.node_id.clone()).unwrap_or_default();
                let mut p = peers.lock().await;
                update_shared_lobby(&shared, &fallback_node_id, local, &mut p);
            }
            recv = recv_socket.recv_from(&mut buf) => {
                let (len, _from) = recv?;
                if let Ok(msg) = serde_json::from_slice::<DiscoveryAnnouncement>(&buf[..len]) {
                    let local_node_id = AppConfig::load().node_id;
                    if msg.version != DISCOVERY_VERSION || msg.node_id == local_node_id {
                        continue;
                    }
                    let mut p = peers.lock().await;
                    p.insert(msg.node_id.clone(), SeenPeer {
                        last_seen: Instant::now(),
                        onion: msg.onion,
                        files: msg.files,
                    });
                    update_shared_lobby(&shared, &local_node_id, build_local_announce(&shared), &mut p);
                }
            }
        }
    }
}

pub async fn gossip_loop(shared: SharedStateRef, peers: PeersMap) -> anyhow::Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(GOSSIP_INTERVAL_SECS));
    tokio::time::sleep(Duration::from_secs(15)).await;

    loop {
        interval.tick().await;

        let (active, socks_addr, bootstrap) = {
            let s = shared.lock().unwrap();
            let cfg = AppConfig::load();
            (s.tor_active, s.tor_socks_addr.clone(), cfg.bootstrap_peers.clone())
        };

        if !active || socks_addr.is_none() { continue; }

        let proxy = match reqwest::Proxy::all(format!("socks5h://{}", socks_addr.unwrap())) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(30))
            .build() {
                Ok(c) => c,
                Err(_) => continue,
            };

        for target in bootstrap {
            let url = format!("{}/network/gossip", target.trim_end_matches('/'));
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(msg) = resp.json::<crate::tracker_proto::GossipMessage>().await {
                        let node_id = msg.node_id.clone();
                        let local_node_id = AppConfig::load().node_id;
                        if node_id == local_node_id { continue; }

                        let mut p = peers.lock().await;
                        p.insert(node_id, SeenPeer {
                            last_seen: Instant::now(),
                            onion: msg.onion,
                            files: msg.files,
                        });
                        update_shared_lobby(&shared, &local_node_id, build_local_announce(&shared), &mut p);
                    }
                }
                Err(_) => {}
            }
        }
    }
}

fn build_local_announce(shared: &SharedStateRef) -> Option<DiscoveryAnnouncement> {
    let s = shared.lock().unwrap();
    let cfg = AppConfig::load();
    if !s.tor_active {
        return None;
    }
    let onion = s.onion_addr.clone()?;
    let files = if cfg.share_publicly {
        s.shared_files
            .iter()
            .map(|f| AnnouncedFile {
                file_id: f.file_id,
                name: f.name.clone(),
                size: f.size,
                link: f.link.clone(),
                content_hash: f.content_hash.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Some(DiscoveryAnnouncement {
        version: DISCOVERY_VERSION,
        node_id: cfg.node_id,
        onion,
        files,
        sent_at_unix: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
    })
}

fn update_shared_lobby(
    shared: &SharedStateRef,
    fallback_local_node_id: &str,
    local_announce: Option<DiscoveryAnnouncement>,
    peers: &mut HashMap<String, SeenPeer>,
) {
    prune_peers(peers);
    let local_node_id = if let Some(local) = &local_announce {
        local.node_id.clone()
    } else {
        fallback_local_node_id.to_string()
    };
    let lobby = aggregate_lobby(&local_node_id, local_announce.as_ref(), peers);
    shared.lock().unwrap().global_lobby = lobby;
}

fn prune_peers(peers: &mut HashMap<String, SeenPeer>) {
    peers.retain(|_, peer| peer.last_seen.elapsed() < Duration::from_secs(PEER_TTL_SECS));
}

fn aggregate_lobby(
    local_node_id: &str,
    local: Option<&DiscoveryAnnouncement>,
    peers: &HashMap<String, SeenPeer>,
) -> NetworkLobby {
    let mut by_hash: HashMap<String, NetworkFile> = HashMap::new();
    let mut online_nodes = peers.len();

    if let Some(local) = local {
        online_nodes += 1;
        merge_files(&mut by_hash, &local.node_id, &local.onion, &local.files);
    } else if !local_node_id.is_empty() {
        online_nodes += 1;
    }

    for (node_id, peer) in peers {
        merge_files(&mut by_hash, node_id, &peer.onion, &peer.files);
    }

    NetworkLobby {
        online_nodes,
        files: by_hash.into_values().collect(),
    }
}

fn merge_files(
    by_hash: &mut HashMap<String, NetworkFile>,
    node_id: &str,
    onion: &str,
    files: &[AnnouncedFile],
) {
    for file in files {
        let entry = by_hash.entry(file.content_hash.clone()).or_insert_with(|| NetworkFile {
            name: file.name.clone(),
            size: file.size,
            link: file.link.clone(),
            content_hash: file.content_hash.clone(),
            peer_count: 0,
            peers: Vec::new(),
        });
        if entry.peers.iter().any(|p| p.node_id == node_id && p.file_id == file.file_id) {
            continue;
        }
        entry.peers.push(PeerLocation {
            node_id: node_id.to_string(),
            onion: onion.to_string(),
            file_id: file.file_id,
            link: file.link.clone(),
        });
        entry.peer_count = entry.peers.len();
    }
}
