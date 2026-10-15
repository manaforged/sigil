#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: scripts/term-shot.sh --out PNG [--app Terminal|Ghostty] [--size COLSxROWS] [--wait SECONDS] -- COMMAND [ARGS...]" >&2
  exit 2
}

app=Terminal
size=120x36
wait=4
out=
while [ $# -gt 0 ]; do
  case "$1" in
    --app) app=$2; shift 2 ;;
    --size) size=$2; shift 2 ;;
    --wait) wait=$2; shift 2 ;;
    --out) out=$2; shift 2 ;;
    --) shift; break ;;
    *) usage ;;
  esac
done
[ -n "$out" ] && [ $# -gt 0 ] || usage
[ "$(uname)" = Darwin ] || { echo "term-shot: macOS only" >&2; exit 2; }
case "$app" in Terminal|Ghostty) ;; *) echo "term-shot: --app is Terminal or Ghostty" >&2; exit 2 ;; esac
cols=${size%x*}
rows=${size#*x}

work=$(mktemp -d "${TMPDIR:-/tmp}/term-shot.XXXXXX")
command=$(printf '%q ' "$@")
cat > "$work/run.command" <<EOF
#!/bin/sh
printf '\033[8;${rows};${cols}t'
echo \$\$ > "$work/pid"
tty > "$work/tty"
cd $(printf '%q' "$PWD")
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
windows() { swift "$work/windows.swift" "$app" 2>/dev/null | sort; }

before=$(windows)
if [ "$app" = Terminal ]; then
  open -a Terminal "$work/run.command"
else
  open -na Ghostty --args -e "$work/run.command"
fi
sleep "$wait"
window=$(comm -13 <(printf '%s\n' "$before") <(windows) | head -1)
[ -n "$window" ] || { echo "term-shot: no new $app window" >&2; exit 1; }

bridge="$HOME/Library/Application Support/Peekaboo/bridge.sock"
if screencapture -x -o -l "$window" "$out" 2>/dev/null && [ -s "$out" ]; then
  :
elif command -v peekaboo >/dev/null 2>&1 && [ -S "$bridge" ]; then
  peekaboo see --no-elements --app "$app" --window-id "$window" --path "$out" --bridge-socket "$bridge" >/dev/null
else
  echo "term-shot: capture failed; grant Screen Recording to this shell or install Peekaboo" >&2
  exit 1
fi

if [ -f "$work/pid" ]; then
  pid=$(cat "$work/pid")
  pkill -TERM -P "$pid" 2>/dev/null || true
  kill -TERM "$pid" 2>/dev/null || true
fi
if [ "$app" = Terminal ] && [ -f "$work/tty" ]; then
  sleep 0.5
  osascript -e 'on run {tty}' -e 'tell application "Terminal"' -e 'repeat with w in windows' \
    -e 'if tty of selected tab of w is tty then close w saving no' -e 'end repeat' -e 'end tell' -e 'end run' \
    "$(cat "$work/tty")" >/dev/null 2>&1 || true
fi
rm -f "$work/run.command" "$work/windows.swift" "$work/pid" "$work/tty"
rmdir "$work" 2>/dev/null || true
echo "$out"
