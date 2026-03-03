# 📚 Referencial Teórico para o TCC (Artigos e Links Reais)

Para a revisão bibliográfica e embasamento (Estado da Arte) do seu projeto, aqui estão publicações acadêmicas reais (com links diretos e PDFs) focadas em redes Peer-to-Peer, anonimato e a rede Tor.

*(Dica: Se a sua universidade tiver convênio com o portal CAPES, você consegue acessar os artigos pagos da IEEE gratuitamente logando com o seu e-mail institucional na opção "Acesso CAFe").*

### 1. DAENet: Making Strong Anonymity Scale in a Fully Decentralized Network (2022)

* **Tema:** Aborda a vulnerabilidade das redes tradicionais como o Tor contra ataques de Análise de Tráfego e propõe uma nova mix network P2P para escalar o anonimato de forma eficiente.
* **Onde usar:** Justificativa da arquitetura descentralizada.
* **Publicação:** *IEEE Transactions on Dependable and Secure Computing (Volume: 20, Issue: 2).*
* 🔗 **DOI:** [10.1109/TDSC.2022.3182855](https://doi.org/10.1109/TDSC.2022.3182855)
* 📄 **Link Original:** [Acessar no IEEE Xplore](https://ieeexplore.ieee.org/document/9796035)

### 2. IPFS and Friends: A Qualitative Comparison of Next Generation Peer-to-Peer Data Networks (2022)

* **Tema:** Uma revisão comparativa completa das tecnologias de redes P2P de próxima geração. Afirma que garantir forte anonimato resistente à censura na transferência P2P continua sendo um desafio global.
* **Onde usar:** Perfeito para a sua Introdução. Você pode citar as conclusões desse artigo para provar que o seu software preenche uma lacuna atual (já que seu TCC traz uma GUI e uma P2P sobre Tor com XChaCha20).
* **Publicação:** *IEEE Communications Surveys & Tutorials (Volume: 24, Issue: 4).*
* 🔗 **DOI:** [10.1109/COMST.2022.3190666](https://doi.org/10.1109/COMST.2022.3190666)
* 📄 **Link Open Access (PDF):** [Baixar PDF completo e gratuito no arXiv](https://arxiv.org/pdf/2202.13110.pdf)

### 3. A Survey of the Tor Anonymity Network: Fundamentals and Research Challenges

* **Tema:** Compreender totalmente a estrutura do onion routing. Este paper ou publicações similares compilam os gargalos do projeto Tor e o uso dos *Onion Services* (.onion) para hospedar dados não-rastreáveis.
* **Onde usar:** Obrigatório para o capítulo (Fundamentação Teórica) que irá explicar o que é o Tor e como ele oculta o endereço IP no seu aplicativo.
* 📄 **Link Open Access (PDFs e Surveys Recentes):** Diferentes autores publicam varreduras parecidas ao longo de 2023–2025. É recomendada a busca por pesquisas de "Tor Anonymity" diretamente na base Open Access: [Buscar PDF gratuito na base arXiv](https://arxiv.org/search/cs?query=tor+anonymity&searchtype=all&abstracts=show&order=-announced_date_first&size=50)

### 4. Leitura Obrigatória Histórica (Artigo Original de 2004)

Apesar de ser mais antigo, é **obrigatório e indispensável** referenciar o paper original escrito pelos criadores da rede Tor naval. Todo TCC de segurança da informação faz isso:

* **Citação:** Dingledine, R., Mathewson, N., & Syverson, P. (2004). *Tor: The second-generation onion router.* USENIX Security Symposium.
* 📄 **Link Oficial (PDF Liberado):** [Baixar Tor Design Paper Oficial](https://svn-archive.torproject.org/svn/projects/design-paper/tor-design.pdf)

---

### 🎓 Dica Prática de Engenharia Social Acadêmica para Livros (Opcional)

Se o professor pedir citações clássicas de **"Livros Base"** sobre arquitetura Peer-to-Peer, você pode utilizar:

* *Peer-to-Peer Computing: Applications, Architecture, Protocols, and Challenges* (Yu-Kwong Ricky Kwok). É a bíblia sagrada pra embasar conceitualmente a lógica do seu *Tracker* em relação aos nós *P2P*.

Se você esbarrar em restrições de *Paywall* de $30 dólares em portais, sempre pegue o nome do artigo que achou e pesquise-o no **[Google Scholar (Google Acadêmico)](https://scholar.google.com.br/)**. Eles frequentemente indexam a versão PDF pura nos links `[PDF] researchgate.net` no lado direito da tela.
