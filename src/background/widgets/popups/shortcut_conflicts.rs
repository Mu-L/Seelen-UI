use seelen_core::state::{CssStyles, SluPopupConfig, SluPopupContent};
use tauri::Listener;

use crate::{
    error::Result,
    log_error,
    widgets::{popups::POPUPS_MANAGER, show_settings_at},
};

pub fn show_shortcut_conflict_popup() -> Result<()> {
    let mut popup_manager = POPUPS_MANAGER.lock();
    let popup_id = popup_manager.create(get_popup_config())?;

    let handle = popup_manager.get_window_handle(&popup_id).unwrap().clone();

    handle.once("open_settings_shortcuts", move |_| {
        log_error!(show_settings_at("/shortcuts"));
        log_error!(POPUPS_MANAGER.lock().close_popup(&popup_id));
    });

    Ok(())
}

fn get_popup_config() -> SluPopupConfig {
    SluPopupConfig {
        width: 380.0,
        height: 160.0,
        title: vec![SluPopupContent::Text {
            value: t!("shortcut.conflicts.title").to_string(),
            styles: None,
        }],
        content: vec![SluPopupContent::Text {
            value: t!("shortcut.conflicts.body").to_string(),
            styles: Some(CssStyles::new().add("textAlign", "center")),
        }],
        footer: vec![
            SluPopupContent::Button {
                inner: vec![SluPopupContent::Text {
                    value: t!("shortcut.conflicts.dismiss").to_string(),
                    styles: None,
                }],
                on_click: "exit".to_string(),
                styles: None,
            },
            SluPopupContent::Button {
                inner: vec![SluPopupContent::Text {
                    value: t!("shortcut.conflicts.review").to_string(),
                    styles: None,
                }],
                on_click: "open_settings_shortcuts".to_string(),
                styles: None,
            },
        ],
    }
}
