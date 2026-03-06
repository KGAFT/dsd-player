use slint::{ComponentHandle, SharedString};
use crate::{MainWindow, PlayerComponentSettings, TrackInfo};
use crate::assets::Assets;

pub fn ui_window() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    ui.set_player_component_settings(PlayerComponentSettings{
        track_info: TrackInfo{
            album: SharedString::from("album"),
            artist: SharedString::from("artist"),
            picture:  slint::Image::load_from_svg_data(Assets::get("track.svg").unwrap().data.as_slice()).unwrap(),
            title: SharedString::from("title"),
        }
    });
    ui.run()
}