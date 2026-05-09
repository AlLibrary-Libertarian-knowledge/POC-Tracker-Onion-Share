// Library surface for consumers (e.g. AlLibrary Tauri). GUI stacks behind `gui`.
pub mod config;
pub mod crypto;
pub mod link;
pub mod server;
pub mod share;
pub mod tor;

#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub mod wizard;

pub mod tracker_proto;
