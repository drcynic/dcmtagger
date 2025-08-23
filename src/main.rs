use clap::Parser;

mod app;
mod dicom;

use app::App;

#[derive(Clone, Debug, Parser)]
#[clap(name = "DICOM Tagger", version = format!("v{}", env!("CARGO_PKG_VERSION")))]
#[clap(about = "Copyright (c) 2025 Daniel Szymanski")]
struct Args {
    #[clap(value_parser)]
    input_file: String,
}

fn main() -> anyhow::Result<()> {
    // let args = Args::parse();
    // let input_file = args.input_file;
    let input_file = "testdata/test.dcm".to_string();
    let mut terminal = ratatui::init();
    let app_result = App::new(&input_file)?.run(&mut terminal);
    ratatui::restore();
    match app_result {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::format_err!("app error: {e}")),
    }
}
