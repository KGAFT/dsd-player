#![feature(str_as_str)]

slint::include_modules!();


mod verbose;
mod ui;
pub mod assets;

#[tokio::main]
async fn main()  -> Result<(), slint::PlatformError>{
    ui::main_window::ui_window()
}
