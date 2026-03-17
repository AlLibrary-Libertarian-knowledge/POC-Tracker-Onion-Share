# Changelog

Todas as mudanças notáveis neste projeto são documentadas aqui.  
Formato: [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) | Versioning: [SemVer](https://semver.org).

---

## [0.8.0] — 2026-03-17

### ✨ Descentralização P2P (DHT-like Gossip)

- **Remoção de Tracker Central** — O sistema não depende mais de um servidor central para descoberta de arquivos. A rede agora é puramente P2P.
- **Protocolo Gossip sobre Tor** — Implementado um mecanismo de sincronização entre nós (Gossip) que troca informações de arquivos e peers diretamente via Hidden Services.
- **Auto-Discovery LAN & WAN** — Descoberta automática em rede local (Multicast UDP) e rede global (Bootstrap Nodes via SOCKS5/Tor).
- **Swarm Hashing Dinâmico** — Download paralelo (Swarm) reconstruído para buscar peers de forma dinâmica no lobby global sincronizado.

### 🔌 Conectividade

- **Bootstrap Nodes** — Suporte a endereços `.onion` configuráveis como pontos de entrada na rede global.
- **Endpoint de Gossip** — Cada nó agora expõe um endpoint `/network/gossip` para que outros nós possam consultar seu estado de forma segura e anônima.

## [0.7.5] — 2026-03-17

### 🎨 GUI & UX

- **Dashboard de Download Real-time** — Adicionado cálculo de velocidade em tempo real e Estimativa de Tempo (ETA) para downloads.
- **Botão "Baixar Agora" Profissional** — Redesign completo do botão de ação no Lobby: cor verde vibrante, texto branco em negrito e feedback visual aprimorado.
- **Micro-interações** — Adicionado `on_hover` nos botões de download e melhoria no contraste dos progressos.

### 🔧 Corrigido

- **Gagueira no Progresso** — Otimização no envio de atualizações de progresso para a GUI, tornando o avanço da barra de download mais fluido.

## [0.7.4] — 2026-03-17

### Added

- **WebSocket over Tor (SOCKS5)** — O aplicativo agora consegue se conectar a trackers que possuem endereço `.onion`, permitindo um lobby global 100% anônimo.
- **Improved Monitoring** — O comando de debug do tracker agora mostra os IDs dos arquivos (`file_id`) para facilitar o diagnóstico de conectividade.

## [0.7.3] — 2026-03-17

### Localização

- **Mudança para Inglês (Default)** — O aplicativo agora inicia em Inglês por padrão, visando um público global.
- **Fallback de Idioma** — O sistema de i18n foi robustecido para evitar telas de carregamento infinitas se um arquivo de tradução faltar.

## [0.7.2] — 2026-03-17

### Corrigido

- **Download 404** — Corrigido erro onde o download falhava com 404 se o peer demorasse a subir o serviço.
- **Tor Bootstrap** — Otimização no tempo de espera do Tor para 90 segundos antes de dar timeout.

## [0.7.1] — 2026-03-17

### Fixed

- **Infinite Loading Screen** — Implementado um "Skip/Bypass" para a tela de carregamento se o Tor demorar mais de 60 segundos para iniciar.

## [0.7.0] — 2026-03-17

### Added

- **Swarm Download** — Agora os arquivos são baixados de múltiplos peers simultaneamente se disponíveis no tracker.
- **Content Hashing (BLAKE3)** — Verificação de integridade ponta-a-ponta usando hashes BLAKE3 em cada chunk e no arquivo final.
- **Auto-Discovery LAN** — Suporte experimental para descoberta de peers na mesma rede local sem necessidade de endereço onion manual.

## [0.6.0] — 2026-03-16

### Added

- **Encrypted Chunks** — Divisão de arquivos em chunks de 256KB cifrados com XChaCha20-Poly1305.
- **Onion Service Management** — Controle automático do processo Tor e criação de serviços ocultos efêmeros.
- **Native GUI (egui)** — Interface moderna e responsiva implementada em Rust puro.
