# orbit

A small **n-body** simulator with two integrators.

## Schemes

- `Verlet`: symplectic, keeps energy bounded over long runs
- `Rk4`: fourth order, accurate over short spans

## Run

Run `cargo run --release` to simulate one year of the Earth around the Sun.

Run `cargo test` to check energy drift.
