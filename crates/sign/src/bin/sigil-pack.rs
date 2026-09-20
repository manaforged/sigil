use std::env::args;
use std::process::ExitCode;

use sigil_sign::pack_cli;

fn main() -> ExitCode {
    let argv: Vec<String> = args().skip(1).collect();
    if argv.first().is_some_and(|c| c == "--help" || c == "help") {
        eprintln!(
            "usage:\n  sigil-pack macos --exe PATH --out PATH.app --name NAME --id ID --version VER [--icon PATH] [--min VER] [--resource SRC:DST]...\n  sigil-pack windows --exe PATH --out DIR --name EXE [--resource SRC:DST]...\n  sigil-pack sign --key PATH --password PW --file PATH"
        );
        return ExitCode::SUCCESS;
    }
    match pack_cli(&argv) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
