# 🧭 Deploy do Servidor Tracker (O "Lobby Global")

Bem-vindo ao manual do Tracker! Esta é a peça central da rede do seu TCC. Ele é incrivelmente pequeno e não guarda arquivos, mas é ele quem funciona como a "Lista Telefônica" anônima para a sua aba de **BUSCAR**.

---

## ❓ Pergunta: "Se eu deixar ele rodando no meu PC hoje (como `127.0.0.1:8080`), quem baixar o meu `.exe` pelo mundo consegue usar a rede?"

**Resposta:** ❌ Não!
Se o seu código-fonte está configurado com `127.0.0.1`, quando um usário no Japão abrir o aplicativo, o aplicativo tentará se conectar à rede local do computador *dele* (no Japão), e não ao *seu* Tracker no Brasil. Ele dirá "Nenhum usuário online".

Para fazer o seu PoC funcionar de verdade mundo afora, você tem que dar **um endereço universal** para o seu Tracker e colocá-lo dentro do Aplicativo. É aí que a rede Tor entra em cena novamente! Ao invés de usar IP ou expor roteador, criamos um Link Onion para o servidor.

---

## 🚀 Como Fazer o Deploy Oficial em 5 Minutos

Nesta pasta (`deploy/`), automatizei todas as regras para que você nem sequer precise instalar o Tor no seu servidor. Tudo virou um **Docker**.
Você pode rodar isso em uma VPS barata (AWS, DigitalOcean) ligada 24h, ou até no seu PC local para as bancas do seu TCC.

### Passo 1: Ligar o Docker Compose

No terminal, dentro desta mesma pasta (`deploy`), rode o comando:

```bash
docker compose up -d --build
```

> **O que isso faz?** Isso baixa a imagem Alpine do Tor, constrói a pequena imagem Rust (sua aplicação tracker 8080), amarra as duas numa sub-rede do Docker, as blinda e esconde do mundo físico. Todo tráfego passa obrigatoriamente pela rede Tor, atuando sob "Confiança Zero", sem vazar seu IP de casa.

### Passo 2: Descobrir o seu Link ".onion" Oficial

Assim que a mágica acontecer acima, o contêiner do Tor conversará com a Diretoria Global da Deep Web e registrará uma chave criptográfica permanentemente pra você.

Vamos descobrir qual "Endereço Web" ele te deu com este comando:

```bash
docker compose exec tor_service cat /var/lib/tor/hidden_service/hostname
```

A saída será algo como:
`zxcy4abcedfg...xyz.onion`

🎊 **Parabéns! Esse é o IP/Domínio permanente do seu Servidor P2P para toda a vida!** 🎊

### Passo 3: Colar o Link dentro do seu Aplicativo Gráfico

Guarde o Link! Ele será introduzido no código do Cliente (no arquivo `src/config.rs`).

1. Abra seu código em `src/config.rs`
2. Vá até a função `impl Default for AppConfig`
3. Troque a variável `tracker_url` para algo assim:

```rust
tracker_url: "http://zxcy4abcedfg...xyz.onion".to_string(),
```

> *(Atenção, o prefixo DEVE ser `http://` ao invés de `https://` porque a própria rede do Tor já criptografa tudo ponta-a-ponta)*

### Passo 4: Fazer Build Final do `.exe`

Agora que o seu App gráfico já sabe que **a casa oficial da rede** se chama `http://zxcy...onion`, todo mundo que o Github Actions compilar passará a ler esse endereço!
Mesmo a pessoa que baixe o `.exe` no Japão usará os túneis invisíveis do Tor dela até os túneis invisíveis do seu servidor P2P (mesmo rodando em Docker no Brasil) - funcionando a aba Buscar e os Contadores de Users de forma limpa, segura e anônima para ambos os lados.

---

## 🔍 Monitoramento e Troubleshooting

### Ver quantos PCs estão conectados (Lobby Real)

Você pode ver a lista bruta de IDs de máquinas conectadas no seu servidor através deste comando no terminal do seu servidor:

```bash
docker compose exec tracker curl -s http://localhost:8080/debug/nodes
```

### ⚠️ Problema: "Tenho várias máquinas mas só aparece 1 Online"

Se você abrir o app em dois PCs e marcar apenas "1 online", o motivo mais provável é **Colisão de ID de Nó**.

**Por que acontece?**
Cada aplicativo gera um `node_id` único no seu arquivo de configuração na primeira vez que abre. Se você baixou o app e **copiou a pasta inteira** (incluindo a pasta de dados do usuário) de um computador para o outro, ambos terão o *mesmo ID*. O servidor vê isso como se fosse o mesmo PC trocando de link e "sobrescreve" a conexão.

**Como resolver:**

1. No computador onde o ID está repetido, feche o aplicativo.
2. Delete o arquivo de configuração localizado em:
   - **Linux:** `~/.config/br.tcc/onion_poc/config.json`
   - **Windows:** `%AppData%\br.tcc\onion_poc\config.json`
3. Abra o app novamente. Ele gerará um novo ID único e agora o Tracker mostrará "2 online".

---

## 🔒 Dica de Ouro sobre as Chaves

 Lembre-se: Dentro deste `docker-compose.yml`, deixei mapeado um Volume chamado `tor_keys`. Ele protege literalmente seu "Domínio" contra perda de dados. Se amanhã você cancelar sua VPS ou desligar o PC, para "reciclar" o mesmo link Onion, você precisa transferir/fazer backup dos arquivos que foram gerados ali pelo Docker. Se um dia perder isso, o Docker inventará *um novo link .onion* quando ligar de novo e os apps antigos que você distribuiu mundo afora se perderão dele!
