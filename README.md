# Rust Snake

[![Rust](https://img.shields.io/badge/language-Rust-orange?logo=rust)](https://www.rust-lang.org/)
[![Macroquad](https://img.shields.io/badge/game%20framework-Macroquad-5c8dbc)](https://crates.io/crates/macroquad)

A small Snake game built with Rust and Macroquad.

## Overview

Rust Snake is a desktop game where you guide a snake around the board, collect food, and try to beat your score without running into yourself. It includes three difficulty levels and a local leaderboard. The repository also provides a compact example of a graphical game implemented in Rust.

## Screenshots

No screenshots are currently included in the repository.

<!-- TODO: Add screenshots here when gameplay images are available. -->

## Features

- Three selectable difficulty levels: Easy, Normal, and Hard. Higher difficulties move the snake faster.
- The snake wraps around the board edges.
- Standard food adds 1 point. A bonus item worth 5 points appears after every 10 standard items eaten.
- The game ends when the snake collides with its own body.
- A local leaderboard retains up to 10 scores, ranked by difficulty and then score.
- The start screen displays the top five saved scores; the in-game panel displays the top 10.
- Player names can be up to 20 characters.

## Controls

| Key | Action |
| --- | --- |
| Type characters | Enter a player name on the start screen |
| `Backspace` | Remove the last character from the player name |
| `Left` / `Right` | Change difficulty on the start screen |
| `Enter` | Start a game when the player name is not blank |
| `W` / `A` / `S` / `D` | Move up / left / down / right |
| `P` or `Esc` | Pause a game or resume from pause |
| `R` | Return to the start screen after game over |

You can also click the difficulty buttons on the start screen.

## Download and Run

There is currently no precompiled GitHub release available. Build the game from source using the instructions below.

The project currently has no external game asset files. If a future build uses external assets, distribute the executable together with those required assets. The leaderboard is also stored beside the executable, so that directory must be writable to save scores.

## Build from Source

### Prerequisites

- Rust and Cargo, installed through [rustup](https://rustup.rs/)
- Git

Clone the repository and build the optimized executable:

```sh
git clone https://github.com/prometheusid/rust-snake-game.git
cd rust-snake-game
cargo build --release
```

Run the game directly through Cargo:

```sh
cargo run --release
```

On Windows, the release executable is `target\release\belajar-rust.exe`. You can launch it directly; Windows release builds use the GUI subsystem and do not open a separate console window.

## Leaderboard and Save Data

The game reads and writes `leaderboard.txt` in the same directory as the running executable, regardless of the current working directory. If the file is missing, the game starts with an empty leaderboard and creates the file when it saves a score.

Leaderboard entries are local to the game files and are not uploaded or shared online. Separate copies of the game have separate leaderboard files when kept in separate directories. The executable's directory must be writable for new scores to be saved; load and save errors are shown in the game window.

## Project Structure

```text
.
├── Cargo.toml
├── Cargo.lock
├── .gitignore
├── leaderboard.txt
└── src/
    └── main.rs
```

The repository includes a `leaderboard.txt` at its root, but Cargo does not copy it to the build output. A built executable reads or creates the file beside itself.

## Technology Stack

- **Language:** Rust, edition 2024
- **Game framework:** [Macroquad](https://crates.io/crates/macroquad) 0.4.16

## Development and Testing

From the repository root, these Cargo commands are useful during development:

```sh
cargo fmt --check
cargo check
cargo test
cargo build --release
```

All four commands were run successfully while preparing this README. The test suite currently covers leaderboard sorting, executable-relative leaderboard path resolution, and missing-file/save/reload behavior.

## License and Credits

No license file or separate attribution notices are currently present in the repository, so the project's license has not yet been specified. Add a license before publishing if you intend to grant others specific reuse rights.

The project uses the Rust language and Macroquad game framework.
