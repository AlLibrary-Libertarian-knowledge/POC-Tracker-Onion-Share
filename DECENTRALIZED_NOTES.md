# Trackerless LAN Swarm Notes

This adaptation removes the runtime dependency on the HTTP/WebSocket tracker for peer discovery.

## What changed

- Peer discovery is now done with UDP multicast on the local network.
- Every node periodically announces:
  - `node_id`
  - `onion`
  - public shared files
  - `content_hash`
- The GUI lobby is rebuilt locally from the peer announcements.
- Swarm downloads now resolve peers from the local discovery cache instead of `/swarm/:content_hash` on a tracker.
- File integrity is checked with:
  - full-file BLAKE3 hash
  - per-chunk BLAKE3 hash list in the manifest

## Scope

This is a **trackerless LAN discovery** approach.
It removes the central tracker requirement for machines that can see the same multicast domain.

It does **not** implement a full Internet-wide DHT/bootstrap network.
For WAN-scale decentralization, the next step would be a libp2p/DHT layer.

## Config

New config fields:

- `discovery_multicast_addr`
- `discovery_port`

Defaults:

- `239.255.77.77`
- `41075`
