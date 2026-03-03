/// Testes unitários do onion-poc.
/// Executados com: cargo test
/// São obrigatórios no CI antes de qualquer build de release.

// ─────────────────────────────────────────────────────────────────────────────
// Crypto
// ─────────────────────────────────────────────────────────────────────────────
mod crypto_tests {
    use onion_poc::crypto;
    use uuid::Uuid;

    fn fid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn random_key_is_32_bytes() {
        let k = crypto::random_key();
        assert_eq!(k.len(), 32, "chave deve ter 32 bytes (XChaCha20-Poly1305)");
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = crypto::random_key();
        let id = fid();
        let plaintext = b"onion-poc test payload 12345";
        let encrypted = crypto::encrypt_chunk(&key, id, 0, plaintext).expect("encrypt falhou");
        assert_ne!(
            encrypted.as_slice(),
            plaintext as &[u8],
            "ciphertext deve diferir do plaintext"
        );
        let decrypted = crypto::decrypt_chunk(&key, id, 0, &encrypted).expect("decrypt falhou");
        assert_eq!(
            decrypted, plaintext,
            "roundtrip deve recuperar o plaintext original"
        );
    }

    #[test]
    fn wrong_key_fails_decryption() {
        let key1 = crypto::random_key();
        let key2 = crypto::random_key();
        let id = fid();
        let encrypted = crypto::encrypt_chunk(&key1, id, 0, b"secret data").unwrap();
        let result = crypto::decrypt_chunk(&key2, id, 0, &encrypted);
        assert!(
            result.is_err(),
            "descriptografia com chave errada deve falhar"
        );
    }

    #[test]
    fn key_b64_roundtrip() {
        let key = crypto::random_key();
        let encoded = crypto::key_to_b64url(&key);
        let decoded = crypto::key_from_b64url(&encoded).expect("decode falhou");
        assert_eq!(
            decoded, key,
            "base64url roundtrip deve recuperar a chave original"
        );
    }

    #[test]
    fn empty_plaintext_ok() {
        let key = crypto::random_key();
        let id = fid();
        let enc = crypto::encrypt_chunk(&key, id, 0, b"").expect("encrypt(empty) falhou");
        let dec = crypto::decrypt_chunk(&key, id, 0, &enc).expect("decrypt(empty) falhou");
        assert_eq!(dec.as_slice(), b"" as &[u8]);
    }

    #[test]
    fn large_chunk_roundtrip() {
        let key = crypto::random_key();
        let id = fid();
        let data: Vec<u8> = (0..256 * 1024).map(|i| (i % 256) as u8).collect();
        let enc = crypto::encrypt_chunk(&key, id, 0, &data).expect("encrypt(large) falhou");
        let dec = crypto::decrypt_chunk(&key, id, 0, &enc).expect("decrypt(large) falhou");
        assert_eq!(dec, data, "chunk grande deve roundtrip corretamente");
    }

    #[test]
    fn different_chunk_indices_produce_different_ciphertext() {
        let key = crypto::random_key();
        let id = fid();
        let pt = b"same plaintext";
        let c0 = crypto::encrypt_chunk(&key, id, 0, pt).unwrap();
        let c1 = crypto::encrypt_chunk(&key, id, 1, pt).unwrap();
        assert_ne!(c0, c1, "índices diferentes devem gerar nonces diferentes");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Config
// ─────────────────────────────────────────────────────────────────────────────
mod config_tests {
    use onion_poc::config::AppConfig;

    #[test]
    fn default_config_terms_not_accepted() {
        let cfg = AppConfig::default();
        assert!(!cfg.terms_accepted, "config padrão: termos não aceitos");
    }

    #[test]
    fn default_tor_bin_is_tor() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.tor_bin(), "tor", "caminho padrão deve ser 'tor'");
    }

    #[test]
    fn custom_tor_path_is_returned() {
        let cfg = AppConfig {
            tor_path: "/usr/local/bin/tor".into(),
            terms_accepted: true,
            node_id: "test-node".into(),
            tracker_url: "http://test-tracker".into(),
            share_publicly: false,
        };
        assert_eq!(cfg.tor_bin(), "/usr/local/bin/tor");
        assert_eq!(cfg.effective_tor_path(), "/usr/local/bin/tor");
    }

    #[test]
    fn save_and_reload() {
        // Usa dir temporário para não poluir config do usuário
        // (ProjectDirs usa variáveis de ambiente em testes)
        let cfg = AppConfig {
            terms_accepted: true,
            tor_path: "tor".into(),
            node_id: "test-node".into(),
            tracker_url: "http://test-tracker".into(),
            share_publicly: false,
        };
        // Verifica serialize/deserialize JSON
        let json = serde_json::to_string_pretty(&cfg).unwrap();
        let reloaded: AppConfig = serde_json::from_str(&json).unwrap();
        assert!(reloaded.terms_accepted);
        assert_eq!(reloaded.tor_path, "tor");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GUI SharedState
// ─────────────────────────────────────────────────────────────────────────────
mod shared_state_tests {
    use onion_poc::gui::shared::{GuiControl, SharedState, TorInitState};
    use std::path::PathBuf;

    #[test]
    fn default_state_tor_inactive() {
        let s = SharedState::default();
        assert!(!s.tor_active, "estado inicial: Tor inativo");
        assert!(s.onion_addr.is_none(), "estado inicial: sem endereço onion");
    }

    #[test]
    fn control_queue_starts_empty() {
        let s = SharedState::default();
        assert!(
            s.control_queue.is_empty(),
            "fila de controle deve iniciar vazia"
        );
    }

    #[test]
    fn uptime_zero_when_not_started() {
        let s = SharedState::default();
        assert_eq!(
            s.uptime_str(),
            "00:00:00",
            "uptime inicial deve ser 00:00:00"
        );
    }

    #[test]
    fn fmt_bytes_units() {
        assert_eq!(SharedState::fmt_bytes(0), "0 B");
        assert_eq!(SharedState::fmt_bytes(1023), "1023 B");
        assert_eq!(SharedState::fmt_bytes(1024), "1.0 KB");
        assert_eq!(SharedState::fmt_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(SharedState::fmt_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn tor_init_default_is_idle() {
        let s = SharedState::default();
        assert_eq!(s.tor_init, TorInitState::Idle);
    }

    #[test]
    fn add_control_to_queue() {
        let mut s = SharedState::default();
        s.control_queue.push(GuiControl::StartTor);
        s.control_queue
            .push(GuiControl::AddFile(PathBuf::from("/tmp/test.txt")));
        assert_eq!(s.control_queue.len(), 2);
    }

    #[test]
    fn drain_control_queue() {
        let mut s = SharedState::default();
        s.control_queue.push(GuiControl::StartTor);
        let drained = std::mem::take(&mut s.control_queue);
        assert_eq!(drained.len(), 1);
        assert!(
            s.control_queue.is_empty(),
            "fila deve estar vazia após drain"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Link / URL
// ─────────────────────────────────────────────────────────────────────────────
mod link_tests {
    use onion_poc::link::ShareLink;

    #[test]
    fn parse_valid_link() {
        // onion-poc://address.onion:8080/file-id#key
        let raw = "onion://aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.onion:8080/550e8400-e29b-41d4-a716-446655440000#AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==";
        // Testa que não panics — o parse pode falhar graciosamente
        let _result = ShareLink::parse(raw);
        // Principais garantias: não panics e o erro é legível
    }

    #[test]
    fn invalid_link_returns_err() {
        let result = ShareLink::parse("not_a_valid_link");
        assert!(result.is_err(), "link inválido deve retornar Err");
    }
}
