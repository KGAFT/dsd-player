slint::include_modules!();


mod verbose;
mod ui;
#[tokio::main]
async fn main()  -> Result<(), slint::PlatformError>{
    ui::main_window::ui_window()
}
