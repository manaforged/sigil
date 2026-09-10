#![allow(clippy::indexing_slicing, clippy::panic, clippy::unwrap_used)]

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use super::{
    Bindings, ContextSet, Key, KeyCode, Keymap, KeymapDiagnostic, Modifier, Modifiers, NamedKey,
    ParseError, Resolution, Resolver,
};

#[test]
fn parse_plain_letter_lowercases() {
    let k = Key::parse("A").unwrap();
    assert_eq!(k.modifiers, Modifiers::empty());
    assert_eq!(k.code, KeyCode::Char('a'));
}

#[test]
fn parse_ctrl_letter() {
    let k = Key::parse("ctrl-s").unwrap();
    assert!(k.modifiers.contains(Modifier::Ctrl));
    assert!(!k.modifiers.contains(Modifier::Shift));
    assert_eq!(k.code, KeyCode::Char('s'));
}

#[test]
fn parse_all_four_modifiers_in_any_order() {
    let a = Key::parse("ctrl-shift-alt-meta-x").unwrap();
    let b = Key::parse("meta-alt-shift-ctrl-x").unwrap();
    assert_eq!(a, b);
    for m in [
        Modifier::Ctrl,
        Modifier::Shift,
        Modifier::Alt,
        Modifier::Meta,
    ] {
        assert!(a.modifiers.contains(m));
    }
}

#[test]
fn parse_modifier_aliases_canonicalise() {
    let a = Key::parse("control-cmd-opt-x").unwrap();
    let b = Key::parse("ctrl-meta-alt-x").unwrap();
    assert_eq!(a, b);
    let c = Key::parse("win-x").unwrap();
    let d = Key::parse("meta-x").unwrap();
    assert_eq!(c, d);
}

#[test]
fn parse_every_named_key() {
    let cases = [
        ("enter", NamedKey::Enter),
        ("tab", NamedKey::Tab),
        ("escape", NamedKey::Escape),
        ("space", NamedKey::Space),
        ("backspace", NamedKey::Backspace),
        ("delete", NamedKey::Delete),
        ("home", NamedKey::Home),
        ("end", NamedKey::End),
        ("pageup", NamedKey::PageUp),
        ("pagedown", NamedKey::PageDown),
        ("up", NamedKey::Up),
        ("down", NamedKey::Down),
        ("left", NamedKey::Left),
        ("right", NamedKey::Right),
    ];
    for (token, expect) in cases {
        let k = Key::parse(token).unwrap();
        assert_eq!(k.code, KeyCode::Named(expect), "for {token}");
    }
}

#[test]
fn parse_named_key_aliases() {
    assert_eq!(
        Key::parse("esc").unwrap().code,
        KeyCode::Named(NamedKey::Escape)
    );
    assert_eq!(
        Key::parse("return").unwrap().code,
        KeyCode::Named(NamedKey::Enter)
    );
    assert_eq!(
        Key::parse("pgup").unwrap().code,
        KeyCode::Named(NamedKey::PageUp)
    );
    assert_eq!(
        Key::parse("pgdn").unwrap().code,
        KeyCode::Named(NamedKey::PageDown)
    );
    assert_eq!(
        Key::parse("del").unwrap().code,
        KeyCode::Named(NamedKey::Delete)
    );
}

#[test]
fn parse_function_keys_in_range() {
    for n in 1..=24_u8 {
        let s = format!("f{n}");
        let k = Key::parse(&s).unwrap();
        assert_eq!(k.code, KeyCode::Function(n));
    }
    let k = Key::parse("ctrl-shift-f12").unwrap();
    assert!(k.modifiers.contains(Modifier::Ctrl));
    assert!(k.modifiers.contains(Modifier::Shift));
    assert_eq!(k.code, KeyCode::Function(12));
}

#[test]
fn parse_function_key_out_of_range() {
    let err = Key::parse("f25").unwrap_err();
    assert_eq!(err, ParseError::FunctionKeyOutOfRange { n: 25 });
}

#[test]
fn parse_symbols() {
    for sym in [',', '.', '/', ';', '\'', '[', ']', '\\', '=', '`'] {
        let s = sym.to_string();
        let k = Key::parse(&s).unwrap();
        assert_eq!(k.code, KeyCode::Symbol(sym), "for {sym}");
    }
}

#[test]
fn parse_digit() {
    for d in 0..=9_u8 {
        let s = d.to_string();
        let k = Key::parse(&s).unwrap();
        assert_eq!(k.code, KeyCode::Digit(d));
    }
}

#[test]
fn parse_minus_as_literal_when_alone() {
    let k = Key::parse("-").unwrap();
    assert_eq!(k.code, KeyCode::Symbol('-'));
}

#[test]
fn parse_empty_input() {
    assert_eq!(Key::parse("").unwrap_err(), ParseError::Empty);
    assert_eq!(Key::parse("   ").unwrap_err(), ParseError::Empty);
}

#[test]
fn parse_trailing_dash_is_no_key() {
    let err = Key::parse("ctrl-").unwrap_err();
    assert_eq!(err, ParseError::NoKey);
}

#[test]
fn parse_duplicate_modifier_rejected() {
    let err = Key::parse("ctrl-ctrl-a").unwrap_err();
    assert_eq!(
        err,
        ParseError::DuplicateModifier {
            modifier: Modifier::Ctrl
        }
    );
}

#[test]
fn parse_unknown_token_rejected() {
    let err = Key::parse("hyper-x").unwrap_err();
    assert_eq!(
        err,
        ParseError::UnknownToken {
            token: "hyper".to_string()
        }
    );
}

#[test]
fn display_round_trips_for_modifier_combos() {
    for token in [
        "a",
        "ctrl-s",
        "ctrl-shift-p",
        "f1",
        "enter",
        "ctrl-shift-alt-meta-x",
        ",",
        "/",
    ] {
        let k = Key::parse(token).unwrap();
        let rendered = k.to_string();
        let re = Key::parse(&rendered).unwrap();
        assert_eq!(k, re, "round-trip {token} -> {rendered}");
    }
}

fn keys(s: &str) -> Vec<Key> {
    s.split_whitespace()
        .map(|t| Key::parse(t).unwrap())
        .collect()
}

#[test]
fn resolver_single_key_to_action() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("ctrl-s"), "buffer.save", []);
    let ctx = ContextSet::new();
    let mut r = Resolver::new();
    let out = r.feed(&bindings, &ctx, Key::parse("ctrl-s").unwrap());
    assert_eq!(out, Resolution::Action(Cow::Borrowed("buffer.save")));
    assert_eq!(r.pending_len(), 0);
}

#[test]
fn resolver_chord_two_keys_to_action() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("g d"), "code.go_to_definition", []);
    let ctx = ContextSet::new();
    let mut r = Resolver::new();
    let first = r.feed(&bindings, &ctx, Key::parse("g").unwrap());
    assert_eq!(first, Resolution::Pending);
    assert_eq!(r.pending_len(), 1);
    let second = r.feed(&bindings, &ctx, Key::parse("d").unwrap());
    assert_eq!(
        second,
        Resolution::Action(Cow::Borrowed("code.go_to_definition"))
    );
    assert_eq!(r.pending_len(), 0);
}

#[test]
fn resolver_unbound_clears_pending() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("g d"), "code.go_to_definition", []);
    let ctx = ContextSet::new();
    let mut r = Resolver::new();
    r.feed(&bindings, &ctx, Key::parse("g").unwrap());
    let out = r.feed(&bindings, &ctx, Key::parse("z").unwrap());
    assert_eq!(out, Resolution::Unbound);
    assert_eq!(r.pending_len(), 0);
}

#[test]
fn resolver_context_mismatch_yields_unbound() {
    let mut bindings = Bindings::new();
    bindings.insert(
        keys("ctrl-s"),
        "buffer.save",
        [Cow::Borrowed("editor"), Cow::Borrowed("focused")],
    );
    let mut ctx = ContextSet::new();
    ctx.insert("editor");
    let mut r = Resolver::new();
    let out = r.feed(&bindings, &ctx, Key::parse("ctrl-s").unwrap());
    assert_eq!(out, Resolution::Unbound);
}

#[test]
fn resolver_user_override_wins() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("ctrl-s"), "buffer.save.default", []);
    bindings.insert(keys("ctrl-s"), "buffer.save.user", []);
    let ctx = ContextSet::new();
    let mut r = Resolver::new();
    let out = r.feed(&bindings, &ctx, Key::parse("ctrl-s").unwrap());
    assert_eq!(out, Resolution::Action(Cow::Borrowed("buffer.save.user")));
}

#[test]
fn resolver_clear_pending() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("g d"), "code.go_to_definition", []);
    let ctx = ContextSet::new();
    let mut r = Resolver::new();
    r.feed(&bindings, &ctx, Key::parse("g").unwrap());
    assert_eq!(r.pending_len(), 1);
    r.clear();
    assert_eq!(r.pending_len(), 0);
}

#[test]
fn resolver_resolve_sequence_one_shot() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("ctrl-c ctrl-v"), "buffer.paste_special", []);
    let ctx = ContextSet::new();
    let out = Resolver::resolve_sequence(&bindings, &ctx, &keys("ctrl-c ctrl-v"));
    assert_eq!(
        out,
        Resolution::Action(Cow::Borrowed("buffer.paste_special"))
    );
    let pending = Resolver::resolve_sequence(&bindings, &ctx, &keys("ctrl-c"));
    assert_eq!(pending, Resolution::Pending);
    let unbound = Resolver::resolve_sequence(&bindings, &ctx, &keys("ctrl-x"));
    assert_eq!(unbound, Resolution::Unbound);
}

#[test]
fn resolver_distinct_contexts_select_correct_binding() {
    let mut bindings = Bindings::new();
    bindings.insert(keys("ctrl-s"), "buffer.save", [Cow::Borrowed("editor")]);
    bindings.insert(keys("ctrl-s"), "modal.dismiss", [Cow::Borrowed("modal")]);
    let mut ctx = ContextSet::new();
    ctx.insert("editor");
    let mut r = Resolver::new();
    let out = r.feed(&bindings, &ctx, Key::parse("ctrl-s").unwrap());
    assert_eq!(out, Resolution::Action(Cow::Borrowed("buffer.save")));
    let mut ctx2 = ContextSet::new();
    ctx2.insert("modal");
    let mut r2 = Resolver::new();
    let out2 = r2.feed(&bindings, &ctx2, Key::parse("ctrl-s").unwrap());
    assert_eq!(out2, Resolution::Action(Cow::Borrowed("modal.dismiss")));
}

fn write_workspace_keymap(dir: &Path, body: &str) -> PathBuf {
    let cfg_dir = dir.join(".sigil-ide");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    let path = cfg_dir.join("keymap.toml");
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn config_defaults_only_when_no_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let mut defaults = Bindings::new();
    defaults.insert(keys("ctrl-s"), "buffer.save", []);
    let (keymap, diags) = Keymap::open(tmp.path(), defaults).unwrap();
    assert!(diags.is_empty(), "no TOMLs = no diags");
    assert_eq!(keymap.bindings().len(), 1);
    let res = Resolver::resolve_sequence(keymap.bindings(), &ContextSet::new(), &keys("ctrl-s"));
    assert_eq!(res, Resolution::Action(Cow::Borrowed("buffer.save")));
}

#[test]
fn config_workspace_overrides_default() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(tmp.path(), "[bindings]\n\"ctrl-s\" = \"buffer.save_as\"\n");
    let mut defaults = Bindings::new();
    defaults.insert(keys("ctrl-s"), "buffer.save", []);
    let (keymap, diags) = Keymap::open(tmp.path(), defaults).unwrap();
    assert!(diags.is_empty(), "valid TOML emits no diagnostics");
    let res = Resolver::resolve_sequence(keymap.bindings(), &ContextSet::new(), &keys("ctrl-s"));
    assert_eq!(res, Resolution::Action(Cow::Borrowed("buffer.save_as")));
}

#[test]
fn config_chord_binding_parses() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(
        tmp.path(),
        "[bindings]\n\"g d\" = \"code.go_to_definition\"\n\"ctrl-c ctrl-v\" = \"buffer.paste_special\"\n",
    );
    let (keymap, diags) = Keymap::open(tmp.path(), Bindings::new()).unwrap();
    assert!(diags.is_empty(), "valid TOML emits no diagnostics");
    let ctx = ContextSet::new();
    let r = Resolver::resolve_sequence(keymap.bindings(), &ctx, &keys("g d"));
    assert_eq!(
        r,
        Resolution::Action(Cow::Borrowed("code.go_to_definition"))
    );
    let r2 = Resolver::resolve_sequence(keymap.bindings(), &ctx, &keys("ctrl-c ctrl-v"));
    assert_eq!(
        r2,
        Resolution::Action(Cow::Borrowed("buffer.paste_special"))
    );
}

#[test]
fn config_invalid_key_emits_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(
        tmp.path(),
        "[bindings]\n\"ctrl-bogus-key\" = \"noop\"\n\"ctrl-s\" = \"buffer.save\"\n",
    );
    let (keymap, diags) = Keymap::open(tmp.path(), Bindings::new()).unwrap();
    assert!(
        diags.iter().any(
            |d| matches!(d, KeymapDiagnostic::InvalidKey { raw, .. } if raw == "ctrl-bogus-key")
        ),
        "got: {diags:?}"
    );
    let r = Resolver::resolve_sequence(keymap.bindings(), &ContextSet::new(), &keys("ctrl-s"));
    assert_eq!(r, Resolution::Action(Cow::Borrowed("buffer.save")));
}

#[test]
fn config_table_form_with_when() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(
        tmp.path(),
        "[bindings]\n\"ctrl-s\" = { action = \"buffer.save\", when = [\"editor\", \"focused\"] }\n",
    );
    let (keymap, diags) = Keymap::open(tmp.path(), Bindings::new()).unwrap();
    assert!(diags.is_empty(), "valid TOML emits no diagnostics");
    let bound = ContextSet::from_iter(["editor", "focused"]);
    let r = Resolver::resolve_sequence(keymap.bindings(), &bound, &keys("ctrl-s"));
    assert_eq!(r, Resolution::Action(Cow::Borrowed("buffer.save")));
    let partial = ContextSet::from_iter(["editor"]);
    let r2 = Resolver::resolve_sequence(keymap.bindings(), &partial, &keys("ctrl-s"));
    assert_eq!(r2, Resolution::Unbound);
}

#[test]
fn config_parse_error_does_not_fail_open() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(tmp.path(), "this is not valid toml [\n");
    let result = Keymap::open(tmp.path(), Bindings::new());
    assert!(result.is_ok(), "parse failures surface as diagnostics");
    let (_, diags) = result.unwrap();
    assert!(
        diags
            .iter()
            .any(|d| matches!(d, KeymapDiagnostic::ParseError { .. })),
        "expected ParseError diagnostic, got {diags:?}"
    );
}

#[test]
fn config_reload_picks_up_new_bindings() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_workspace_keymap(tmp.path(), "[bindings]\n\"ctrl-s\" = \"buffer.save\"\n");
    let (mut keymap, _) = Keymap::open(tmp.path(), Bindings::new()).unwrap();
    let r = Resolver::resolve_sequence(keymap.bindings(), &ContextSet::new(), &keys("ctrl-s"));
    assert_eq!(r, Resolution::Action(Cow::Borrowed("buffer.save")));
    std::fs::write(&path, "[bindings]\n\"ctrl-s\" = \"buffer.save_as\"\n").unwrap();
    keymap.reload().unwrap();
    let r2 = Resolver::resolve_sequence(keymap.bindings(), &ContextSet::new(), &keys("ctrl-s"));
    assert_eq!(r2, Resolution::Action(Cow::Borrowed("buffer.save_as")));
}

#[test]
fn config_invalid_value_emits_diagnostic() {
    let tmp = tempfile::tempdir().unwrap();
    write_workspace_keymap(tmp.path(), "[bindings]\n\"ctrl-s\" = 42\n");
    let (_, diags) = Keymap::open(tmp.path(), Bindings::new()).unwrap();
    assert!(
        diags
            .iter()
            .any(|d| matches!(d, KeymapDiagnostic::InvalidValue { raw, .. } if raw == "ctrl-s")),
        "got {diags:?}"
    );
}

#[test]
fn non_existent_user_config_dir_is_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let result = Keymap::open(tmp.path(), Bindings::new());
    assert!(result.is_ok(), "missing user dir must not fail open");
}
