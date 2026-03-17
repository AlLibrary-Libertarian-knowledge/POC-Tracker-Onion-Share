# Decentralized P2P Network Notes (v0.8.0)

Esta versão remove a dependência de um servidor central fixo para descoberta de arquivos, implementando uma rede puramente P2P.

## O que mudou

- **Descoberta Híbrida:**
  - **LAN:** Discovery via UDP Multicast (anúncios a cada 4 segundos).
  - **WAN:** Protocolo **Gossip over Tor**. Sincronização periódica (45s) com Bootstrap Nodes e peers descobertos.
- **Protocolo Gossip:**
  - Cada nó expõe `/network/gossip` via Hidden Service.
  - Troca de listas de arquivos e lista de endereços Onion conhecidos.
  - Reconstrução dinâmica do lobby global no cliente.
- **Swarm Downloads:**
  - Agora os peers são resolvidos a partir do cache de descoberta local (que agrega LAN + WAN).
  - Suporte a download paralelo de múltiplos peers descobertos via Gossip.
- **Bootstrap Nodes:**
  - Suporte a "Entry Points" configuráveis para entrada na rede global.

## Configuração (AppConfig)

- `bootstrap_peers`: Lista de endereços Onion confiáveis.
- `discovery_multicast_addr`: Default `239.255.77.77`.
- `discovery_port`: Default `41075`.

## Segurança e Anonimato

- Toda a comunicação WAN é feita exclusivamente através do túnel SOCKS5 do Tor.
- A integridade dos arquivos é garantida por hashes BLAKE3 (chunk e full-file).
- A topologia da rede é oculta; um observador externo não consegue mapear quem está baixando de quem sem quebrar a criptografia do Tor.
