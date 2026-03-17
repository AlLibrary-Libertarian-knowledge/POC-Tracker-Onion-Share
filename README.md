# 🧅 onion-poc v0.8.0 — Decentralized P2P Network (Gossip over Tor)

> **TCC PoC** — Compartilhamento seguro de arquivos via Tor Onion Service, implementado 100% em Rust.
> Esta versão elimina a dependência de um servidor central, criando uma rede puramente ponto-a-ponto (P2P) com descoberta baseada em Gossip e Multicast.

[![CI/Release](https://github.com/DJmesh/onion_poc/actions/workflows/build.yml/badge.svg)](https://github.com/DJmesh/onion_poc/actions)
[![Latest Release](https://img.shields.io/github/v/release/DJmesh/onion_poc?label=unstable&color=blue)](https://github.com/DJmesh/onion_poc/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## ✨ Funcionalidades v0.8.0

| Feature | Descrição |
|---|---|
| �️ **Rede P2P** | Descoberta descentralizada sem Tracker central (Serverless) |
| �️ **Gossip over Tor** | Sincronização anônima de lobby via Hidden Services |
| � **Discovery LAN** | Descoberta instantânea zero-conf via UDP Multicast |
| 🔒 **Criptografia** | XChaCha20-Poly1305 + integridade via BLAKE3 per chunk |
| 🧅 **Tor Onion v3** | Cada nó é seu próprio servidor oculto na DarkWeb |
| 🚀 **Parallel Swarm** | Download em "enxame" de múltiplos peers simultâneos |
| 🖥️ **GUI Nativa** | Interface egui ultra-rápida e responsiva |

---

## 🏗️ Arquitetura P2P Descentralizada

O **onion-poc v0.8.0** marca a transição para uma rede de malha (mesh) totalmente autônoma. Diferente de sistemas tradicionais, aqui não existe um "servidor central".

### 🧭 1. Descoberta Híbrida (LAN + WAN)

Para encontrar quem possui o conteúdo que você deseja (identificado via `content_hash`), o aplicativo utiliza uma estratégia dupla:

* **Rede Local (LAN):** Utiliza **UDP Multicast**. O app anuncia sua presença em um grupo IP local. Outros nós na mesma rede capturam o sinal e trocam metadados instantaneamente. Ideal para escritórios, casas ou eventos.
* **Rede Global (WAN - Gossip):** Utiliza o protocolo **Gossip (Fofoca)** sobre a rede Tor. O aplicativo conecta-se a "Nós de Entrada" (**Bootstrap Nodes**) e pergunta quem mais eles conhecem. A rede se propaga conforme os nós trocam entre si suas listas de pares conhecidos e arquivos compartilhados.

### � 2. Protocolo Gossip (Sync P2P)

Diferente do WebSocket fixo da v0.7.x, na v0.8.0 cada cliente expõe um segredo `/network/gossip` em seu próprio serviço oculto:

1. **Sincronização:** A cada 45 segundos, seu nó escolhe um peer aleatório da lista e solicita uma atualização.
2. **Propagação:** Se você descobre um novo arquivo ou nó, essa informação será "fofocada" para os próximos peers com quem você se conectar, tornando a rede resiliente a censoras ou quedas parciais.

### 🌩️ 3. O Download em Enxame (Swarm)

Mesmo sem um tracker para te dizer quem tem o arquivo, o seu "Lobby Local" agrega tudo o que ele ouviu na LAN e no Gossip. Ao baixar:

* O sistema resolve todos os endereços Onion que possuem o Hash solicitado.
* Inicia conexões simultâneas com até 8 peers.
* Baixa pedaços (chunks) de 256KB em paralelo.
* Verifica a integridade de cada pedaço com BLAKE3.

---

## 🚀 Download Automático (Builds Oficiais)

Baixe a versão **v0.8.0 (Rede Descentralizada)** compilada para Windows, Linux e macOS:
👉 **[Baixar onion-poc v0.8.0 (GitHub Releases)](https://github.com/DJmesh/onion_poc/releases/latest)**

---

## � Instalação & Uso

### Requisitos

* **Tor Expert Bundle** (O onion-poc tenta instalar automaticamente se você não tiver).
* Permissão de rede para Multicast (Firewall local).

### Fluxo de Uso

1. **Start Tor:** Clique no botão de ativação. O app gera seu `.onion` único.
2. **Lobby Global:** Deixe o app aberto por alguns minutos. Ele começará a "fofocar" com a rede e o lobby de arquivos será preenchido.
3. **Compartilhe:** Arraste um arquivo para o app. Ele gera um link `onion://` seguro.
4. **Busque:** Use a aba de busca para encontrar arquivos anunciados organicamente pela rede P2P.

---

## 🏗️ Estrutura do Projeto (v0.8.0)

```text
src/
├── discovery.rs     — NÚCLEO: Gerencia Gossip, Multicast e Peers Maps
├── server/          — Handler local: expõe Manifests, Chunks e Endpoint de Gossip
├── crypto.rs        — XChaCha20-Poly1305 + Integridade BLAKE3
├── gui/
│   ├── bg.rs        — Orquestrador de threads (Tor, Downloads, Discovery)
│   └── app.rs       — Interface reativa em tempo real
└── tracker_proto.rs — Definições de mensagens Gossip e lobby
```

---

## 🛠️ Build Local & Desenvolvimento

```bash
# Requisitos: Rust stable + libs gráficas (libgtk-3-dev, etc)
# Compilar e rodar
cargo run --release

# Para rodar testes de integridade P2P
cargo test
```

---

## 📋 Changelog Principal

| Versão | Principais Mudanças |
|---|---|
| **v0.8.0** | **Decentralized Network** (Gossip over Tor, Bootstrap Nodes, Serverless) |
| **0.7.5** | **UX Real-time** (Speed calculation, ETA, Professional GUI colors) |
| **0.7.0** | **Swarm & Hashing** (Parallel downloads, BLAKE3 content-addressing) |
| **0.6.0** | **Global Lobby** (First version with tracking/searching capabilities) |

---

## 📄 Licença

MIT — Eduardo Prestes, 2024.  
TCC: *Compartilhamento P2P Criptografado sobre Tor Hidden Services.*
