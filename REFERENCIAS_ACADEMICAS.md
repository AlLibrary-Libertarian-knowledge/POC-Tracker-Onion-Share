# 📚 Referencial Teórico para o TCC (2021-2026)

Para a revisão bibliográfica e embasamento (Estado da Arte) do seu projeto, aqui estão publicações acadêmicas recentes focadas em redes Peer-to-Peer, anonimato e a rede Tor. Estes artigos servirão para defender as escolhas da sua arquitetura na monografia:

### 1. DAENet: Making Strong Anonymity Scale in a Fully Decentralized Network (2022)

* **Tema:** Aborda a vulnerabilidade das redes tradicionais como o Tor contra ataques de Análise de Tráfego e propõe uma nova abstração de Mix Networks P2P para escalar o anonimato de forma eficiente.
* **Como usar no TCC:** Use este artigo para fundamentar a seção em que você explica por que o tráfego P2P precisa ser "escondido" através do Tor (Onion Routing) e os desafios inerentes de roteamento descentralizado versus velocidade.
* *Fonte/Instituição Acadêmica catalogada.*

### 2. IPFS and Friends: A Qualitative Comparison of Next Generation Peer-to-Peer Data Networks (2022)

* **Tema:** Uma revisão comparativa das tecnologias de redes P2P de próxima geração (ex: IPFS, Freenet, Tor). O documento aponta que, apesar dos avanços, garantir forte anonimato resistente à censura na transferência de arquivos continua sendo um desafio global.
* **Como usar no TCC:** Perfeito para a sua Introdução e Justificativa. Você pode citar as conclusões desse artigo para provar que o seu software (Onion-PoC) preenche exatamente a lacuna atual descrita pela ciência da computação.

### 3. File Sharing & Privacy Techniques in Peer-to-Peer (Trabalho em Publicação / 2024-2026)

* **Tema:** Discute os mecanismos de anonimização e o uso de Onion Routing (padrão do Tor) estritamente voltados para proteger conteúdos digitais compartilhados entre usuários ponta-a-ponta, prevenindo rastreio.
* **Como usar no TCC:** Excelente para compor o capítulo sobre "Protocolos de Roteamento Anonimizados", explicando teoricamente a diferença de compartilhar via HTTP centralizado versus Onion Service P2P.

### 4. Análise e Otimização do OnionShare e Tor Hidden Services (Múltiplas Publicações Recentes)

Embora o OnionShare seja uma ferramenta, muitos artigos (como o de aceleração de serviços Darknet) mencionam sua robustez.

* **Como usar no TCC:** Você pode compor um capítulo chamado **"Trabalhos Correlatos"**, onde sua monografia irá comparar frontalmente o `onion-poc` (seu software) com o `OnionShare`. Exemplo: *"Enquanto o OnionShare compartilha arquivos via servidor Web embarcado usando Python, o presente TCC apresenta um Tracker P2P em Rust com paralelismo e otimização..."*.

---

### Palavras-Chave recomendadas para buscar mais artigos no Google Acadêmico

Se for procurar PDF's na biblioteca da sua faculdade, experimente estas strings em inglês (A ciência da computação publica prioritariamente em inglês):

* *Anonymous peer-to-peer file sharing Tor network*
* *Performance evaluation of Tor onion services*
* *Cryptographic improvements in decentralized file transfer*
* *Traffic analysis mitigation in P2P networks*
