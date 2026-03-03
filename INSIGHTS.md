# 💡 Insights, Métricas e Trabalhos Futuros para o TCC

Este documento compila as ideias de evolução arquitetônica e métricas de desempenho que podem elevar o **onion-poc** a um nível de excelência acadêmica e profissional. Como a entrega final é em dezembro, há tempo hábil para implementar ou fundamentar estas escolhas teóricas na monografia.

---

## 📊 1. Propostas de Métricas e Validação Científica (Para a Monografia)

A banca avaliadora valoriza projetos que provam suas escolhas técnicas através de testes empíricos. Aqui estão os experimentos planejados:

* **A. Impacto do Tamanho do Chunk (Benchmark de Tor):** Testar o tempo de transferência de um arquivo de 100MB utilizando diferentes tamanhos de blocos (ex: 256KB, 512KB, 1MB) para encontrar o "sweet spot" ideal de throughput na rede Tor instável.
* **B. Overhead Criptográfico (Custo CPU/RAM):** Medir o impacto no processador e na memória ao aplicar a criptografia **XChaCha20-Poly1305** chunk por chunk durante o download de arquivos gigantes (ex: 1GB), provando a eficiência da linguagem Rust frente à pesada camada de segurança.
* **C. Teste de Estresse do Servidor (Tracker):** Utilizar ferramentas de simulação de carga (ex: *k6* ou *Apache JMeter*) para enviar milhares de "Pings" simultâneos à porta 8080 do Docker/Axum, demonstrando a escalabilidade com baixo uso de RAM (State in Memory).
* **D. Mitigação de Traffic Analysis:** Capturar os pacotes de rede locais utilizando *Wireshark* para comprovar visualmente na monografia que o provedor de internet (ISP) enxerga apenas tráfego TLS opaco roteado para nós de entrada do Tor, protegendo o "Quem", o "O Quê" e o tamanho real do arquivo.

---

## 🚀 2. Evoluções Arquitetônicas "Estado da Arte" (Para Desenvolver)

Como o prazo se estende até dezembro, os seguintes recursos levariam o software de uma "PoC" para um protocolo peer-to-peer robusto:

### 🌳 2.1. Árvores de Merkle (Integridade nível Web3)

Atualmente o projeto assegura hashes por blocos independentes. Transitar para uma estrutura de **Merkle Tree**, com um *Root Hash* global trocado de antemão, permitiria ao cliente detectar e rejeitar blocos corrompidos ou maliciosos introduzidos por nós Tor de saída comprometidos de forma instantânea, reestruturando a confiança de forma puramente matemática.

### 🔀 2.2. Swarm e Download Multi-Peer

Evoluir a topologia 1-pra-1 para uma verdadeira arquitetura Swarm (estilo BitTorrent). Se dezenas de usuários possuírem o mesmo arquivo no "Lobby", o cliente Rust passará a solicitar assinaturas de chunks assincronamente a múltiplos Peers simulaneamente, mitigando o gargalo crônico de velocidade inerente ao proxy SOCKS5 do Tor.

### ❄️ 2.3. Pluggable Transports (Bypass de Deep Packet Inspection)

Implementar integração transparente com *obfs4* ou *Snowflake*, camuflando os handshakes do protocolo Tor em sessões simuladas de WebRTC (vídeo) ou tráfego HTTP benigno. Foco primordial na evasão de firewalls estatais agressivos (como os da China ou da Rússia) que detectam metadados Tor por comportamento.

### 🔗 2.4. Identidade Baseada em Assinaturas Ed25519

Tornar o "Tracker" trustless. Hoje o servidor aceita UUIDs crus. A evolução exigiria a geração de chaves assimétricas (`Ed25519`) em cada cliente após a inicialização. Adições e remoções no Tracker só seriam acatadas se assinadas digitalmente pelo respectivo dono do arquivo, obliterando ataques de envenenamento de lista global (Sybil Attacks).

### 🏠 2.5. Roteamento Híbrido Tor + LAN (mDNS / Zeroconf)

Adotar a descoberta local. Em campi universitários ou ambientes corporativos, o tráfego não deve voltar para a rede global pública de onion routing se o destino estiver no mesmo roteador. Identificação mDNS permitiria um handshake veloz pela WLAN, atingindo velocidades de taxa de disco (Gigabite) enquanto mantém ativa, porém localmente, a via SOCKS e a criptografia pesada XChaCha.
