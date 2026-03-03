# 🧅 onion-poc

> **TCC PoC** — Compartilhamento seguro de arquivos via Tor Onion Service, implementado 100% em Rust.

[![CI/Release](https://github.com/DJmesh/onion_poc/actions/workflows/build.yml/badge.svg)](https://github.com/DJmesh/onion_poc/actions)
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

Baixe as versões mais recentes compiladas para Windows, Linux e macOS diretamente na **página de Releases**:
👉 **[Baixar onion-poc (GitHub Releases)](https://github.com/DJmesh/onion_poc/releases/latest)**

---

## 💻 Instalação

### Linux (.deb — Ubuntu, Debian, Mint)

```bash
sudo dpkg -i onion-poc_linux_amd64.deb
# Encontre o app no menu de aplicativos: "onion-poc"
# Ou execute diretamente: onion_poc
```

### Windows

```
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

## 🏗️ Como a Mágica Acontece? (A Arquitetura P2P + Tracker)

O **onion-poc** funciona nos bastidores de forma elegante, dividindo a responsabilidade em duas partes principais: **A Bússola da Rede (O Servidor)** e **O Aplicativo Ponto-a-Ponto (Os Clientes)**. Inspirada na filosofia BitTorrent e Web3, a arquitetura garante segurança e anonimato máximos através da rede Tor.

### 🧭 1. O Lado do Servidor: A Bússola da Rede (Docker)

No servidor, operam dois contêineres trabalhando em dupla:

*   **O Tracker (O Código em Rust):** Um servidor Web contruído em Axum extremamente leve. Ele não salva absolutamente nenhum arquivo. Seu único papel é ouvir a porta 8080 e gerenciar um "Lobby" em memória RAM: *"O usuário XYZ está online e possui o arquivo 'foto.jpg'"*. Se algum cliente ficar silencioso por 2 minutos, ele o remove da lista.
*   **A "Capa" do Tor (`tor_service`):** Como o Tracker por si só não se comunica com a rede Tor, este segundo contêiner atua como um **"Porteiro Cego"**. Ele roda o serviço oficial do Tor, cria um endereço secreto `.onion` e intercepta qualquer acesso vindo da DarkWeb. Ele decripta o pacote e repassa para a porta 8080 do Tracker. Graças a ele, o Tracker enxerga apenas requisições locais comuns e não faz ideia do verdadeiro IP que originou a mensagem!

Essa dupla garante um "ponto de encontro" eficiente sem comprometer a identidade física de ninguém.

### ✨ 2. O Lado do Cliente: Onde a Mágica Acontece (App GUI)

Quando você executa o `onion_poc.exe` (ou as versões em .deb e macOS) no seu computador, o espetáculo começa:

#### 🚇 Inicializando o "Motor" Local

*   O seu aplicativo inicia de forma invisível um nó do Tor local em segundo plano.
*   Ele rapidamente "cava um túnel" criptografado diretamente para a rede mundial do Tor.
*   O seu computador ganha seu próprio URL `.onion`. Se você compartilha um arquivo, sua máquina se torna um mini-servidor invisível preparado para transferir **Chunks (pedaços)**.

#### 💓 O Batimento Cardíaco (Aba global de Busca)

Existe um loop silencioso (no `src/gui/bg.rs`) rodando a cada 5 segundos:

*   **Ping (Avisando que estou vivo):** O aplicativo coleta a sua lista de arquivos públicos, converte num JSON e trafega de fininho pelo túnel (Socks5) até o Tracker `.onion` remoto: *"Eaí Tracker, sou o Usuário 1234, ainda tô aqui e tenho os arquivos A e B."*
*   **Fetch (Lendo o Radar):** Imediatamente, ele solicita: *"Me manda a lista de quem mais tá online"*. O Tracker responde com a lista global em posse de todas as máquinas conectadas.
*   **Injeção ao Vivo:** A tela Egui é atualizada na "Limbo/Search" piscando em tempo real.

#### 📥 Download e Transferência Exclusiva Ponto-a-Ponto

Aqui está o grande diferencial: **o Tracker jamais trafega arquivos!** Ele apenas aponta o mapa. Quando você clica em "Baixar", O Tracker "sai da jogada".

1. O seu cliente Tor se conecta diretamente ao cliente Tor local de quem tem o arquivo (P2P real via `.onion`), solicitando de forma assíncrona o arquivo despedaçado em "Chunks".
2. O tráfego é blindado primeiramente pela rede Tor e, como camada extrema de segurança, **cada pedaço (Chunk)** é criptografado nativamente com seu algoritmo **XChaCha20-Poly1305** militar. Ninguém (nem o Provedor de Internet nem os Nós da rede Tor) sabe que arquivo está sendo passado!
3. No seu computador, os pedaços chegam, são submetidos a uma dupla decriptagem e **remontados perfeitamente em memória.**

### 🎭 Resumo da Ópera

O Tracker funciona como o painel de **Classificados** de um jornal anônimo – ele anuncia quem tem o quê. Mas o verdadeiro leilão e as entregas ocorrem de forma rigorosa, secreta e **criptografada Ponto-a-Ponto de Máquina A para Máquina B**, furando firewalls naturalmente sem necessidade de liberar portas de modem ou ter IP fixo. 

Toda essa orquestra militar dentro de **um binário veloz de interface limpa!** 🚀

---

## 🏗️ Estrutura de Diretórios

```
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

```
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

| Versão | Data | Destaque |
|---|---|---|
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
