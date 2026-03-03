# 1. Builder local: Usamos a imagem oficial do Rust para compilar o Tracker
FROM rust:1.92 as builder

# Cria uma pasta temporária para compilar o projeto inteiro
WORKDIR /usr/src/app
# Copia todos os arquivos do repositório onion_poc
COPY . .

# Compila apenas o executável do Tracker em modo release
RUN cargo build --release --bin tracker

# 2. Imagem final do Servidor: Super leve e focada apenas em rodar
FROM debian:bookworm-slim

# Instala certificados necessários para conexões (caso requeira)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copia do builder o binário gerado acima
COPY --from=builder /usr/src/app/target/release/tracker /usr/local/bin/tracker

# Define algumas env vars e a porta interna
ENV RUST_LOG=info
EXPOSE 8080

# Inicia o Servidor Rastreador
CMD ["tracker"]
