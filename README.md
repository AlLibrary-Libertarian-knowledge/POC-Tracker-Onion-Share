# 🧅 onion-poc

> **Build v0.8.0 descentralizada P2P (Gossip over Tor):** esta versão implementa uma rede puramente ponto-a-ponto. A descoberta de arquivos acontece via UDP multicast na rede local e via protocolo Gossip sobre Tor na rede global, sem depender de um servidor central fixo.

> **TCC PoC** — Compartilhamento seguro de arquivos via Tor Onion Service, implementado 100% em Rust.

[![CI/Release](https://github.com/DJmesh/onion_poc/actions/workflows/build.yml/badge.svg)](https://github.com/DJmesh/onion_poc/actions)
[![Latest Release](https://img.shields.io/github/v/release/DJmesh/onion_poc?label=unstable&color=blue)](https://github.com/DJmesh/onion_poc/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## ✨ Funcionalidades

| Feature | Descrição |
|---|---|
| 🔒 **Criptografia** | XChaCha20-Poly1305 + BLAKE3 por chunk |
| 🧅 **Tor Onion v3** | Endereço .onion gerado automaticamente |
| 🖥️ **GUI Nativa** | egui/eframe — sem Electron, sem terminal |
| 🖱️ **Mouse + Drag & Drop** | Arraste arquivos diretamente para a janela |
| 📂 **Diálogo nativo** | Explorador de arquivos do sistema |
| 👥 **Online em tempo real** | Conta usuários conectados via watch channel |
| 🔧 **Auto-install Tor** | Linux/macOS/Windows — wizard instala automaticamente |
| 🧪 **20 testes** | Suite completa de qualidade obrigatória no CI |

---

## 🚀 Download Automático (Builds Oficiais)

Baixe a versão **v0.8.0 (Rede Descentralizada)** compilada para Windows, Linux e macOS:
👉 **[Baixar onion-poc v0.8.0 (GitHub Releases)](https://github.com/DJmesh/onion_poc/releases/latest)**

---

## 💻 Instalação

### Linux (.deb — Ubuntu, Debian, Mint)

```bash
sudo dpkg -i onion-poc_linux_amd64.deb
# Encontre o app no menu de aplicativos: "onion-poc"
# Ou execute diretamente: onion_poc
```

### Windows

```text
Baixe e execute o instalador onion_poc_setup_windows.exe.
O wizard (Inno Setup) cuidará de instalar o app em seu computador,
extrair o Tor pré-embutido e criar o atalho na área de trabalho!
```

### macOS

```bash
chmod +x onion_poc-macos-universal
./onion_poc-macos-universal
```

---

## 🎯 Primeira Execução

1. **Aceite os Termos** — role até o final para ativar o botão
2. **Tor instala automaticamente** — o wizard detecta e instala se necessário (30–90s)
3. **Dashboard aparece** — Tor ativo, endereço .onion gerado
4. **Compartilhe** — arraste arquivos ou use "＋ Adicionar arquivo"
5. **Copie o link** — clique em 📋 ao lado de cada arquivo

> A configuração é salva em `~/.config/br.tcc/onion_poc/config.json`.  
> Reinstalar o app reseta os termos — o wizard exibe novamente na primeira abertura.

---

## 🏗️ Como a Mágica Acontece? (A Arquitetura P2P Descentralizada)

O **onion-poc v0.8.0** marca a transição para uma rede totalmente descentralizada. Não existe mais um "servidor central" (Tracker) obrigatório. Cada instância do aplicativo atua simultaneamente como cliente e servidor (Node), colaborando para manter o mapa da rede (Lobby) ativo.

### 🧭 1. Descoberta Híbrida (LAN + WAN)

Para encontrar quem tem o arquivo que você busca, o aplicativo usa dois caminhos:

* **Rede Local (LAN):** Usa **UDP Multicast**. O app envia um sinal "estou aqui" para um grupo IP específico na rede local. Todos os outros usuários na mesma rede (casa ou escritório) ouvem e trocam listas de arquivos instantaneamente, sem precisar passar pela internet.
* **Rede Global (WAN):** Usa o **Protocolo Gossip sobre Tor**. O aplicativo se conecta a "nós conhecidos" (Bootstrap Peers) via endereços `.onion`. Ele pergunta: *"Quem você conhece e o que eles têm?"*. A resposta é integrada ao seu lobby local e você passa a conhecer novos peers, criando uma teia de descobertas anônima e sem ponto único de falha.

### ✨ 2. O Lado do Cliente: Onde a Mágica Acontece (App GUI)

Quando você executa o `onion_poc.exe` no seu computador:

#### 🚇 Túneis Invisíveis

* O aplicativo inicia um nó do Tor local e "cava um túnel" criptografado.
* Seu computador ganha seu próprio URL `.onion`. Se você compartilha um arquivo, sua máquina se torna um mini-servidor invisível preparado para transferir **Chunks (pedaços)**.

#### 💓 O "Gossip" (Sincronização P2P)

* Existe um loop de background (`src/discovery.rs`) que roda em paralelo.
* **LAN:** A cada 4 segundos, anuncia seus arquivos via Multicast.
* **WAN:** A cada 45 segundos, sorteia peers conhecidos e sincroniza o estado da rede via Tor SOCKS5 Proxy.

* **Ping (Avisando que estou vivo):** O aplicativo coleta a sua lista de arquivos públicos, converte num JSON e trafega de fininho pelo túnel (Socks5) até o Tracker `.onion` remoto: *"Eaí Tracker, sou o Usuário 1234, ainda tô aqui e tenho os arquivos A e B."*
* **Fetch (Lendo o Radar):** Imediatamente, ele solicita: *"Me manda a lista de quem mais tá online"*. O Tracker responde com a lista global em posse de todas as máquinas conectadas.
* **Injeção ao Vivo:** A tela Egui é atualizada na "Limbo/Search" piscando em tempo real.

#### 🔄 Sincronização e Download (WebSockets + Swarm)

A partir da versão **0.7.5**, o processo de download foi otimizado para ser 100% resiliente:

1. **Conexão Perene:** Ao abrir o app, ele estabelece um túnel **WebSocket via SOCKS5/Tor** com o Tracker. Isso significa que o servidor sabe instantaneamente quem entra e quem sai, mantendo o Lobby limpo.
2. **Busca Baseada em Conteúdo:** Quando você busca um arquivo, o app recebe uma lista de **todos os endereços Onion** que possuem aquele mesmo Hash.
3. **Download em Enxame (Swarm):** Ao clicar em baixar, o app inicia um "enxame":
    * Ele divide o arquivo em pedaços de 256 KB.
    * Ele solicita o Pedaço 1 do PC A, o Pedaço 2 do PC B, o Pedaço 3 do PC C... simultaneamente.
    * Se o PC B cair, o app detecta e pede o Pedaço 2 para o PC A ou C automaticamente.
4. **Dashboard de Alta Precisão:** A nova interface calcula a média de velocidade dos últimos pedaços e projeta o **ETA (Tempo Estimado)**, proporcionando uma experiência digna de gerenciadores de download profissionais.

Toda essa orquestra militar dentro de **um binário veloz de interface limpa!** 🚀

---

## 🔄 Evolução: O Que Mudou na v0.7.0?

A versão atual (**0.7.0**) representa uma evolução fundamental em relação à arquitetura anterior. Abaixo, detalhamos o salto de um discovery simples para um sistema de enxame (**Swarm**) moderno:

| Característica | Arquitetura Antiga (v0.6.x) | Nova Arquitetura Swarm (v0.7.0) | Benefício |
| --- | --- | --- | --- |
| **Protocolo Tracker** | HTTP Long-polling (lento) | **WebSocket Bi-direcional** | Lobby atualizado em tempo real. |
| **Identificação** | Nome do arquivo (Vulnerável a colisão) | **BLAKE3 Content Hash** | Identifica conteúdo único globalmente. |
| **Transferência** | 1 Cliente → 1 Servidor (Onion único) | **Multi-peer Swarm** | Baixa pedaços de vários peers ao mesmo tempo. |
| **Deduplicação** | Arquivos iguais apareciam duplicados | **Agrupamento por Hash** | Mesma mídia em 10 máquinas = 1 entrada com 10 fontes. |
| **Criptografia** | Chave aleatória por compartilhamento | **Chave Determinística (Hash)** | Permite baixar chunks de qualquer peer do enxame. |

### 🛠️ Por que usar Hash BLAKE3?

Diferente da versão anterior que dependia de links únicos (como o OnionShare original), a v0.7.0 implementa **descoberta baseada em conteúdo**. Se você tiver o `installer.iso` e outras 5 pessoas também tiverem (mesmo com nomes de arquivo diferentes), o sistema reconhece o hash e permite que você "puxe" os chunks de todos esses peers em paralelo, aumentando a disponibilidade e velocidade (similar ao BitTorrent).

---

## 🏗️ Estrutura de Diretórios

```text
src/
├── main.rs          — entrada: GUI (padrão) ou CLI (share/join)
├── lib.rs           — expõe módulos para testes externos
├── config.rs        — AppConfig persistente (JSON)
├── crypto.rs        — XChaCha20-Poly1305 + BLAKE3
├── link.rs          — parsing/geração de links onion://
├── share.rs         — chunking de arquivos
├── tor.rs           — controle do processo Tor
├── server/          — Axum HTTP server (multi-arquivo, sessões)
├── wizard/          — installer do Tor por plataforma + TERMS_TEXT
└── gui/
    ├── mod.rs       — runner: inicia background thread + eframe
    ├── shared.rs    — SharedState (Arc<Mutex>) + GuiControl + TorInitState
    ├── bg.rs        — background manager (tokio runtime próprio)
    └── app.rs       — GUI egui: sidebar, views, modais, drag & drop

tests/
└── unit.rs          — 20 testes: crypto, config, shared_state, link
```

### Fluxo de dados

```text
        GUI Thread (eframe)                 Background Thread (tokio)
        ──────────────────                  ─────────────────────────
update() → lock(SharedState) →  read ──→  poll control_queue()
           → write control_queue()         → StartTor / StopTor / AddFile
           → unlock                        → update tor_active, onion_addr, stats
                                           → unlock
```

---

## 🧪 Qualidade & Testes

Os testes são um **gate obrigatório** no CI — a build só avança se todos passarem.

```bash
# Rodar todos os testes localmente
cargo test

# Saída esperada:
# test crypto_tests::encrypt_decrypt_roundtrip ... ok
# test crypto_tests::wrong_key_fails_decryption ... ok
# ...
# test result: ok. 20 passed; 0 failed
```

### Cobertura da suite

| Módulo | Cenários testados |
|---|---|
| **crypto** | encrypt/decrypt roundtrip, chave errada, base64, vazio, 256 KB, nonces únicos |
| **config** | defaults, serialize/deserialize, caminhos customizados |
| **gui/shared** | estado inicial, fila de controle, fmt_bytes, uptime |
| **link** | parse válido, link inválido retorna Err |

---

## 🛠️ Build local

```bash
# Requer: Rust stable + sistema: libxcb-render0-dev libgtk-3-dev pkg-config

# Dependências no Ubuntu/Debian:
sudo apt-get install -y \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libwayland-dev libgtk-3-dev libssl-dev pkg-config

# Compilar + executar o cliente grafico:
cargo run --bin onion_poc

# Compilar + executar o servidor Rastreador (Tracker):
# O Tracker é responsavel por manter o lobby de arquivos e a contagem de usuarios.
cargo run --bin tracker

# Testes:
cargo test

# Pacote .deb (cliente grafico):
cargo install cargo-deb
cargo deb
```

---

## 📋 Changelog

Veja [CHANGELOG.md](CHANGELOG.md) para histórico completo de versões.

| **0.7.5** | 2026-03-17 | **UX & Real-time Dashboard** — Velocidade (Mbps/KB/s), ETA e Redesign do Lobby |
| **0.7.4** | 2026-03-17 | **WebSocket @ Tor (SOCKS5)** — Suporte a Trackers Onion e Lobby Global Anônimo |
| **0.7.3** | 2026-03-17 | **Fix Build & Production Tracker** — Correção de imports, debug nodes e URL Onion oficial |
| **0.7.0** | 2026-03-17 | **Tracker WebSocket & Swarm Download** — lobby bi-direcional e download multi-peer por hash |
| **0.6.1** | 2026-03-03 | Busca em tempo real e Correção da Bridge Docker no Tracker |
| **0.6.0** | 2026-03-03 | Rede Decentralizada: Servidor Rastreador (Tracker) e aba de Lobby/Busca global implementada |
| **0.5.0** | 2026-03-03 | Painel de Download nativo operando com socks_addr Tor em background (velocidade real) |
| **0.4.0** | 2026-03-03 | Instalador Inno Setup automatizado pro Windows embarcando Tor |
| **0.3.1** | 2026-03-03 | Cores legíveis, file dialog não-bloqueante, 20 testes |
| **0.3.0** | 2026-03-03 | GUI nativa egui/eframe — mouse, drag & drop, modais |
| **0.2.1** | 2026-03-03 | .desktop + ícone SVG + AppStream no .deb |
| **0.2.0** | 2026-03-03 | Wizard TUI, auto-install Tor, CI builds multiplataforma |
| **0.1.0** | 2026-03-03 | MVP: compartilhamento via Tor, criptografia por chunk |

---

## 📄 Licença

MIT — Eduardo Prestes, 2024.  
Repositório: [github.com/DJmesh/onion_poc](https://github.com/DJmesh/onion_poc)

---

> **Nota para Teste (POC):** O endereço padrão do servidor de rastreio (tracker) agora é: `http://3phps2siiwstimug2mipw7tlizdvdmfydjf5clb7phujg4yfnkrh56qd.onion`. (Certifique-se de que o tracker esteja ativo se for testar localmente em outra porta).
