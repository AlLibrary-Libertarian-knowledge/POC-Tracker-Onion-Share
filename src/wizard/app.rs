/// wizard/app.rs — mantido apenas para expor TERMS_TEXT à GUI.
/// O resto da lógica TUI foi removido em favor de src/gui/.
pub struct App;

impl App {
    pub const TERMS_TEXT: &'static str = include_str!("../../TERMS.txt");
}
