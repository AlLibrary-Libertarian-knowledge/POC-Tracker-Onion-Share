/// GUI principal do onion-poc — egui/eframe
/// v0.8.1: paleta corrigida, file dialog não-bloqueante, auto-start Tor pós-termos, WebSocket support
use std::time::{Duration, Instant};

use egui::{Color32, FontId, RichText, Stroke, Vec2};
// use uuid::Uuid;
use crate::config::AppConfig;

use super::shared::{GuiControl, SharedFileInfo, SharedStateRef, TorInitState};

// ─────────────────────────────────────────────────────────────────────────────
// Paleta de cores — alto contraste, legível em fundo dark
// ─────────────────────────────────────────────────────────────────────────────
// REGRA: nunca use C_ACCENT como background de texto colorido no mesmo tom.
//        Sempre assegure ratio de contraste ≥ 4.5:1 (WCAG AA).

const C_BG: Color32 = Color32::from_rgb(11, 12, 21); // fundo principal
const C_PANEL: Color32 = Color32::from_rgb(19, 21, 35); // painéis
const C_PANEL2: Color32 = Color32::from_rgb(25, 28, 46); // painéis secundários
const C_SIDEBAR: Color32 = Color32::from_rgb(15, 17, 28); // sidebar

// Texto — sempre bright para contraste com os fundos dark
const C_TEXT: Color32 = Color32::from_rgb(225, 228, 248); // texto primário (#E1E4F8)
const C_TEXT2: Color32 = Color32::from_rgb(168, 174, 210); // texto secundário
const C_DIM: Color32 = Color32::from_rgb(108, 116, 155); // texto dim/hint

// Acentos — cores vibrantes legíveis sobre dark
const C_ACCENT: Color32 = Color32::from_rgb(105, 180, 252); // azul claro (#69B4FC)
const C_GREEN: Color32 = Color32::from_rgb(82, 215, 125); // verde (#52D77D)
const C_RED: Color32 = Color32::from_rgb(252, 90, 90); // vermelho (#FC5A5A)
const C_YELLOW: Color32 = Color32::from_rgb(255, 208, 70); // amarelo (#FFD046)
const C_CYAN: Color32 = Color32::from_rgb(60, 220, 200); // ciano (#3CDCC8)

// Bordas sutis
const C_BORDER: Color32 = Color32::from_rgb(42, 47, 78);
const C_BORDER_HL: Color32 = Color32::from_rgb(80, 110, 180);

// ─────────────────────────────────────────────────────────────────────────────
// Helpers de cor semi-transparente (SEMPRE unmultiplied → correto)
// ─────────────────────────────────────────────────────────────────────────────
fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

// ─────────────────────────────────────────────────────────────────────────────
// Views
// ─────────────────────────────────────────────────────────────────────────────
#[derive(Clone, PartialEq, Debug)]
pub enum View {
    Dashboard,
    Files,
    Download,
    Search,
    About,
}

// ─────────────────────────────────────────────────────────────────────────────
// GUI App
// ─────────────────────────────────────────────────────────────────────────────
pub struct GuiApp {
    pub shared: SharedStateRef,
    pub config: AppConfig,

    pub view: View,

    // Terms scroll tracking
    terms_scroll_y: f32,
    terms_content_h: f32,
    terms_viewport_h: f32,

    // Tor modal
    pub show_tor_modal: bool,
    tor_modal_start: Option<Instant>,

    // Files view
    pub search_query: String,
    drag_hover: bool,

    // Non-blocking file dialog
    file_dialog_rx: Option<std::sync::mpsc::Receiver<Option<Vec<std::path::PathBuf>>>>,
    folder_dialog_rx: Option<std::sync::mpsc::Receiver<Option<std::path::PathBuf>>>,

    // Download state
    download_link_input: String,
    download_dir: Option<std::path::PathBuf>,

    // Status + clipboard feedback
    status_msg: Option<(String, Instant, Color32)>,
    clipboard_msg: Option<(String, Instant)>,

    // WAN discovery
    pub bootstrap_peer_input: String,
}

impl GuiApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        shared: SharedStateRef,
        config: AppConfig,
    ) -> Self {
        apply_theme(&cc.egui_ctx);

        Self {
            shared,
            config,
            view: View::Dashboard,
            terms_scroll_y: 0.0,
            terms_content_h: 9999.0,
            terms_viewport_h: 300.0,
            show_tor_modal: false,
            tor_modal_start: None,
            search_query: String::new(),
            drag_hover: false,
            file_dialog_rx: None,
            folder_dialog_rx: None,
            download_link_input: String::new(),
            download_dir: directories::UserDirs::new()
                .and_then(|u| u.download_dir().map(|p| p.to_path_buf())),
            status_msg: None,
            clipboard_msg: None,
            bootstrap_peer_input: String::new(),
        }
    }

    fn send(&self, cmd: GuiControl) {
        self.shared.lock().unwrap().control_queue.push(cmd);
    }

    fn set_status(&mut self, msg: impl Into<String>, color: Color32) {
        self.status_msg = Some((msg.into(), Instant::now(), color));
    }

    fn toggle_tor(&mut self) {
        let active = self.shared.lock().unwrap().tor_active;
        if active {
            self.send(GuiControl::StopTor);
            self.show_tor_modal = false;
        } else {
            self.send(GuiControl::StartTor);
            self.show_tor_modal = true;
            self.tor_modal_start = Some(Instant::now());
        }
    }

    /// Abre o file dialog em thread separada — não bloqueia a UI
    fn open_file_dialog(&mut self) {
        if self.file_dialog_rx.is_some() {
            return; // já tem um dialog aberto
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.file_dialog_rx = Some(rx);
        std::thread::spawn(move || {
            // rfd é bloqueante — roda na thread separada
            let files = rfd::FileDialog::new()
                .set_title("Selecionar arquivo para compartilhar")
                .pick_files();
            let _ = tx.send(files); // envia resultado (None se cancelou)
        });
    }

    /// Abre o folder dialog em thread separada
    fn open_folder_dialog(&mut self) {
        if self.folder_dialog_rx.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        self.folder_dialog_rx = Some(rx);
        std::thread::spawn(move || {
            let folder = rfd::FileDialog::new()
                .set_title("Selecionar pasta para Downloads")
                .pick_folder();
            let _ = tx.send(folder);
        });
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut vis = egui::Visuals::dark();

    vis.panel_fill = C_PANEL;
    vis.window_fill = C_PANEL2;
    vis.extreme_bg_color = C_BG;
    vis.override_text_color = Some(C_TEXT);

    // widgets
    vis.widgets.noninteractive.bg_fill = C_PANEL2;
    vis.widgets.noninteractive.bg_stroke = Stroke::new(1.0, C_BORDER);
    vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0, C_TEXT2);
    vis.widgets.inactive.bg_fill = C_PANEL;
    vis.widgets.inactive.bg_stroke = Stroke::new(1.0, C_BORDER);
    vis.widgets.inactive.fg_stroke = Stroke::new(1.0, C_TEXT);
    vis.widgets.hovered.bg_fill = Color32::from_rgb(32, 38, 64);
    vis.widgets.hovered.bg_stroke = Stroke::new(1.5, C_BORDER_HL);
    vis.widgets.active.bg_fill = C_ACCENT;
    vis.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    vis.selection.bg_fill = with_alpha(C_ACCENT, 55);
    vis.selection.stroke = Stroke::new(1.0, C_ACCENT);
    vis.window_rounding = egui::Rounding::same(8.0);
    vis.menu_rounding = egui::Rounding::same(6.0);
    vis.hyperlink_color = C_CYAN;
    vis.warn_fg_color = C_YELLOW;
    vis.error_fg_color = C_RED;

    ctx.set_visuals(vis);
    ctx.set_pixels_per_point(1.15);
}

// ─────────────────────────────────────────────────────────────────────────────
// eframe::App
// ─────────────────────────────────────────────────────────────────────────────
impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));

        // ── Expira mensagens ───────────────────────────────────────────────
        if let Some((_, t, _)) = &self.status_msg {
            if t.elapsed() > Duration::from_secs(5) {
                self.status_msg = None;
            }
        }
        if let Some((_, t)) = &self.clipboard_msg {
            if t.elapsed() > Duration::from_secs(3) {
                self.clipboard_msg = None;
            }
        }

        // ── Resultado do file dialog (thread separada) ─────────────────────
        if let Some(rx) = &self.file_dialog_rx {
            if let Ok(result) = rx.try_recv() {
                self.file_dialog_rx = None;
                if let Some(files) = result {
                    let tor_active = self.shared.lock().unwrap().tor_active;
                    if !tor_active {
                        self.set_status(
                            "⚠ Ative o OnionShare primeiro para compartilhar arquivos.",
                            C_YELLOW,
                        );
                    } else {
                        for f in files {
                            self.send(GuiControl::AddFile(f.clone()));
                            self.set_status(
                                format!(
                                    "📤 Adicionando: {}",
                                    f.file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default()
                                ),
                                C_GREEN,
                            );
                        }
                    }
                }
                // Se result == None: usuário cancelou, não faz nada (sem freeze)
            }
        }

        if let Some(rx) = &self.folder_dialog_rx {
            if let Ok(result) = rx.try_recv() {
                self.folder_dialog_rx = None;
                if let Some(folder) = result {
                    self.download_dir = Some(folder);
                }
            }
        }

        // ── Lê estado compartilhado ────────────────────────────────────────
        let (
            tor_active,
            tor_init,
            onion_addr,
            _online_now,
            uptime,
            total_sessions,
            total_bytes,
            chunks_served,
            shared_files,
            active_downloads,
            global_lobby,
        ) = {
            let s = self.shared.lock().unwrap();
            (
                s.tor_active,
                s.tor_init.clone(),
                s.onion_addr.clone(),
                s.online_now,
                s.uptime_str(),
                s.total_sessions,
                s.total_bytes,
                s.chunks_served,
                s.shared_files.clone(),
                s.active_downloads.clone(),
                s.global_lobby.clone(),
            )
        };

        // Fecha modal quando Tor pronto / erro
        if self.show_tor_modal {
            match &tor_init {
                TorInitState::Ready => {
                    self.show_tor_modal = false;
                    self.set_status("✅ OnionShare ativado com sucesso!", C_GREEN);
                }
                TorInitState::Error(e) => {
                    self.show_tor_modal = false;
                    self.set_status(format!("❌ Tor: {}", e), C_RED);
                }
                _ => {}
            }
        }

        // ── Drag & Drop ────────────────────────────────────────────────────
        self.drag_hover = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            if tor_active {
                self.send(GuiControl::AddFile(path.clone()));
                self.set_status(
                    format!(
                        "📤 Compartilhando: {}",
                        path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default()
                    ),
                    C_GREEN,
                );
            } else {
                self.set_status("⚠ Ative o OnionShare antes de soltar arquivos.", C_YELLOW);
            }
        }

        // ── Termos de Uso ─────────────────────────────────────────────────
        if !self.config.terms_accepted {
            self.draw_terms(ctx);
            return;
        }

        // ── Modal de ativação ─────────────────────────────────────────────
        if self.show_tor_modal {
            self.draw_tor_modal(ctx, &tor_init);
        }

        // ── Overlay de drag ────────────────────────────────────────────────
        if self.drag_hover {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("drag_overlay"),
            ));
            let rect = ctx.screen_rect();
            painter.rect_filled(rect, 12.0, with_alpha(C_ACCENT, 18));
            painter.rect_stroke(rect.shrink(3.0), 12.0, Stroke::new(3.0, C_ACCENT));
            painter.text(
                rect.center() - Vec2::new(0.0, 24.0),
                egui::Align2::CENTER_CENTER,
                "🗂  Solte para compartilhar",
                FontId::proportional(26.0),
                C_ACCENT,
            );
            painter.text(
                rect.center() + Vec2::new(0.0, 18.0),
                egui::Align2::CENTER_CENTER,
                "Criptografia XChaCha20-Poly1305 automática",
                FontId::proportional(13.0),
                C_TEXT2,
            );
        }

        // ── Render ─────────────────────────────────────────────────────────
        self.draw_topbar(
            ctx,
            tor_active,
            &onion_addr,
            global_lobby.online_nodes.max(1), // Pelo menos o usuário local na descoberta LAN
            &uptime,
        );
        self.draw_sidebar(ctx, tor_active);
        self.draw_statusbar(ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(C_BG)
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| match self.view {
                View::Dashboard => self.draw_dashboard(
                    ui,
                    tor_active,
                    &onion_addr,
                    global_lobby.online_nodes.max(1), // Nós ativos na descoberta descentralizada
                    &uptime,
                    total_sessions,
                    total_bytes,
                    chunks_served,
                    &shared_files,
                ),
                View::Files => self.draw_files(ui, tor_active, &shared_files),
                View::Download => self.draw_download(ui, tor_active, &active_downloads),
                View::Search => self.draw_search(ui, &global_lobby.files),
                View::About => self.draw_about(ui),
            });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Componentes
// ─────────────────────────────────────────────────────────────────────────────
impl GuiApp {
    // ── Top Bar ───────────────────────────────────────────────────────────────
    fn draw_topbar(
        &self,
        ctx: &egui::Context,
        tor_active: bool,
        onion_addr: &Option<String>,
        online_now: usize,
        uptime: &str,
    ) {
        egui::TopBottomPanel::top("topbar")
            .frame(
                egui::Frame::none()
                    .fill(C_SIDEBAR)
                    .inner_margin(egui::Margin::symmetric(16.0, 8.0)),
            )
            .min_height(46.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🧅 onion-poc")
                            .size(17.0)
                            .color(C_ACCENT)
                            .strong(),
                    );

                    if let Some(addr) = onion_addr {
                        ui.add_space(10.0);
                        let short = addr.get(..20).unwrap_or(addr);
                        ui.label(RichText::new(format!("{}…", short)).size(10.5).color(C_DIM));
                    }

                    ui.add_space(16.0);

                    // Badge Tor
                    let (color, text) = if tor_active {
                        (C_GREEN, "● Tor ATIVO")
                    } else {
                        (C_RED, "○ Tor INATIVO")
                    };
                    badge(ui, text, color);

                    ui.add_space(14.0);
                    ui.label(RichText::new("👥").size(13.0));
                    ui.label(
                        RichText::new(format!("{} online", online_now))
                            .size(12.5)
                            .color(C_CYAN),
                    );

                    ui.add_space(14.0);
                    ui.label(RichText::new("⏱").size(13.0).color(C_DIM));
                    ui.label(RichText::new(uptime).size(12.5).color(C_TEXT2));
                });
            });
    }

    // ── Sidebar ───────────────────────────────────────────────────────────────
    fn draw_sidebar(&mut self, ctx: &egui::Context, tor_active: bool) {
        egui::SidePanel::left("sidebar")
            .frame(
                egui::Frame::none()
                    .fill(C_SIDEBAR)
                    .inner_margin(egui::Margin::same(10.0)),
            )
            .min_width(175.0)
            .max_width(192.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new("NAVEGAÇÃO").size(9.5).color(C_DIM).strong());
                ui.add_space(4.0);

                nav_btn(
                    ui,
                    "📊  Dashboard",
                    self.view == View::Dashboard,
                    &mut self.view,
                    View::Dashboard,
                );
                nav_btn(
                    ui,
                    "📂  Arquivos",
                    self.view == View::Files,
                    &mut self.view,
                    View::Files,
                );
                nav_btn(
                    ui,
                    "📥  Baixar",
                    self.view == View::Download,
                    &mut self.view,
                    View::Download,
                );
                nav_btn(
                    ui,
                    "🔍  Buscar",
                    self.view == View::Search,
                    &mut self.view,
                    View::Search,
                );
                nav_btn(
                    ui,
                    "ℹ️   Sobre",
                    self.view == View::About,
                    &mut self.view,
                    View::About,
                );

                ui.add_space(12.0);
                ui.add(egui::Separator::default().spacing(4.0));
                ui.add_space(12.0);

                ui.label(RichText::new("ONION SHARE").size(9.5).color(C_DIM).strong());
                ui.add_space(8.0);

                let (btn_label, btn_fg, btn_bg) = if tor_active {
                    ("🔴  Desativar", C_RED, with_alpha(C_RED, 30))
                } else {
                    ("🟢  Ativar", C_GREEN, with_alpha(C_GREEN, 30))
                };

                let toggle =
                    egui::Button::new(RichText::new(btn_label).size(13.5).color(btn_fg).strong())
                        .min_size(Vec2::new(158.0, 36.0))
                        .fill(btn_bg)
                        .stroke(Stroke::new(1.5, btn_fg))
                        .rounding(egui::Rounding::same(7.0));

                if ui.add(toggle).clicked() {
                    self.toggle_tor();
                }

                if !tor_active {
                    ui.add_space(5.0);
                    ui.label(
                        RichText::new("Ative para começar\na compartilhar via Tor")
                            .size(10.0)
                            .color(C_DIM),
                    );
                }

                ui.add_space(14.0);
                ui.add(egui::Separator::default().spacing(4.0));
                ui.add_space(8.0);
                ui.label(
                    RichText::new("💡 Arraste arquivos\npara compartilhar")
                        .size(9.5)
                        .color(C_DIM),
                );

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("v0.8.1 • MIT License").size(9.0).color(C_DIM));
                });
            });
    }

    // ── Status Bar ────────────────────────────────────────────────────────────
    fn draw_statusbar(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("statusbar")
            .frame(
                egui::Frame::none()
                    .fill(C_SIDEBAR)
                    .inner_margin(egui::Margin::symmetric(16.0, 6.0)),
            )
            .min_height(28.0)
            .show(ctx, |ui| {
                let (text, color) = if let Some((msg, _, c)) = &self.status_msg {
                    (msg.as_str(), *c)
                } else if let Some((msg, _)) = &self.clipboard_msg {
                    (msg.as_str(), C_GREEN)
                } else {
                    ("Pronto — arraste arquivos ou clique em Ativar.", C_DIM)
                };
                ui.label(RichText::new(text).size(11.5).color(color));
            });
    }

    // ── Dashboard ─────────────────────────────────────────────────────────────
    fn draw_dashboard(
        &self,
        ui: &mut egui::Ui,
        tor_active: bool,
        onion_addr: &Option<String>,
        online_now: usize,
        uptime: &str,
        total_sessions: u64,
        total_bytes: u64,
        chunks_served: u64,
        shared_files: &[SharedFileInfo],
    ) {
        ui.label(
            RichText::new("📊 Dashboard")
                .size(17.0)
                .color(C_TEXT)
                .strong(),
        );
        ui.add_space(10.0);

        // ── Stat cards (linha) ─────────────────────────────────────────────
        ui.horizontal(|ui| {
            stat_card(ui, "👥 Online", &format!("{}", online_now), C_CYAN);
            stat_card(ui, "📊 Sessões", &format!("{}", total_sessions), C_ACCENT);
            stat_card(
                ui,
                "📦 Enviados",
                &crate::gui::shared::SharedState::fmt_bytes(total_bytes),
                C_GREEN,
            );
            stat_card(ui, "⏱ Uptime", uptime, C_YELLOW);
        });

        ui.add_space(12.0);

        // ── 2 colunas ─────────────────────────────────────────────────────
        let w = (ui.available_width() - 14.0) / 2.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(w, 200.0), |ui| {
                card(ui, "🌐 Status da Rede", |ui| {
                    let (clr, txt) = if tor_active {
                        (C_GREEN, "● ATIVO")
                    } else {
                        (C_RED, "○ INATIVO")
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Status:").color(C_DIM).size(12.0));
                        ui.label(RichText::new(txt).color(clr).strong().size(14.0));
                    });
                    ui.add_space(6.0);
                    ui.label(RichText::new("Endereço Onion:").color(C_DIM).size(10.5));
                    let onion = onion_addr.as_deref().unwrap_or("— não iniciado —");
                    ui.label(
                        RichText::new(onion)
                            .color(if tor_active { C_CYAN } else { C_DIM })
                            .size(10.5)
                            .monospace(),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("🔒 Criptografia XChaCha20-Poly1305 por chunk")
                            .color(C_TEXT2)
                            .size(11.0),
                    );
                });
            });

            ui.add_space(14.0);

            ui.allocate_ui(Vec2::new(w, 200.0), |ui| {
                card(ui, "📈 Atividade Recente", |ui| {
                    if total_sessions == 0 && shared_files.is_empty() {
                        ui.label(
                            RichText::new(
                                "Sem atividade ainda.\nAtive o OnionShare e compartilhe arquivos.",
                            )
                            .color(C_DIM)
                            .size(12.0),
                        );
                    } else {
                        if total_sessions > 0 {
                            ui.label(
                                RichText::new(format!("✔ {} sessão(ões)", total_sessions))
                                    .color(C_GREEN)
                                    .size(12.0),
                            );
                        }
                        if chunks_served > 0 {
                            ui.label(
                                RichText::new(format!("⬆ {} chunks servidos", chunks_served))
                                    .color(C_CYAN)
                                    .size(12.0),
                            );
                        }
                        if !shared_files.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "🔒 {} arquivo(s) ativo(s)",
                                    shared_files.len()
                                ))
                                .color(C_ACCENT)
                                .size(12.0),
                            );
                        }
                    }
                });
            });
        });

        ui.add_space(12.0);

        // ── Lista de arquivos ──────────────────────────────────────────────
        card(
            ui,
            &format!("📤 Arquivos Compartilhados ({})", shared_files.len()),
            |ui| {
                if shared_files.is_empty() {
                    ui.label(
                        RichText::new("Nenhum arquivo compartilhado.")
                            .color(C_DIM)
                            .size(12.0),
                    );
                } else {
                    for f in shared_files {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔒").color(C_CYAN).size(13.0));
                            ui.label(RichText::new(&f.name).color(C_TEXT).size(12.0));
                            ui.label(
                                RichText::new(crate::gui::shared::SharedState::fmt_bytes(f.size))
                                    .color(C_DIM)
                                    .size(11.0),
                            );
                        });
                    }
                }
            },
        );
    }

    // ── Files ─────────────────────────────────────────────────────────────────
    fn draw_files(&mut self, ui: &mut egui::Ui, tor_active: bool, shared_files: &[SharedFileInfo]) {
        // Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("📂 Arquivos Compartilhados")
                    .size(17.0)
                    .color(C_TEXT)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let loading = self.file_dialog_rx.is_some();
                let add_label = if loading {
                    "⏳ Aguardando…"
                } else {
                    "＋ Adicionar arquivo"
                };
                let add_btn =
                    egui::Button::new(RichText::new(add_label).color(Color32::WHITE).size(12.5))
                        .min_size(Vec2::new(148.0, 30.0))
                        .fill(if loading { C_DIM } else { C_ACCENT })
                        .rounding(egui::Rounding::same(6.0));

                if ui.add_enabled(!loading, add_btn).clicked() {
                    if !tor_active {
                        self.set_status("⚠ Ative o OnionShare primeiro.", C_YELLOW);
                    } else {
                        self.open_file_dialog(); // não bloqueia
                    }
                }
            });
        });

        ui.add_space(8.0);

        if !tor_active {
            let warn = egui::Frame::none()
                .fill(with_alpha(C_YELLOW, 18))
                .stroke(Stroke::new(1.0, with_alpha(C_YELLOW, 90)))
                .rounding(egui::Rounding::same(7.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0));
            warn.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").color(C_YELLOW).size(14.0));
                    ui.label(
                        RichText::new("OnionShare inativo. Clique em Ativar na barra lateral.")
                            .color(C_YELLOW)
                            .size(12.0),
                    );
                });
            });
            ui.add_space(6.0);
        }

        // ── Zona de drop ──────────────────────────────────────────────────
        let border_color = if self.drag_hover { C_ACCENT } else { C_BORDER };
        let drop_frame = egui::Frame::none()
            .fill(C_PANEL2)
            .stroke(Stroke::new(
                if self.drag_hover { 2.5 } else { 1.0 },
                border_color,
            ))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::same(14.0));

        drop_frame.show(ui, |ui| {
            if shared_files.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(36.0);
                    ui.label(RichText::new("🗂").size(44.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Arraste arquivos aqui para compartilhar")
                            .size(15.0)
                            .color(C_TEXT2),
                    );
                    ui.label(
                        RichText::new("Criptografia XChaCha20-Poly1305 automática por chunk")
                            .size(11.0)
                            .color(C_DIM),
                    );
                    ui.add_space(36.0);
                });
            } else {
                let files_clone = shared_files.to_vec();
                for f in &files_clone {
                    let file_frame = egui::Frame::none()
                        .fill(C_PANEL)
                        .rounding(egui::Rounding::same(7.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .stroke(Stroke::new(1.0, C_BORDER));
                    file_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🔒").color(C_CYAN).size(15.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&f.name).color(C_TEXT).strong().size(12.5));
                                ui.label(
                                    RichText::new(format!(
                                        "{}   ⬇ {} download(s)",
                                        crate::gui::shared::SharedState::fmt_bytes(f.size),
                                        f.downloads,
                                    ))
                                    .color(C_DIM)
                                    .size(10.5),
                                );
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let rm = egui::Button::new(
                                        RichText::new("🗑 Remover").color(C_RED).size(11.0),
                                    )
                                    .fill(with_alpha(C_RED, 22))
                                    .stroke(Stroke::new(1.0, with_alpha(C_RED, 130)))
                                    .rounding(egui::Rounding::same(5.0));
                                    if ui.add(rm).clicked() {
                                        self.send(GuiControl::RemoveFile(f.file_id));
                                        self.set_status(format!("🗑 Removido: {}", f.name), C_TEXT2);
                                    }

                                    ui.add_space(6.0);

                                    let cp = egui::Button::new(
                                        RichText::new("📋 Copiar link").color(C_ACCENT).size(11.0),
                                    )
                                    .fill(with_alpha(C_ACCENT, 22))
                                    .stroke(Stroke::new(1.0, with_alpha(C_ACCENT, 130)))
                                    .rounding(egui::Rounding::same(5.0));
                                    if ui.add(cp).clicked() {
                                        ui.output_mut(|o| o.copied_text = f.link.clone());
                                        self.clipboard_msg = Some((
                                            format!(
                                                "📋 Copiado: {}",
                                                &f.link[..f.link.len().min(28)]
                                            ),
                                            Instant::now(),
                                        ));
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(5.0);
                }
            }
        });
    }

    // ── Download ──────────────────────────────────────────────────────────────
    fn draw_download(
        &mut self,
        ui: &mut egui::Ui,
        tor_active: bool,
        active_downloads: &[crate::gui::shared::DownloadState],
    ) {
        ui.label(
            RichText::new("📥 Baixar Arquivos")
                .size(17.0)
                .color(C_TEXT)
                .strong(),
        );
        ui.add_space(8.0);

        if !tor_active {
            let warn = egui::Frame::none()
                .fill(with_alpha(C_YELLOW, 18))
                .stroke(Stroke::new(1.0, with_alpha(C_YELLOW, 90)))
                .rounding(egui::Rounding::same(7.0))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0));
            warn.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").color(C_YELLOW).size(14.0));
                    ui.label(
                        RichText::new(
                            "OnionShare inativo. Ative para poder baixar arquivos via Tor.",
                        )
                        .color(C_YELLOW)
                        .size(12.0),
                    );
                });
            });
            ui.add_space(8.0);
        }

        card(ui, "Novo Download", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Link onion://").color(C_TEXT2).size(12.0));
                let link_input = egui::TextEdit::singleline(&mut self.download_link_input)
                    .hint_text("onion://...")
                    .min_size(Vec2::new(300.0, 30.0));
                ui.add(link_input);
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Salvar em: ").color(C_TEXT2).size(12.0));
                if let Some(ref d) = self.download_dir {
                    ui.label(RichText::new(d.to_string_lossy()).color(C_CYAN).size(11.0));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let folder_btn = egui::Button::new(RichText::new("📂 Alterar...").size(11.0))
                        .fill(C_PANEL2)
                        .stroke(Stroke::new(1.0, C_BORDER));
                    if ui.add(folder_btn).clicked() {
                        self.open_folder_dialog();
                    }
                });
            });
            ui.add_space(10.0);

            let btn = egui::Button::new(
                RichText::new("🚀 Iniciar Download")
                    .color(Color32::WHITE)
                    .size(13.0),
            )
            .fill(if tor_active { C_ACCENT } else { C_DIM })
            .rounding(egui::Rounding::same(6.0))
            .min_size(Vec2::new(160.0, 32.0));

            if ui.add_enabled(tor_active, btn).clicked() {
                if !self.download_link_input.is_empty() {
                    if let Some(ref d) = self.download_dir {
                        self.send(GuiControl::DownloadItem(
                            self.download_link_input.clone(),
                            d.clone(),
                        ));
                        self.set_status("Iniciando download...", C_GREEN);
                        self.download_link_input.clear();
                    } else {
                        self.set_status("Escolha uma pasta destino.", C_YELLOW);
                    }
                } else {
                    self.set_status("Cole um link onion:// primeiro.", C_YELLOW);
                }
            }
        });

        ui.add_space(12.0);

        card(
            ui,
            &format!("Downloads Ativos ({})", active_downloads.len()),
            |ui| {
                if active_downloads.is_empty() {
                    ui.label(
                        RichText::new("Nenhum download em andamento.")
                            .color(C_DIM)
                            .size(12.0),
                    );
                } else {
                    for dl in active_downloads {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("📥").size(14.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&dl.name).strong().color(C_TEXT).size(12.5));
                                if dl.is_done {
                                    if let Some(ref err) = dl.error {
                                        ui.label(
                                            RichText::new(format!("❌ {}", err))
                                                .color(C_RED)
                                                .size(10.5),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("✅ Concluído").color(C_GREEN).size(10.5),
                                        );
                                    }
                                } else {
                                    let speed_str = format!(
                                        "{}/s",
                                        crate::gui::shared::SharedState::fmt_bytes(
                                            dl.speed_bytes_per_sec
                                        )
                                    );
                                    let eta_str = dl
                                        .eta_seconds
                                        .map(|s| {
                                            format!(
                                                " • ETA: {}",
                                                crate::gui::shared::SharedState::fmt_duration(s)
                                            )
                                        })
                                        .unwrap_or_default();

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{:.1}% — {}{}",
                                                dl.progress * 100.0,
                                                dl.status,
                                                eta_str
                                            ))
                                            .color(C_CYAN)
                                            .size(10.5),
                                        );
                                        // Speed
                                        if dl.speed_bytes_per_sec > 0 {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(speed_str)
                                                            .color(C_YELLOW)
                                                            .size(10.5)
                                                            .strong(),
                                                    );
                                                },
                                            );
                                        }
                                    });

                                    ui.add(
                                        egui::ProgressBar::new(dl.progress)
                                            .desired_width(ui.available_width() - 20.0),
                                    );
                                }
                            });
                        });
                        ui.add_space(8.0);
                    }
                }
            },
        );
    }

    // ── Search (Lobby Global) ──────────────────────────────────────────────────
    fn draw_search(
        &mut self,
        ui: &mut egui::Ui,
        network_files: &[crate::tracker_proto::NetworkFile],
    ) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🔍 Buscar na Rede")
                    .size(17.0)
                    .color(C_TEXT)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = egui::Button::new(RichText::new("🔄 Atualizar Agora").size(12.0))
                    .fill(C_PANEL2)
                    .stroke(egui::Stroke::new(1.0, C_BORDER));
                if ui.add(btn).clicked() {
                    self.shared
                        .lock()
                        .unwrap()
                        .control_queue
                        .push(crate::gui::shared::GuiControl::RefreshTracker);
                }
            });
        });
        ui.add_space(8.0);

        ui.label(RichText::new("Estes são os arquivos compartilhados publicamente pelas pessoas conectadas ao onion-poc.").color(C_TEXT2).size(12.0));
        ui.add_space(8.0);

        // --- CONEXÃO WAN (ADICIONAR PEER) ---
        card(ui, "🔗 Conexão Direta (WAN / Amigos)", |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Seu Endereço Onion (compartilhe com amigos):")
                            .color(C_DIM)
                            .size(11.0),
                    );
                    let addr = self
                        .shared
                        .lock()
                        .unwrap()
                        .onion_addr
                        .clone()
                        .unwrap_or_else(|| "aguardando ativação...".to_string());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&addr).color(C_CYAN).monospace().size(11.0));
                        if ui.button("📋 Copiar").clicked() {
                            ui.output_mut(|o| o.copy_text = addr.clone());
                            // self.set_status se tivessemos um pra clipboard aqui mas o overlay já é suficiente
                        }
                    });
                });
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Adicionar endereço de um amigo para pareamento:")
                        .color(C_TEXT2)
                        .size(12.0),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.bootstrap_peer_input)
                            .hint_text("Ex: http://nomedoamigo.onion")
                            .min_size(Vec2::new(340.0, 26.0)),
                    );

                    let btn = egui::Button::new(RichText::new("🔌 Conectar").size(12.0).strong())
                        .fill(C_ACCENT)
                        .rounding(4.0);

                    if ui.add(btn).clicked() {
                        let peer = self.bootstrap_peer_input.clone();
                        if !peer.is_empty() {
                            self.send(GuiControl::AddBootstrapPeer(peer));
                            self.bootstrap_peer_input.clear();
                            // self.set_status("Pareamento iniciado! Aguarde alguns instantes...", C_ACCENT);
                        }
                    }
                });
                ui.add_space(2.0);
                ui.label(
                    RichText::new(
                        "Dica: Em até 60s o protocolo Gossip sincronizará os arquivos entre vocês.",
                    )
                    .size(10.0)
                    .color(C_DIM),
                );
            });
        });

        ui.add_space(14.0);

        card(ui, "🔍 Filtro de Pesquisa", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Palavra-chave: ").color(C_TEXT).size(12.5));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Ex: backup, foto...")
                        .min_size(Vec2::new(300.0, 26.0)),
                );
            });
        });

        ui.add_space(14.0);

        let query = self.search_query.to_lowercase();
        let filtered: Vec<_> = network_files
            .iter()
            .filter(|f| query.is_empty() || f.name.to_lowercase().contains(&query))
            .collect();

        if filtered.is_empty() {
            ui.label(
                RichText::new(
                    "Nenhum arquivo encontrado no Lobby (tente aguardar alguns instantes).",
                )
                .color(C_DIM)
                .size(13.0),
            );
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for f in filtered {
                    card(ui, &f.name, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("🧊").size(15.0));
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&f.name).color(C_TEXT).size(13.5).strong());
                                ui.label(
                                    RichText::new(format!(
                                        "Tamanho: {} • Peers: {}",
                                        crate::gui::shared::SharedState::fmt_bytes(f.size),
                                        f.peer_count
                                    ))
                                    .color(C_TEXT2)
                                    .size(11.0),
                                );
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let btn = egui::Button::new(
                                        RichText::new("📥 Baixar Agora")
                                            .color(Color32::WHITE)
                                            .size(11.5)
                                            .strong(),
                                    )
                                    .fill(C_GREEN)
                                    .rounding(4.0)
                                    .min_size(Vec2::new(100.0, 28.0));

                                    if ui
                                        .add(btn)
                                        .on_hover_text("Clique para iniciar o download via Swarm")
                                        .clicked()
                                    {
                                        self.view = View::Download;
                                        self.download_link_input = crate::link::SwarmLink {
                                            content_hash: f.content_hash.clone(),
                                        }
                                        .to_string();
                                        self.set_status(
                                            "Swarm link preparado para Download!",
                                            C_GREEN,
                                        );
                                    }
                                },
                            );
                        });
                    });
                    ui.add_space(5.0);
                }
            });
        }
    }

    // ── About ─────────────────────────────────────────────────────────────────
    fn draw_about(&self, ui: &mut egui::Ui) {
        card(ui, "ℹ️ Sobre o onion-poc", |ui| {
            ui.label(
                RichText::new("🧅 onion-poc v0.8.1")
                    .size(19.0)
                    .color(C_ACCENT)
                    .strong(),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Prova de Conceito — Trabalho de Conclusão de Curso")
                    .color(C_TEXT2)
                    .size(13.0),
            );
            ui.add_space(12.0);

            for feat in &[
                ("✓", "Tor Onion Service v3 integrado", C_GREEN),
                ("✓", "Criptografia XChaCha20-Poly1305 + BLAKE3", C_GREEN),
                ("✓", "Usuários online em tempo real", C_GREEN),
                ("✓", "Multi-arquivo simultâneo", C_GREEN),
                ("✓", "Drag & drop + diálogo nativo", C_GREEN),
                ("✓", "GUI nativa 100% Rust (egui/eframe)", C_GREEN),
                ("✓", "Testes unitários e de integração", C_GREEN),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(feat.0).color(feat.2).size(13.0));
                    ui.label(RichText::new(feat.1).color(C_TEXT).size(13.0));
                });
            }

            ui.add_space(12.0);
            ui.hyperlink_to(
                RichText::new("github.com/DJmesh/onion_poc")
                    .color(C_CYAN)
                    .size(13.0),
                "https://github.com/DJmesh/onion_poc",
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new("MIT License — Eduardo Prestes, 2024")
                    .color(C_DIM)
                    .size(11.0),
            );
        });
    }

    // ── Modal: Termos de Uso ──────────────────────────────────────────────────
    fn draw_terms(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(with_alpha(C_BG, 240)))
            .show(ctx, |_| {});

        egui::Window::new(
            RichText::new("🧅 onion-poc — Termos de Uso")
                .color(C_ACCENT)
                .strong()
                .size(15.0),
        )
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(580.0)
        .max_width(620.0)
        .show(ctx, |ui| {
            let scroll_output = egui::ScrollArea::vertical()
                .max_height(300.0)
                .id_salt("terms_scroll")
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(crate::wizard::app::App::TERMS_TEXT)
                            .size(12.0)
                            .color(C_TEXT2),
                    );
                });

            // Rastreia scroll
            self.terms_content_h = scroll_output.content_size.y;
            self.terms_viewport_h = scroll_output.inner_rect.height();
            self.terms_scroll_y = scroll_output.state.offset.y;

            let scrolled = self.terms_scroll_y
                >= (self.terms_content_h - self.terms_viewport_h - 5.0).max(0.0);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if !scrolled {
                    ui.label(
                        RichText::new("⬇ Role até o final para habilitar o botão")
                            .color(C_YELLOW)
                            .size(11.0),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let accept = egui::Button::new(
                        RichText::new("✓ Aceitar e continuar")
                            .color(Color32::WHITE)
                            .size(13.5)
                            .strong(),
                    )
                    .min_size(Vec2::new(185.0, 36.0))
                    .fill(if scrolled { C_GREEN } else { C_DIM })
                    .rounding(egui::Rounding::same(8.0));

                    if ui.add_enabled(scrolled, accept).clicked() {
                        self.config.terms_accepted = true;
                        let _ = self.config.save();
                        // Auto-start Tor após aceitar termos
                        self.send(GuiControl::StartTor);
                        self.show_tor_modal = true;
                        self.tor_modal_start = Some(Instant::now());
                    }

                    ui.add_space(8.0);
                    if ui.button(RichText::new("Sair").color(C_DIM)).clicked() {
                        std::process::exit(0);
                    }
                });
            });
        });
    }

    // ── Modal: Ativando Tor ────────────────────────────────────────────────────
    fn draw_tor_modal(&mut self, ctx: &egui::Context, state: &TorInitState) {
        egui::Window::new(
            RichText::new("🔄 Ativando OnionShare")
                .color(C_ACCENT)
                .strong(),
        )
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .min_width(440.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);

            match state {
                TorInitState::Installing { progress, message } => {
                    ui.label(
                        RichText::new("📦 Instalando Tor…")
                            .color(C_YELLOW)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(5.0);
                    ui.label(RichText::new(message).color(C_TEXT).size(12.0));
                    ui.add_space(8.0);
                    ui.add(
                        egui::ProgressBar::new(*progress)
                            .animate(true)
                            .show_percentage(),
                    );
                }
                TorInitState::Starting { progress, message } => {
                    ui.label(
                        RichText::new("🧅 Conectando à rede Tor…")
                            .color(C_CYAN)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(5.0);
                    ui.label(RichText::new(message).color(C_TEXT).size(12.0));
                    ui.add_space(8.0);
                    ui.add(
                        egui::ProgressBar::new(*progress)
                            .animate(true)
                            .show_percentage(),
                    );
                }
                _ => {
                    ui.label(
                        RichText::new("🔄 Inicializando…")
                            .color(C_ACCENT)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add(egui::ProgressBar::new(0.05).animate(true));
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);

            let elapsed = self
                .tor_modal_start
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            let hint = if elapsed < 20 {
                RichText::new(format!(
                    "⏱ {}s — Construindo circuito Tor anônimo…",
                    elapsed
                ))
                .color(C_DIM)
                .size(11.0)
            } else if elapsed < 60 {
                RichText::new(format!("⏱ {}s — Selecionando nós de saída…", elapsed))
                    .color(C_TEXT2)
                    .size(11.0)
            } else {
                RichText::new(format!(
                    "⏱ {}s ⚠ Demora maior que o normal — verifique sua conexão.",
                    elapsed
                ))
                .color(C_YELLOW)
                .size(11.0)
            };
            ui.label(hint);
            ui.label(
                RichText::new("O Tor normalmente leva entre 30 e 90 segundos.")
                    .color(C_DIM)
                    .size(10.5),
            );

            ui.add_space(6.0);
            ctx.request_repaint_after(Duration::from_secs(1));
        });
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Widget helpers *sem* premultiplied bugs
// ─────────────────────────────────────────────────────────────────────────────

/// Pequena badge colorida (texto + borda, fundo semitransparente)
fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .fill(with_alpha(color, 28))
        .stroke(Stroke::new(1.0, with_alpha(color, 180)))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(11.5).color(color).strong());
        });
}

/// Card de estatística (label + valor grande)
fn stat_card(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::none()
        .fill(C_PANEL2)
        .stroke(Stroke::new(1.0, C_BORDER))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.set_min_width(135.0);
            ui.label(RichText::new(label).size(10.5).color(C_DIM));
            ui.label(RichText::new(value).size(19.0).color(color).strong());
        });
}

/// Painel com título e conteúdo
fn card(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(C_PANEL2)
        .stroke(Stroke::new(1.0, C_BORDER))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(RichText::new(title).color(C_ACCENT).strong().size(12.5));
            ui.add(egui::Separator::default().spacing(6.0));
            ui.add_space(4.0);
            content(ui);
        });
}

/// Botão de navegação lateral — selecionado usa cor de texto ACCENT (não colore o fundo com a cor de acento)
fn nav_btn(ui: &mut egui::Ui, label: &str, selected: bool, view: &mut View, target: View) {
    let (fg, bg) = if selected {
        (C_ACCENT, Color32::from_rgb(28, 38, 68)) // fundo azul escuro, texto accent LEGÍVEL
    } else {
        (C_TEXT2, Color32::TRANSPARENT)
    };
    let btn = egui::Button::new(RichText::new(label).size(13.5).color(fg))
        .min_size(Vec2::new(160.0, 30.0))
        .fill(bg)
        .rounding(egui::Rounding::same(6.0));
    if ui.add(btn).clicked() {
        *view = target;
    }
}
