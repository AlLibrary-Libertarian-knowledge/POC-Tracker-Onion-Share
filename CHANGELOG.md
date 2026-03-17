# Changelog

Todas as mudanças notáveis neste projeto são documentadas aqui.  
Formato: [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) | Versioning: [SemVer](https://semver.org).

---

## [0.7.0] — 2026-03-17

### ✨ Adicionado

- **Tracker com WebSocket** — Migração do pooling HTTP para WebSocket bi-direcional em `/ws`. Presença em tempo real e atualizações instantâneas do lobby global.
- **Protocolo de Swarm (Multipeer)** — Novo formato de link `opocswarm://` que permite baixar arquivos de múltiplos peers simultâneos.
- **Agrupamento por Hash BLAKE3** — O lobby agora agrupa arquivos identicamente baseados no conteúdo (hash BLAKE3 de 256 bits).
- **Deduplicação Automática** — Mesma mídia em máquinas diferentes aponta para o mesmo hash no servidor, otimizando a disponibilidade.
- **Download em Enxame (Swarm)** — Download distribuído iniciado pela busca, baixando chunks de diferentes peers em paralelo.
- **Chaves Determinísticas** — A chave de criptografia XChaCha20 agora é derivada do hash do conteúdo, garantindo que peers com o mesmo arquivo usem a mesma chave.

### 🔧 Alterado

- **Presença Reversa** — O tracker agora considera nós offline automaticamente se a conexão WebSocket cair ou se não houver resposta em 30 segundos.
- **Tracker URL Padrão** — Ajustado para `http://127.0.0.1:8080` para facilitar a Prova de Conceito (POC) local.
- **Busca por Hash** — Busca na rede agora agrega peers por hash e mostra a contagem total de fontes disponíveis.

### 🎨 GUI

- Aba **🔍 Buscar** atualizada para mostrar contagem de peers e botão de baixar via Swarm.
- Refatoração interna do background manager (`bg.rs`) para gerenciar downloads multipeer.

## [0.3.1] — 2026-03-03

### 🐛 Corrigido

- **Cores ilegíveis** — bug crítico: `Color32::from_rgba_premultiplied` com valores RGB altos e alpha baixo renderizava como cor sólida. Migrado para `from_rgba_unmultiplied` + helper `with_alpha()` em todos os pontos. Contraste WCAG AA garantido (≥ 4.5:1).
- **Freeze ao cancelar diálogo de arquivo** — `rfd::FileDialog::pick_files()` bloqueava a UI thread. Movido para thread separada com `std::sync::mpsc::channel`. Cancelar o diálogo agora não trava mais o programa.
- **Termos → Tor auto-start** — após aceitar os termos, o wizard agora inicia o Tor automaticamente sem precisar clicar em "Ativar".

### ✨ Adicionado

- **Suite de testes (20 testes)** em `tests/unit.rs`:
  - Crypto: encrypt/decrypt roundtrip, chave errada, base64, vazio, 256KB, nonces únicos por chunk
  - Config: defaults, serialize/deserialize, caminhos customizados
  - SharedState: estado inicial, fila de controle, fmt_bytes, uptime
  - Link: parse válido, link inválido retorna Err
- **Gate de qualidade no CI** — `cargo test` obrigatório antes de qualquer build de release
- **lib.rs** — módulos públicos para testes de integração externos
- **Paleta nova de cores**:
  - Fundo: `#0B0C15` (navy profundo)
  - Texto primário: `#E1E4F8` (branco-azulado)
  - Accent: `#69B4FC` (azul ciano vibrante)
  - Verde: `#52D77D`, Vermelho: `#FC5A5A`, Amarelo: `#FFD046`, Ciano: `#3CDCC8`
  - Nav selected: fundo `#1C2644` + texto accent (legível)

### 🔧 Alterado

- `src/gui/app.rs` reescrito: função `with_alpha()` centraliza transparência correta
- `nav_btn()`: fundo dark-blue escuro quando selecionado, nunca cor de acento como fill de texto

---

## [0.3.0] — 2026-03-03

### ✨ Adicionado

- **GUI nativa egui/eframe** substituindo o TUI (ratatui/crossterm)
  - Sidebar clicável com navegação por mouse
  - Header: status Tor, online count, uptime em tempo real
  - Dashboard: stat cards, status da rede, atividade recente
  - Aba Arquivos: drag & drop + diálogo nativo (rfd)
  - Aba Busca: filtro em tempo real
  - Modal Termos: scroll obrigatório antes de aceitar
  - Modal Tor: progress bar animada + timer de espera (30–90s)
  - Status bar responsiva
- `src/gui/shared.rs` — SharedState + TorInitState compartilhado por `Arc<Mutex>`
- `src/gui/bg.rs` — background manager com runtime tokio próprio
- `.desktop` — `Terminal=false` para GUI nativa

### 🔧 Alterado

- `Cargo.toml`: substituídas deps `ratatui/crossterm/futures` por `eframe/egui/rfd`
- `src/wizard/`: simplificado; mantém apenas `installer.rs` + `TERMS_TEXT`
- CI: instalação de system deps para egui no Ubuntu (X11, Wayland, GTK3)

---

## [0.2.1] — 2026-03-03

### ✨ Adicionado

- `.desktop` entry com `Terminal=true` (necessário antes da migração para GUI)
- `debian/onion-poc.svg` — ícone SVG (cebola + cadeado)
- `debian/onion-poc.appdata.xml` — metadados AppStream (remove avisos GNOME Software)
- CI: validação do conteúdo do `.deb` + launcher `.bat` para Windows

---

## [0.2.0] — 2026-03-03

### ✨ Adicionado

- **Wizard TUI** com ratatui/crossterm
  - Tela de Termos de Uso na primeira inicialização
  - Tela de verificação/instalação do Tor
  - Dashboard com 4 abas: Dashboard, Arquivos, Buscar, Sobre
- Instalação automática do Tor:
  - Linux: `apt-get` / `dnf` / `pacman` / `zypper`
  - macOS: `brew`
  - Windows: download do Tor Expert Bundle
- `src/config.rs` — configuração persistente (termos aceitos, caminho Tor)
- CI/CD com GitHub Actions: builds Linux (.deb), Windows (.exe), macOS

### 🔧 Alterado

- `src/main.rs`: sem argumentos → TUI; com `share`/`join` → modo CLI

---

## [0.1.0] — 2026-03-03

### ✨ Inicial

- Compartilhamento de arquivos via Tor Onion Service v3
- Criptografia XChaCha20-Poly1305 + BLAKE3 por chunk
- Servidor Axum multi-arquivo com sessões
- Cliente reqwest via SOCKS5 Tor
- Modo CLI: `onion_poc share --file ...` / `onion_poc join --link ...`
