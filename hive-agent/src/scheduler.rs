use libp2p::{PeerId, Multiaddr};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub id: PeerId,
    pub address: Vec<Multiaddr>,
    pub status: String, // "active", "busy"
}

pub struct Scheduler {
    pub peers: HashMap<PeerId, PeerInfo>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            peers: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        let entry = self.peers.entry(peer_id).or_insert(PeerInfo {
            id: peer_id,
            address: Vec::new(),
            status: "active".to_string(),
        });
        if !entry.address.contains(&addr) {
            entry.address.push(addr);
        }
    }

    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.peers.remove(peer_id);
    }

    pub fn get_available_peer(&self) -> Option<PeerId> {
        self.peers.keys().next().cloned()
    }
}
