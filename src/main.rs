mod app;
mod dicom;
mod help;
mod tree_widget;

use app::{App, AppParameter};
use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = AppParameter::parse();
    let mut terminal = ratatui::init();
    let app_result = App::new(args)?.run(&mut terminal);
    ratatui::restore();
    match app_result {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::format_err!("app error: {e}")),
    }
}
