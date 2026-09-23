use std::env::args;
use std::process::ExitCode;

use sigil_sign::pack_cli;

fn main() -> ExitCode {
    let argv: Vec<String> = args().skip(1).collect();
    if argv.first().is_some_and(|c| c == "--help" || c == "help") {
        eprintln!(
            "usage:\n  sigil-pack macos --exe PATH --out PATH.app --name NAME --id ID --version VER [--icon PATH] [--min VER] [--identity NAME] [--entitlements PATH] [--resource-from PATH --resource-to REL]...\n  sigil-pack windows --exe PATH --out DIR --name EXE [--resource-from PATH --resource-to REL]...\n  sigil-pack sign --key PATH --file PATH\n  genkey DIR\n  sign password: SIGIL_PACK_PASSWORD\n  .sig is base64 minisign.\n  macos codesign identity: --identity or SIGIL_CODESIGN_IDENTITY; without one the bundle is ad-hoc signed and Gatekeeper refuses a downloaded copy."
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
