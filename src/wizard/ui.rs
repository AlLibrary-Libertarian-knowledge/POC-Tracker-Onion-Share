use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, ListState, Padding, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs, Wrap,
    },
    Frame,
};

use crate::wizard::app::{App, Screen, Tab};

// ─────────────────────────────────────────────────────────────────────────────
// Paleta
// ─────────────────────────────────────────────────────────────────────────────
const C_BG: Color = Color::Rgb(10, 10, 18);
const C_PANEL: Color = Color::Rgb(20, 22, 34);
const C_BORDER: Color = Color::Rgb(60, 65, 100);
const C_ACCENT: Color = Color::Rgb(99, 179, 237);
const C_GREEN: Color = Color::Rgb(72, 199, 116);
const C_RED: Color = Color::Rgb(240, 80, 80);
const C_YELLOW: Color = Color::Rgb(240, 200, 80);
const C_TEXT: Color = Color::Rgb(220, 220, 235);
const C_DIM: Color = Color::Rgb(100, 105, 130);
const C_CYAN: Color = Color::Rgb(52, 211, 195);

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────
pub fn render(f: &mut Frame, app: &mut App) {
    f.render_widget(Block::default().style(Style::default().bg(C_BG)), f.area());

    match app.screen {
        Screen::Terms => render_terms(f, app),
        Screen::TorCheck => render_tor_check(f, app),
        Screen::TorInstalling => render_tor_installing(f, app),
        Screen::App => render_app(f, app),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ▌TERMS
// ─────────────────────────────────────────────────────────────────────────────
fn render_terms(f: &mut Frame, app: &mut App) {
    let area = centered_rect(90, 90, f.area());

    let outer = Block::default()
        .title(title_span(" 🧅 onion_poc — Termos de Uso "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_ACCENT))
        .style(Style::default().bg(C_PANEL));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(inner);

    // Texto dos termos
    let lines: Vec<Line> = App::TERMS_TEXT
        .lines()
        .map(|l| Line::from(Span::styled(l, Style::default().fg(C_TEXT))))
        .collect();

    let visible_height = chunks[0].height as usize;
    let max_scroll = lines.len().saturating_sub(visible_height);
    app.terms_max_scroll = max_scroll as u16;
    app.terms_scroll = app.terms_scroll.min(max_scroll as u16);

    let para = Paragraph::new(Text::from(lines))
        .scroll((app.terms_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::NONE)
                .padding(Padding::horizontal(2)),
        );
    f.render_widget(para, chunks[0]);

    // Barra de progresso de scroll
    let progress = if max_scroll == 0 {
        1.0
    } else {
        app.terms_scroll as f64 / max_scroll as f64
    };
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(C_ACCENT))
        .ratio(progress.min(1.0));
    f.render_widget(gauge, chunks[1]);

    // Footer
    let hint = if progress >= 1.0 {
        Line::from(vec![
            Span::styled("  Pressione ", Style::default().fg(C_DIM)),
            Span::styled(
                "[Y] ou [Enter]",
                Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" para aceitar e continuar   ", Style::default().fg(C_DIM)),
            Span::styled("[Q]", Style::default().fg(C_RED)),
            Span::styled(" para sair", Style::default().fg(C_DIM)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Role até o final: ", Style::default().fg(C_DIM)),
            Span::styled("[↓] [PgDn]", Style::default().fg(C_ACCENT)),
            Span::styled(" para rolar   ", Style::default().fg(C_DIM)),
            Span::styled("[Q]", Style::default().fg(C_RED)),
            Span::styled(" para sair", Style::default().fg(C_DIM)),
        ])
    };
    let footer = Paragraph::new(hint).alignment(Alignment::Center);
    f.render_widget(footer, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
// ▌TOR CHECK
// ─────────────────────────────────────────────────────────────────────────────
fn render_tor_check(f: &mut Frame, _app: &App) {
    let area = centered_rect(70, 60, f.area());

    let block = Block::default()
        .title(title_span(" ⚙️  Configuração — Tor "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_YELLOW))
        .style(Style::default().bg(C_PANEL));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ❌  Tor não foi encontrado no seu sistema.",
            Style::default().fg(C_RED).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  O Tor é necessário para criar a rede Onion anônima.",
            Style::default().fg(C_TEXT),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Pressione [Enter] para instalar automaticamente.",
            Style::default().fg(C_GREEN),
        )),
        Line::from(Span::styled(
            "  Pressione [R] para reverificar (se já instalou manualmente).",
            Style::default().fg(C_ACCENT),
        )),
        Line::from(Span::styled(
            "  Pressione [Q] para sair.",
            Style::default().fg(C_DIM),
        )),
        Line::from(""),
        det_platform_hint(),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn det_platform_hint() -> Line<'static> {
    #[cfg(target_os = "linux")]
    return Line::from(Span::styled(
        "  Linux detectado → será executado: sudo apt-get install tor",
        Style::default().fg(C_DIM),
    ));
    #[cfg(target_os = "macos")]
    return Line::from(Span::styled(
        "  macOS detectado → será executado: brew install tor",
        Style::default().fg(C_DIM),
    ));
    #[cfg(target_os = "windows")]
    return Line::from(Span::styled(
        "  Windows detectado → Tor Expert Bundle será baixado automaticamente.",
        Style::default().fg(C_DIM),
    ));
    #[allow(unreachable_code)]
    Line::from("")
}

// ─────────────────────────────────────────────────────────────────────────────
// ▌TOR INSTALLING
// ─────────────────────────────────────────────────────────────────────────────
fn render_tor_installing(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 50, f.area());

    let block = Block::default()
        .title(title_span(" 📦 Instalando Tor... "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_CYAN))
        .style(Style::default().bg(C_PANEL));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .margin(1)
        .split(inner);

    let msg = Paragraph::new(Line::from(Span::styled(
        format!("  {}", app.install_message),
        Style::default().fg(C_TEXT),
    )));
    f.render_widget(msg, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(C_CYAN).bg(C_BG))
        .ratio(app.install_progress.min(1.0))
        .label(format!("{:.0}%", app.install_progress * 100.0));
    f.render_widget(gauge, chunks[1]);

    let hint = Paragraph::new(Line::from(Span::styled(
        "  Aguarde...",
        Style::default().fg(C_DIM),
    )));
    f.render_widget(hint, chunks[2]);
}

// ─────────────────────────────────────────────────────────────────────────────
// ▌APP PRINCIPAL
// ─────────────────────────────────────────────────────────────────────────────
fn render_app(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Length(3), // tabs
            Constraint::Fill(1),   // content
            Constraint::Length(3), // footer
        ])
        .split(area);

    render_header(f, app, root[0]);
    render_tabs(f, app, root[1]);

    match app.active_tab {
        Tab::Dashboard => render_dashboard(f, app, root[2]),
        Tab::Files => render_files(f, app, root[2]),
        Tab::Search => render_search(f, app, root[2]),
        Tab::About => render_about(f, root[2]),
    }

    render_footer(f, app, root[3]);
}

// ─── Header ──────────────────────────────────────────────────────────────────
fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let (tor_label, tor_color) = if app.tor_active {
        ("● ATIVO ", C_GREEN)
    } else {
        ("○ INATIVO", C_RED)
    };

    let onion_short = app
        .onion_addr
        .as_deref()
        .map(|s| format!("  {}…", &s[..12.min(s.len())]))
        .unwrap_or_default();

    let line = Line::from(vec![
        Span::styled(
            "  🧅 onion_poc",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(onion_short, Style::default().fg(C_DIM)),
        Span::raw("   "),
        Span::styled("Tor: ", Style::default().fg(C_DIM)),
        Span::styled(
            tor_label,
            Style::default().fg(tor_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("     "),
        Span::styled("👥 ", Style::default().fg(C_DIM)),
        Span::styled(
            format!("{}", app.online_now),
            Style::default().fg(C_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" online     ⏱ ", Style::default().fg(C_DIM)),
        Span::styled(app.uptime_str(), Style::default().fg(C_TEXT)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_PANEL));

    let para = Paragraph::new(line).block(block);
    f.render_widget(para, area);
}

// ─── Tabs ─────────────────────────────────────────────────────────────────────
fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        format!(" 1 Dashboard ({}) ", app.shared_files.len()),
        format!(" 2 Arquivos ({}) ", app.shared_files.len()),
        " 3 Buscar ".to_string(),
        " 4 Sobre ".to_string(),
    ];
    let selected = match app.active_tab {
        Tab::Dashboard => 0,
        Tab::Files => 1,
        Tab::Search => 2,
        Tab::About => 3,
    };
    let tabs = Tabs::new(titles)
        .select(selected)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(C_BORDER))
                .style(Style::default().bg(C_PANEL)),
        )
        .highlight_style(Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD))
        .divider(Span::styled(" │ ", Style::default().fg(C_BORDER)));
    f.render_widget(tabs, area);
}

// ─── Dashboard ────────────────────────────────────────────────────────────────
fn render_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Fill(1)])
        .margin(1)
        .split(area);

    // ── Col esquerda ─────────────────────────────────────────────────────────
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Fill(1)])
        .split(cols[0]);

    // Status da rede
    let (status_label, status_color, toggle_hint) = if app.tor_active {
        ("● ATIVO", C_GREEN, "[T] Desativar OnionShare")
    } else {
        ("○ INATIVO", C_RED, "[T] Ativar OnionShare")
    };

    let onion_line = app
        .onion_addr
        .as_deref()
        .map(|a| format!("  {}", a))
        .unwrap_or_else(|| "  — não iniciado —".into());

    let net_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Status:  ", Style::default().fg(C_DIM)),
            Span::styled(
                status_label,
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Endereço Onion:",
            Style::default().fg(C_DIM),
        )),
        Line::from(Span::styled(&onion_line, Style::default().fg(C_CYAN))),
        Line::from(""),
        Line::from(Span::styled(
            format!("  [T] {}", toggle_hint),
            Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  🔒 Todos os arquivos são criptografados",
            Style::default().fg(C_DIM),
        )),
        Line::from(Span::styled(
            "     com XChaCha20-Poly1305 por chunk.",
            Style::default().fg(C_DIM),
        )),
    ];
    let net_block = panel(" 🌐 Status da Rede ", C_ACCENT);
    let net_inner = net_block.inner(left[0]);
    f.render_widget(net_block, left[0]);
    f.render_widget(Paragraph::new(net_lines), net_inner);

    // Arquivos compartilhados (mini-lista)
    let file_items: Vec<ListItem> = app
        .shared_files
        .iter()
        .map(|f| {
            ListItem::new(Line::from(vec![
                Span::styled("  🔒 ", Style::default().fg(C_CYAN)),
                Span::styled(&f.name, Style::default().fg(C_TEXT)),
                Span::styled(
                    format!("  {}", App::fmt_bytes(f.size)),
                    Style::default().fg(C_DIM),
                ),
            ]))
        })
        .collect();

    let files_block = panel(
        &format!(" 📤 Compartilhados ({}) ", app.shared_files.len()),
        C_BORDER,
    );
    let list = List::new(file_items).block(files_block);
    f.render_widget(list, left[1]);

    // ── Col direita: estatísticas ──────────────────────────────────────────
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Fill(1)])
        .split(cols[1]);

    let s_online = format!("{}", app.online_now);
    let s_sessions = format!("{}", app.total_sessions);
    let s_bytes = App::fmt_bytes(app.total_bytes);
    let s_chunks = format!("{}", app.chunks_served);
    let s_uptime = app.uptime_str();
    let stat_lines = vec![
        Line::from(""),
        stat_row("  👥  Online agora:", &s_online, C_CYAN),
        Line::from(""),
        stat_row("  📊  Sessões totais:", &s_sessions, C_ACCENT),
        Line::from(""),
        stat_row("  📦  Dados enviados:", &s_bytes, C_GREEN),
        Line::from(""),
        stat_row("  🧩  Chunks servidos:", &s_chunks, C_TEXT),
        Line::from(""),
        stat_row("  ⏱   Uptime:", &s_uptime, C_YELLOW),
    ];
    let stats_block = panel(" 📈 Estatísticas de Uso ", C_ACCENT);
    let stats_inner = stats_block.inner(right[0]);
    f.render_widget(stats_block, right[0]);
    f.render_widget(Paragraph::new(stat_lines), stats_inner);

    // Activity log mock
    let log_lines: Vec<ListItem> = {
        let mut items = Vec::new();
        if app.total_sessions > 0 {
            items.push(list_log(
                "✔",
                &format!("{} sessão(ões) registrada(s)", app.total_sessions),
                C_GREEN,
            ));
        }
        if app.chunks_served > 0 {
            items.push(list_log(
                "⬆",
                &format!("{} chunks enviados", app.chunks_served),
                C_CYAN,
            ));
        }
        if !app.shared_files.is_empty() {
            items.push(list_log(
                "🔒",
                &format!(
                    "{} arquivo(s) criptografado(s) ativo(s)",
                    app.shared_files.len()
                ),
                C_ACCENT,
            ));
        }
        if items.is_empty() {
            items.push(list_log(
                "·",
                "Sem atividade ainda. Ative o OnionShare.",
                C_DIM,
            ));
        }
        items
    };
    let log_block = panel(" 🗒  Atividade Recente ", C_BORDER);
    let log = List::new(log_lines).block(log_block);
    f.render_widget(log, right[1]);
}

// ─── Files tab ────────────────────────────────────────────────────────────────
fn render_files(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Fill(1)])
        .margin(1)
        .split(area);

    // Input de share
    let (input_border, cursor) = if app.share_input_mode {
        (C_GREEN, "▋")
    } else {
        (C_BORDER, "")
    };
    let input_text = format!("  {}{}", app.share_input, cursor);
    let input_hint = if let Some(e) = &app.share_error {
        Span::styled(format!("  ⚠ {}", e), Style::default().fg(C_RED))
    } else if app.share_input_mode {
        Span::styled(
            "  [Enter] Compartilhar  [Esc] Cancelar",
            Style::default().fg(C_DIM),
        )
    } else {
        Span::styled(
            "  [A] Adicionar arquivo para compartilhar",
            Style::default().fg(C_DIM),
        )
    };

    let input_block = Block::default()
        .title(title_span(" 📤 Compartilhar novo arquivo "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(input_border))
        .style(Style::default().bg(C_PANEL));
    let input_inner = input_block.inner(chunks[0]);
    f.render_widget(input_block, chunks[0]);

    let input_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(input_inner);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            input_text,
            Style::default().fg(C_TEXT),
        ))),
        input_layout[0],
    );
    f.render_widget(Paragraph::new(Line::from(input_hint)), input_layout[1]);

    // Lista de arquivos
    let files = app.shared_files.clone();
    let items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let is_sel = i == app.file_selected;
            let style = if is_sel {
                Style::default()
                    .fg(C_BG)
                    .bg(C_ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_TEXT)
            };
            ListItem::new(Line::from(vec![
                Span::styled(if is_sel { " ▶ " } else { "   " }, style),
                Span::styled("🔒 ", Style::default().fg(C_CYAN)),
                Span::styled(&f.name, style),
                Span::styled(
                    format!("  {}  ", App::fmt_bytes(f.size)),
                    Style::default().fg(C_DIM),
                ),
                Span::styled("⬇ ", Style::default().fg(C_DIM)),
                Span::styled(format!("{}", f.downloads), Style::default().fg(C_ACCENT)),
                Span::styled("  [C] Copiar link  [D] Remover", Style::default().fg(C_DIM)),
            ]))
        })
        .collect();

    let list_block = panel(
        &format!(
            " 📂 Arquivos disponíveis na rede ({}) — todos criptografados 🔒 ",
            files.len()
        ),
        C_ACCENT,
    );
    let mut list_state = ListState::default();
    if !files.is_empty() {
        list_state.select(Some(app.file_selected));
    }
    let list = List::new(items)
        .block(list_block)
        .highlight_style(Style::default().fg(C_BG).bg(C_ACCENT));
    f.render_stateful_widget(list, chunks[1], &mut list_state);
}

// ─── Search tab ───────────────────────────────────────────────────────────────
fn render_search(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Fill(1)])
        .margin(1)
        .split(area);

    // Search bar
    let (border_color, cursor) = if app.search_mode {
        (C_GREEN, "▋")
    } else {
        (C_BORDER, "")
    };
    let search_text = format!("  🔍  {}{}", app.search_query, cursor);
    let search_block = Block::default()
        .title(title_span(" Buscar arquivos compartilhados "))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(C_PANEL));
    let search_inner = search_block.inner(chunks[0]);
    f.render_widget(search_block, chunks[0]);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            search_text,
            Style::default().fg(C_TEXT),
        ))),
        search_inner,
    );

    // Results
    let filtered = app.filtered_files();
    let items: Vec<ListItem> = filtered
        .iter()
        .map(|f| {
            ListItem::new(Line::from(vec![
                Span::styled("   🔒 ", Style::default().fg(C_CYAN)),
                Span::styled(&f.name, Style::default().fg(C_TEXT)),
                Span::styled(
                    format!("  {}", App::fmt_bytes(f.size)),
                    Style::default().fg(C_DIM),
                ),
                Span::styled(format!("  ⬇ {}x", f.downloads), Style::default().fg(C_DIM)),
            ]))
        })
        .collect();

    let hint = if app.search_query.is_empty() {
        format!(
            " {} arquivo(s) disponíveis. Pressione [/] para buscar. ",
            app.shared_files.len()
        )
    } else {
        format!(" {} resultado(s) para \"{}\"", filtered.len(), app.search_query)
    };
    let list_block = panel(&hint, C_ACCENT);
    let list = List::new(items).block(list_block);
    f.render_widget(list, chunks[1]);
}

// ─── About tab ───────────────────────────────────────────────────────────────
fn render_about(f: &mut Frame, area: Rect) {
    let block = panel(" ℹ️  Sobre o onion_poc ", C_ACCENT);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  🧅  onion_poc v0.1.1",
            Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Prova de Conceito para TCC — Engenharia/Ciência da Computação",
            Style::default().fg(C_TEXT),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Funcionalidades:",
            Style::default().fg(C_DIM),
        )),
        Line::from(Span::styled(
            "  ✓  Compartilhamento de arquivos via Tor Onion Service v3",
            Style::default().fg(C_GREEN),
        )),
        Line::from(Span::styled(
            "  ✓  Criptografia por chunk: XChaCha20-Poly1305 + BLAKE3",
            Style::default().fg(C_GREEN),
        )),
        Line::from(Span::styled(
            "  ✓  Contagem de usuários online em tempo real",
            Style::default().fg(C_GREEN),
        )),
        Line::from(Span::styled(
            "  ✓  Multi-arquivo simultâneo",
            Style::default().fg(C_GREEN),
        )),
        Line::from(Span::styled(
            "  ✓  100% Rust — sem dependências Python",
            Style::default().fg(C_GREEN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  https://github.com/DJmesh/onion_poc",
            Style::default().fg(C_CYAN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  MIT License — Eduardo Prestes, 2024",
            Style::default().fg(C_DIM),
        )),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

// ─── Footer ───────────────────────────────────────────────────────────────────
fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let status = app
        .status_msg
        .as_ref()
        .map(|(s, _)| s.as_str())
        .unwrap_or("");

    let shortcuts = match app.active_tab {
        Tab::Dashboard | Tab::Files => {
            "[T] Toggle Tor  [A] Add  [D] Remove  [C] Copiar link  [/] Buscar  [Tab] Aba  [Q] Sair"
        }
        Tab::Search => "[/] Buscar  [Esc] Sair da busca  [Tab] Mudar aba  [Q] Sair",
        Tab::About => "[Tab] Mudar aba  [Q] Sair",
    };

    let line = if status.is_empty() {
        Line::from(Span::styled(
            format!("  {}", shortcuts),
            Style::default().fg(C_DIM),
        ))
    } else {
        Line::from(vec![
            Span::styled(format!("  {} ", status), Style::default().fg(C_CYAN)),
            Span::styled(format!("│ {}", shortcuts), Style::default().fg(C_DIM)),
        ])
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(C_BORDER))
        .style(Style::default().bg(C_PANEL));

    f.render_widget(Paragraph::new(line).block(block), area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]
}

fn title_span(title: &str) -> ratatui::widgets::block::Title {
    ratatui::widgets::block::Title::from(Span::styled(
        title,
        Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD),
    ))
}

fn panel(title: &str, border_color: Color) -> Block<'static> {
    Block::default()
        .title(Span::styled(
            title.to_string(),
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(C_PANEL))
}

fn stat_row<'a>(label: &'a str, value: &'a str, val_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, Style::default().fg(C_DIM)),
        Span::raw("  "),
        Span::styled(
            value,
            Style::default().fg(val_color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn list_log(icon: &str, msg: &str, color: Color) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
        Span::styled(msg.to_string(), Style::default().fg(C_TEXT)),
    ]))
}
