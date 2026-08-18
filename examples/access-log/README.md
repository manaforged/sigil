# Live tape

Same 1 million HTTP lines in a Sigil window and a GPUI 0.2.2 window.
The book is a `Vec` of 16-byte records (~16 MiB). The list paints 60
rows × 8 cells. Every 16 ms the visible band ticks (`bytes` / `seq`).

Sigil keeps the row NodeIds and updates the two live cells. GPUI 0.2.2
rebuilds the element tree on every tick. That is the tap-out: same
virtual list, different per-frame work. The HUD prints tick/rebuild
time.

```text
ACCESS_LOG_ROWS=200000 cargo run --release -p access-log --bin access-log-sigil

ACCESS_LOG_ROWS=200000 cargo run --release -p access-log --bin access-log-gpui --features gpui-window

# default 1M
cargo run --release -p access-log --bin access-log-sigil
```

`ACCESS_LOG_ROWS=10000000` holds ten million lines (~160 MiB). The
window is still 60 rows.
