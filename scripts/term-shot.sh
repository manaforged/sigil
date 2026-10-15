#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: scripts/term-shot.sh --out PNG [--app Terminal|Ghostty|iTerm] [--size COLSxROWS] [--font NAME] [--font-size POINTS] [--env KEY=VALUE]... [--keys CHORDS] [--wait SECONDS] -- COMMAND [ARGS...]" >&2
  exit 2
}

app=Terminal
size=120x36
wait=4
out=
font=
font_size=
keys=
exports=
while [ $# -gt 0 ]; do
  case "$1" in
    --app) app=$2; shift 2 ;;
    --size) size=$2; shift 2 ;;
    --wait) wait=$2; shift 2 ;;
    --out) out=$2; shift 2 ;;
    --font) font=$2; shift 2 ;;
    --font-size) font_size=$2; shift 2 ;;
    --env) exports="${exports}export $(printf '%q' "$2")"$'\n'; shift 2 ;;
    --keys) keys=$2; shift 2 ;;
    --) shift; break ;;
    *) usage ;;
  esac
done
[ -n "$out" ] && [ $# -gt 0 ] || usage
[ "$(uname)" = Darwin ] || { echo "term-shot: macOS only" >&2; exit 2; }
case "$app" in
  Terminal) owner=Terminal ;;
  Ghostty) owner=Ghostty ;;
  iTerm) owner=iTerm2 ;;
  *) echo "term-shot: --app is Terminal, Ghostty, or iTerm" >&2; exit 2 ;;
esac
cols=${size%x*}
rows=${size#*x}
profile="term-shot-$$"
work=$(mktemp -d "${TMPDIR:-/tmp}/term-shot.XXXXXX")

finish() {
  if [ -f "$work/pid" ]; then
    pid=$(cat "$work/pid")
    pkill -TERM -P "$pid" 2>/dev/null || true
    kill -TERM "$pid" 2>/dev/null || true
  fi
  if [ "$app" = Terminal ] && [ -s "$work/tty" ]; then
    sleep 0.5
    osascript -e 'on run {xTty}' -e 'tell application "Terminal"' -e 'repeat with w in windows' \
      -e 'if tty of selected tab of w is xTty then close w saving no' -e 'end repeat' -e 'end tell' -e 'end run' \
      "$(cat "$work/tty")" >/dev/null 2>&1 || true
  fi
  if [ "$app" = Terminal ] && [ -n "$font$font_size" ]; then
    osascript -e 'on run {xLabel}' -e 'tell application "Terminal" to delete settings set xLabel' -e 'end run' \
      "$profile" >/dev/null 2>&1 || true
  fi
  rm -f "$work/run.command" "$work/windows.swift" "$work/pid" "$work/tty"
  rmdir "$work" 2>/dev/null || true
}
trap finish EXIT

command=$(printf '%q ' "$@")
cat > "$work/run.command" <<EOF
#!/bin/sh
printf '\033[8;${rows};${cols}t'
echo \$\$ > "$work/pid"
tty > "$work/tty"
sleep 1
printf '\033[8;${rows};${cols}t'
${exports}cd $(printf '%q' "$PWD")
exec $command
EOF
chmod +x "$work/run.command"

cat > "$work/windows.swift" <<'EOF'
import CoreGraphics
let owner = CommandLine.arguments[1]
let options = CGWindowListOption(arrayLiteral: .optionOnScreenOnly, .excludeDesktopElements)
let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] ?? []
for window in windows where window[kCGWindowOwnerName as String] as? String == owner
  && window[kCGWindowLayer as String] as? Int == 0 {
  print(window[kCGWindowNumber as String] ?? "")
}
EOF
windows() { swift "$work/windows.swift" "$owner" 2>/dev/null | sort; }

terminal_profile() {
  [ -n "$font$font_size" ] || return 0
  for _ in 1 2 3 4 5 6 7 8 9 10; do [ -s "$work/tty" ] && break; sleep 0.3; done
  [ -s "$work/tty" ] || return 0
  osascript -e 'on run {xLabel, xTty, xFace, xSize}' -e 'tell application "Terminal"' \
    -e 'set custom to make new settings set with properties {name:xLabel}' \
    -e 'if xFace is not "" then set font name of custom to xFace' \
    -e 'if xSize is not "" then set font size of custom to (xSize as number)' \
    -e 'repeat with w in windows' \
    -e 'if tty of selected tab of w is xTty then set current settings of selected tab of w to custom' \
    -e 'end repeat' -e 'end tell' -e 'end run' \
    "$profile" "$(cat "$work/tty")" "$font" "$font_size" >/dev/null
}

before=$(windows)
case "$app" in
  Terminal)
    open -a Terminal "$work/run.command"
    terminal_profile
    ;;
  Ghostty)
    extra=()
    [ -n "$font" ] && extra+=("--font-family=$font")
    [ -n "$font_size" ] && extra+=("--font-size=$font_size")
    open -na Ghostty --args ${extra[@]+"${extra[@]}"} -e "$work/run.command"
    ;;
  iTerm)
    [ -z "$font$font_size" ] || echo "term-shot: iTerm uses its default profile font" >&2
    osascript -e 'on run {xScript}' -e 'tell application "iTerm" to create window with default profile command xScript' \
      -e 'end run' "$work/run.command" >/dev/null
    ;;
esac
sleep "$wait"
window=$(comm -13 <(printf '%s\n' "$before") <(windows) | head -1)
[ -n "$window" ] || { echo "term-shot: no new $app window" >&2; exit 1; }

bridge="$HOME/Library/Application Support/Peekaboo/bridge.sock"
if [ -n "$keys" ]; then
  read -r -a chords <<< "$keys"
  peekaboo press "${chords[@]}" --app "$owner" --window-id "$window" --bridge-socket "$bridge" >/dev/null
  sleep 1.5
fi
if screencapture -x -o -l "$window" "$out" 2>/dev/null && [ -s "$out" ]; then
  :
elif command -v peekaboo >/dev/null 2>&1 && [ -S "$bridge" ]; then
  peekaboo see --no-elements --app "$owner" --window-id "$window" --path "$out" --bridge-socket "$bridge" >/dev/null
else
  echo "term-shot: capture failed; grant Screen Recording to this shell or install Peekaboo" >&2
  exit 1
fi
echo "$out"
