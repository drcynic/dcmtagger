use clap::Parser;

mod app;
mod dicom;
mod tree_widget;

use app::App;

#[derive(Clone, Debug, Parser)]
#[clap(name = "DICOM Tagger", version = format!("v{}", env!("CARGO_PKG_VERSION")))]
#[clap(about = "Copyright (c) 2025 Daniel Szymanski")]
struct Args {
    #[clap(value_parser)]
    input_path: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input_path = args.input_path;

    let mut terminal = ratatui::init();
    let app_result = App::new(&input_path)?.run(&mut terminal);
    ratatui::restore();
    match app_result {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::format_err!("app error: {e}")),
    }
}
