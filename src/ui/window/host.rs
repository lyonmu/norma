use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use gpui::{App, AppContext, Application};

use crate::app::NormaAppState;
use crate::config::{AppConfig, NormaConfig};
use crate::runtime::RuntimeUpdate;
use crate::ui::settings::SettingsWindow;
use crate::ui::shell::AppShell;

use super::WindowPolicy;

pub fn open_settings_window(
    cx: &mut App,
    config: AppConfig,
    runtime_config: Option<NormaConfig>,
    config_file: Option<PathBuf>,
) {
    let options = WindowPolicy::settings_window_options();
    if let Err(error) = cx.open_window(options, |_, cx| {
        cx.new(|_| SettingsWindow::new(config, runtime_config, config_file))
    }) {
        tracing::error!(
            component = "ui",
            error = %error,
            "failed to open Norma settings window"
        );
    }
}

pub fn run(state: NormaAppState, updates: Receiver<RuntimeUpdate>) {
    let window_config = state
        .runtime_config
        .as_ref()
        .map(|config| config.window.clone());

    Application::new().run(move |cx: &mut App| {
        let options = WindowPolicy::main_window_options(window_config.as_ref());
        if let Err(error) =
            cx.open_window(options, |_, cx| cx.new(|_| AppShell::new(state, updates)))
        {
            tracing::error!(
                component = "ui",
                error = %error,
                "failed to open Norma window"
            );
        }
    });
}
