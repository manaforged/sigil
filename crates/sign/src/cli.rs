use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::value::{Error as ValueError, StrDeserializer};

use crate::files::{replace, write_new};
use crate::{
    AppBundle, CodeSign, Error, ErrorKind, ResourceCopy, macos_app, sign_file, verify_file,
    windows_dir,
};

const USAGE: &str = "usage: sigil-pack <macos|dmg|notarize|windows|codesign|sign|verify|genkey>";
const PASSWORD_ENV: &str = "SIGIL_PACK_PASSWORD";
const REFUSE_PASSWORD_FLAG: &str = "set SIGIL_PACK_PASSWORD; do not pass --password";

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PackCommand {
    Macos,
    Dmg,
    Notarize,
    Windows,
    Codesign,
    Sign,
    Verify,
    Genkey,
}

fn flag(args: &[String], name: &str) -> Result<Option<String>, Error> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if arg == name {
            return match rest.next() {
                Some(value) if !value.starts_with("--") => Ok(Some(value.clone())),
                _ => Err(Error::usage(format!("{name} requires a value"))),
            };
        }
    }
    Ok(None)
}

fn required(args: &[String], name: &str) -> Result<String, Error> {
    flag(args, name)?.ok_or_else(|| Error::usage(format!("missing {name}")))
}

fn resource_pairs(args: &[String]) -> Result<Vec<(PathBuf, PathBuf)>, Error> {
    let mut froms = Vec::new();
    let mut tos = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--resource-from" | "--resource-to" => {
                let value = rest
                    .next()
                    .filter(|value| !value.starts_with("--"))
                    .ok_or_else(|| Error::usage(format!("missing {arg} value")))?;
                let list = if arg == "--resource-from" {
                    &mut froms
                } else {
                    &mut tos
                };
                list.push(PathBuf::from(value));
            }
            "--resource" => {
                return Err(Error::usage(
                    "use --resource-from PATH --resource-to REL (colon form is not used)",
                ));
            }
            "--password" => return Err(Error::usage(REFUSE_PASSWORD_FLAG)),
            _ if arg.starts_with("--resource") => {
                return Err(Error::usage(format!(
                    "{arg} is not a flag; use separate --resource-from/--resource-to values"
                )));
            }
            _ => {}
        }
    }
    if froms.len() != tos.len() {
        return Err(Error::usage(
            "--resource-from and --resource-to counts must match",
        ));
    }
    Ok(froms.into_iter().zip(tos).collect())
}

fn resources(pairs: &[(PathBuf, PathBuf)]) -> Vec<ResourceCopy<'_>> {
    pairs
        .iter()
        .map(|(from, to)| ResourceCopy::new(from, to))
        .collect()
}

/// Runs one `sigil-pack` command from its arguments, without the program name.
pub fn pack_cli(argv: &[String]) -> Result<(), Error> {
    let (cmd, rest) = argv.split_first().ok_or_else(|| Error::usage(USAGE))?;
    let command = PackCommand::deserialize(StrDeserializer::<ValueError>::new(cmd))
        .ok()
        .ok_or_else(|| Error::usage(format!("unknown command: {cmd}")))?;
    if !matches!(command, PackCommand::Macos)
        && rest.iter().any(|arg| arg == "--provisioning-profile")
    {
        return Err(Error::usage(
            "--provisioning-profile is supported only by macos",
        ));
    }
    match command {
        PackCommand::Macos => pack_macos(rest),
        PackCommand::Dmg => pack_dmg(rest),
        PackCommand::Notarize => pack_notarize(rest),
        PackCommand::Windows => pack_windows(rest),
        PackCommand::Codesign => pack_codesign(rest),
        PackCommand::Sign => pack_sign(rest),
        PackCommand::Verify => pack_verify(rest),
        PackCommand::Genkey => pack_genkey(rest),
    }
}

fn codesign_flags(argv: &[String]) -> Result<[Option<String>; 2], Error> {
    let identity =
        flag(argv, "--identity")?.or_else(|| std::env::var("SIGIL_CODESIGN_IDENTITY").ok());
    Ok([identity, flag(argv, "--entitlements")?])
}

fn code_sign([identity, entitlements]: &[Option<String>; 2]) -> CodeSign<'_> {
    CodeSign::default()
        .identity(identity.as_deref().filter(|value| !value.is_empty()))
        .entitlements(entitlements.as_deref().map(Path::new))
}

fn pack_codesign(argv: &[String]) -> Result<(), Error> {
    let file = required(argv, "--file")?;
    code_sign(&codesign_flags(argv)?).sign(Path::new(&file))
}

fn pack_macos(argv: &[String]) -> Result<(), Error> {
    let exe = required(argv, "--exe")?;
    let out = required(argv, "--out")?;
    let name = required(argv, "--name")?;
    let ident = required(argv, "--id")?;
    let version = required(argv, "--version")?;
    let icon = flag(argv, "--icon")?;
    let provisioning_profile = flag(argv, "--provisioning-profile")?;
    let min = flag(argv, "--min")?;
    let exe_name = flag(argv, "--exe-name")?;
    let copies = resource_pairs(argv)?;
    let signing = codesign_flags(argv)?;
    let mut app = AppBundle::new(&name, Path::new(&exe))
        .identifier(&ident)
        .version(&version)
        .icon(icon.as_deref().map(Path::new))
        .provisioning_profile(provisioning_profile.as_deref().map(Path::new));
    if let Some(exe_name) = &exe_name {
        app = app.exe_name(exe_name);
    }
    if let Some(min) = &min {
        app = app.min_system(min);
    }
    macos_app(
        &app,
        &resources(&copies),
        Path::new(&out),
        &code_sign(&signing),
    )
}

#[cfg(target_os = "macos")]
fn pack_dmg(argv: &[String]) -> Result<(), Error> {
    let app = required(argv, "--app")?;
    let out = required(argv, "--out")?;
    let app_path = Path::new(&app);
    let volume = match flag(argv, "--volume")? {
        Some(volume) => volume,
        None => app_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .ok_or_else(|| Error::usage("--app has no file name"))?,
    };
    let background = flag(argv, "--background")?;
    let icon = flag(argv, "--icon")?;
    let layout = flag(argv, "--layout")?;
    let signing = codesign_flags(argv)?;
    let dmg = crate::Dmg::new(app_path, &volume)
        .background(background.as_deref().map(Path::new))
        .icon(icon.as_deref().map(Path::new))
        .layout(layout.as_deref().map(Path::new));
    crate::macos_dmg(&dmg, Path::new(&out), &code_sign(&signing))
}

#[cfg(target_os = "macos")]
fn pack_notarize(argv: &[String]) -> Result<(), Error> {
    let file = required(argv, "--file")?;
    let profile = flag(argv, "--profile")?
        .or_else(|| std::env::var("SIGIL_NOTARY_PROFILE").ok())
        .ok_or_else(|| Error::usage("missing --profile"))?;
    crate::notarize(Path::new(&file), &profile)
}

#[cfg(not(target_os = "macos"))]
fn pack_dmg(_argv: &[String]) -> Result<(), Error> {
    Err(Error::usage("dmg is supported only on macOS"))
}

#[cfg(not(target_os = "macos"))]
fn pack_notarize(_argv: &[String]) -> Result<(), Error> {
    Err(Error::usage("notarize is supported only on macOS"))
}

fn pack_windows(argv: &[String]) -> Result<(), Error> {
    let exe = required(argv, "--exe")?;
    let out = required(argv, "--out")?;
    let name = required(argv, "--name")?;
    let copies = resource_pairs(argv)?;
    let app = AppBundle::new(&name, Path::new(&exe));
    windows_dir(&app, &resources(&copies), Path::new(&out))
}

fn signing_password() -> Result<String, Error> {
    std::env::var(PASSWORD_ENV)
        .ok()
        .ok_or_else(|| Error::usage("set SIGIL_PACK_PASSWORD"))
}

fn generated_key_error(error: minisign::PError) -> Error {
    Error::caused(ErrorKind::Key, error)
}

fn write_error(path: &Path, error: io::Error) -> Error {
    Error::context(ErrorKind::Io, format!("write {}", path.display()), error)
}

fn pack_genkey(argv: &[String]) -> Result<(), Error> {
    refuse_password_flag(argv)?;
    let dir = match argv {
        [dir] => Path::new(dir),
        _ => return Err(Error::usage("usage: sigil-pack genkey DIR")),
    };
    let key_path = dir.join("e2e.key");
    let pub_path = dir.join("e2e.pub");
    if key_path.symlink_metadata().is_ok() || pub_path.symlink_metadata().is_ok() {
        return Err(Error::usage(format!(
            "{} already holds e2e.key/e2e.pub; refusing to overwrite",
            dir.display()
        )));
    }
    let pair = minisign::KeyPair::generate_encrypted_keypair(Some(signing_password()?))
        .map_err(generated_key_error)?;
    let secret = pair.sk.to_box(None).map_err(generated_key_error)?;
    let public = pair.pk.to_box().map_err(generated_key_error)?;
    write_new(&key_path, secret.into_string().as_bytes(), 0o600)
        .map_err(|error| write_error(&key_path, error))?;
    write_new(&pub_path, public.into_string().as_bytes(), 0o644)
        .map_err(|error| write_error(&pub_path, error))?;
    eprintln!("wrote {} and {}", key_path.display(), pub_path.display());
    Ok(())
}

fn refuse_password_flag(argv: &[String]) -> Result<(), Error> {
    if argv
        .iter()
        .any(|arg| arg == "--password" || arg.starts_with("--password="))
    {
        return Err(Error::usage(REFUSE_PASSWORD_FLAG));
    }
    Ok(())
}

fn pack_sign(argv: &[String]) -> Result<(), Error> {
    refuse_password_flag(argv)?;
    let key = required(argv, "--key")?;
    let file = required(argv, "--file")?;
    let version = required(argv, "--version")?;
    let password = std::env::var(PASSWORD_ENV).ok();
    let envelope = sign_file(
        Path::new(&file),
        Path::new(&key),
        password.as_deref(),
        &version,
    )?;
    let sig_path = PathBuf::from(format!("{file}.sig"));
    replace(&sig_path, envelope.as_bytes()).map_err(Error::io)
}

fn pack_verify(argv: &[String]) -> Result<(), Error> {
    let key = required(argv, "--pub")?;
    let file = required(argv, "--file")?;
    let sig = flag(argv, "--sig")?.unwrap_or_else(|| format!("{file}.sig"));
    let version = required(argv, "--version")?;
    verify_file(Path::new(&key), Path::new(&file), Path::new(&sig), &version)?;
    eprintln!("verified {file} version {version}");
    Ok(())
}
