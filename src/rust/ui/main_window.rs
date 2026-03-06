use slint::{ComponentHandle, SharedString};
use crate::{MainWindow, PlayerComponentSettings};

pub fn ui_window()  -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    ui.set_player_component_settings(PlayerComponentSettings{
        title: SharedString::from("Player component")
    });
    ui.run()
}