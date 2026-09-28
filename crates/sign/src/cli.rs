use std::path::{Path, PathBuf};

use crate::files::{replace, write_new};
use crate::{AppBundle, CodeSign, ResourceCopy, macos_app, sign_file, verify_file, windows_dir};

fn flag(args: &[String], name: &str) -> Result<Option<String>, String> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == name {
            return match rest.next() {
                Some(value) if value.starts_with("--") => Err(format!("{name} requires a value")),
                Some(value) => Ok(Some(value.clone())),
                None => Ok(None),
            };
        }
    }
    Ok(None)
}

fn resource_pairs(args: &[String]) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut froms = Vec::new();
    let mut tos = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--resource-from" => {
                let value = rest
                    .next()
                    .filter(|value| !value.starts_with("--"))
                    .ok_or("missing --resource-from value")?;
                froms.push(PathBuf::from(value));
            }
            "--resource-to" => {
                let value = rest
                    .next()
                    .filter(|value| !value.starts_with("--"))
                    .ok_or("missing --resource-to value")?;
                tos.push(PathBuf::from(value));
            }
            "--resource" => {
                return Err(
                    "use --resource-from PATH --resource-to REL (colon form is not used)".into(),
                );
            }
            "--password" => {
                return Err("set SIGIL_PACK_PASSWORD; do not pass --password".into());
            }
            _ if arg.starts_with("--resource") => {
                return Err(format!(
                    "{arg} is not a flag; use separate --resource-from/--resource-to values"
                ));
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

pub fn pack_cli(argv: &[String]) -> Result<(), String> {
    let (cmd, rest) = argv
        .split_first()
        .ok_or("usage: sigil-pack <macos|windows|sign|verify|genkey>")?;
    match cmd.as_str() {
        "macos" => pack_macos(rest),
        "windows" => pack_windows(rest),
        "sign" => pack_sign(rest),
        "verify" => pack_verify(rest),
        "genkey" => pack_genkey(rest),
        other => Err(format!("unknown command: {other}")),
    }
}

fn pack_macos(argv: &[String]) -> Result<(), String> {
    let exe = flag(argv, "--exe")?.ok_or("missing --exe")?;
    let out = flag(argv, "--out")?.ok_or("missing --out")?;
    let name = flag(argv, "--name")?.ok_or("missing --name")?;
    let ident = flag(argv, "--id")?.ok_or("missing --id")?;
    let version = flag(argv, "--version")?.ok_or("missing --version")?;
    let icon = flag(argv, "--icon")?;
    let min = flag(argv, "--min")?.unwrap_or_else(|| "10.15".into());
    let exe_name = flag(argv, "--exe-name")?.unwrap_or_else(|| name.clone());
    let copies = resource_pairs(argv)?;
    let identity =
        flag(argv, "--identity")?.or_else(|| std::env::var("SIGIL_CODESIGN_IDENTITY").ok());
    let entitlements = flag(argv, "--entitlements")?;
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
    let exe = flag(argv, "--exe")?.ok_or("missing --exe")?;
    let out = flag(argv, "--out")?.ok_or("missing --out")?;
    let name = flag(argv, "--name")?.ok_or("missing --name")?;
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

fn pack_genkey(argv: &[String]) -> Result<(), String> {
    refuse_password_flag(argv)?;
    let dir = match argv {
        [dir] => Path::new(dir),
        _ => return Err("usage: sigil-pack genkey DIR".into()),
    };
    let key_path = dir.join("e2e.key");
    let pub_path = dir.join("e2e.pub");
    if key_path.symlink_metadata().is_ok() || pub_path.symlink_metadata().is_ok() {
        return Err(format!(
            "{} already holds e2e.key/e2e.pub; refusing to overwrite",
            dir.display()
        ));
    }
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some(signing_password()?))
        .map_err(|e| e.to_string())?;
    let secret = pair.sk.to_box(None).map_err(|e| e.to_string())?;
    let public = pair.pk.to_box().map_err(|e| e.to_string())?;
    write_new(&key_path, secret.into_string().as_bytes(), 0o600)
        .map_err(|e| format!("write {}: {e}", key_path.display()))?;
    write_new(&pub_path, public.into_string().as_bytes(), 0o644)
        .map_err(|e| format!("write {}: {e}", pub_path.display()))?;
    eprintln!("wrote {} and {}", key_path.display(), pub_path.display());
    Ok(())
}

fn refuse_password_flag(argv: &[String]) -> Result<(), String> {
    if argv
        .iter()
        .any(|arg| arg == "--password" || arg.starts_with("--password="))
    {
        return Err("set SIGIL_PACK_PASSWORD; do not pass --password".into());
    }
    Ok(())
}

fn pack_sign(argv: &[String]) -> Result<(), String> {
    refuse_password_flag(argv)?;
    let key = flag(argv, "--key")?.ok_or("missing --key")?;
    let file = flag(argv, "--file")?.ok_or("missing --file")?;
    let version = flag(argv, "--version")?.ok_or("missing --version")?;
    let password = std::env::var("SIGIL_PACK_PASSWORD").ok();
    let envelope = sign_file(
        Path::new(&file),
        Path::new(&key),
        password.as_deref(),
        &version,
    )
    .map_err(|e| e.to_string())?;
    let sig_path = PathBuf::from(format!("{file}.sig"));
    replace(&sig_path, envelope.as_bytes()).map_err(|e| e.to_string())
}

fn pack_verify(argv: &[String]) -> Result<(), String> {
    let key = flag(argv, "--pub")?.ok_or("missing --pub")?;
    let file = flag(argv, "--file")?.ok_or("missing --file")?;
    let sig = flag(argv, "--sig")?.unwrap_or_else(|| format!("{file}.sig"));
    let version = flag(argv, "--version")?.ok_or("missing --version")?;
    verify_file(Path::new(&key), Path::new(&file), Path::new(&sig), &version)
        .map_err(|e| e.to_string())?;
    eprintln!("verified {file} version {version}");
    Ok(())
}
