pub mod app;
pub mod bg;
pub mod shared;

use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::config::AppConfig;
use shared::SharedState;

pub fn run() -> Result<()> {
    let config = AppConfig::load();
    let tor_path = config.effective_tor_path();

    // Estado compartilhado entre GUI e background
    let shared = Arc::new(Mutex::new(SharedState::default()));

    // Inicia background em thread separada (com runtime tokio próprio)
    let shared_bg = Arc::clone(&shared);
    let tp = tor_path.clone();
    std::thread::Builder::new()
        .name("bg-manager".into())
        .spawn(move || bg::run_blocking(shared_bg, tp))
        .expect("thread spawn");

    // Se Tor está disponível e termos foram aceitos → inicia automaticamente
    if config.terms_accepted && config.tor_available() {
        shared
            .lock()
            .unwrap()
            .control_queue
            .push(shared::GuiControl::StartTor);
    }

    // Janela nativa
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("🧅 onion-poc")
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "onion-poc",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::GuiApp::new(cc, shared, config)) as Box<dyn eframe::App>)),
    )
    .map_err(|e| anyhow::anyhow!("{}", e))
}
