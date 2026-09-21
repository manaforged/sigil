use std::path::{Path, PathBuf};

use crate::{AppBundle, CodeSign, ResourceCopy, macos_app, sign_file, windows_dir};

fn flag(args: &[String], name: &str) -> Option<String> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == name {
            return rest.next().cloned();
        }
    }
    None
}

fn resource_pairs(args: &[String]) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut froms = Vec::new();
    let mut tos = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--resource-from" => {
                froms.push(PathBuf::from(
                    rest.next().ok_or("missing --resource-from value")?,
                ));
            }
            "--resource-to" => {
                tos.push(PathBuf::from(
                    rest.next().ok_or("missing --resource-to value")?,
                ));
            }
            "--resource" => {
                return Err(
                    "use --resource-from PATH --resource-to REL (colon form is not used)".into(),
                );
            }
            "--password" => {
                return Err("set SIGIL_PACK_PASSWORD; do not pass --password".into());
            }
            _ => {}
        }
    }
    if froms.len() != tos.len() {
        return Err("--resource-from and --resource-to counts must match".into());
    }
    Ok(froms.into_iter().zip(tos).collect())
}

fn resources(pairs: &[(PathBuf, PathBuf)]) -> Vec<ResourceCopy<'_>> {
    pairs
        .iter()
        .map(|(from, to)| ResourceCopy {
            from: from.as_path(),
            to: to.as_path(),
        })
        .collect()
}

/// Run `sigil-pack` from already-parsed argv (no program name).
///
/// Commands: `macos`, `windows`, `sign`.
/// Sign password is `SIGIL_PACK_PASSWORD`. The on-disk `.sig` is a base64
/// minisign envelope.
///
/// # Errors
///
/// Returns a user-facing message when arguments are missing or a pack step
/// fails.
pub fn pack_cli(argv: &[String]) -> Result<(), String> {
    let (cmd, rest) = argv
        .split_first()
        .ok_or("usage: sigil-pack <macos|windows|sign>")?;
    match cmd.as_str() {
        "macos" => pack_macos(rest),
        "windows" => pack_windows(rest),
        "sign" => pack_sign(rest),
        other => Err(format!("unknown command: {other}")),
    }
}

fn pack_macos(argv: &[String]) -> Result<(), String> {
    let exe = flag(argv, "--exe").ok_or("missing --exe")?;
    let out = flag(argv, "--out").ok_or("missing --out")?;
    let name = flag(argv, "--name").ok_or("missing --name")?;
    let ident = flag(argv, "--id").ok_or("missing --id")?;
    let version = flag(argv, "--version").ok_or("missing --version")?;
    let icon = flag(argv, "--icon");
    let min = flag(argv, "--min").unwrap_or_else(|| "10.15".into());
    let exe_name = flag(argv, "--exe-name").unwrap_or_else(|| name.clone());
    let copies = resource_pairs(argv)?;
    let identity =
        flag(argv, "--identity").or_else(|| std::env::var("SIGIL_CODESIGN_IDENTITY").ok());
    let entitlements = flag(argv, "--entitlements");
    let sign = CodeSign {
        identity: identity.as_deref().filter(|value| !value.is_empty()),
        entitlements: entitlements.as_deref().map(Path::new),
    };
    let icon_path = icon.as_deref().map(Path::new);
    let app = AppBundle {
        name: &name,
        identifier: &ident,
        version: &version,
        executable: Path::new(&exe),
        exe_name: &exe_name,
        icon: icon_path,
        min_system: &min,
    };
    macos_app(&app, &resources(&copies), Path::new(&out), &sign).map_err(|e| e.to_string())
}

fn pack_windows(argv: &[String]) -> Result<(), String> {
    let exe = flag(argv, "--exe").ok_or("missing --exe")?;
    let out = flag(argv, "--out").ok_or("missing --out")?;
    let name = flag(argv, "--name").ok_or("missing --name")?;
    let copies = resource_pairs(argv)?;
    let app = AppBundle {
        name: &name,
        identifier: "",
        version: "",
        executable: Path::new(&exe),
        exe_name: &name,
        icon: None,
        min_system: "",
    };
    windows_dir(&app, &resources(&copies), Path::new(&out)).map_err(|e| e.to_string())
}

fn signing_password() -> Result<String, String> {
    std::env::var("SIGIL_PACK_PASSWORD").map_err(|_| "set SIGIL_PACK_PASSWORD".to_string())
}

fn pack_sign(argv: &[String]) -> Result<(), String> {
    let key = flag(argv, "--key").ok_or("missing --key")?;
    let file = flag(argv, "--file").ok_or("missing --file")?;
    let password = signing_password()?;
    let envelope =
        sign_file(Path::new(&file), Path::new(&key), &password).map_err(|e| e.to_string())?;
    let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, envelope);
    std::fs::write(format!("{file}.sig"), encoded).map_err(|e| e.to_string())
}
