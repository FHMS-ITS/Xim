# Xim

Xim is useful for simple hex editing tasks. However, it will consume much memory, because the final data structure (a persistent rope) is not integrated yet. Use at your own risc.

![Screenshot](assets/xim.png)

## Installation

### Install Rustup

Arch Linux:

```
$ sudo pacman -S rustup
```

Others:

```
$ curl https://sh.rustup.rs -sSf | sh
```

### Update/Install Toolchain

```
$ rustup install stable
$ rustup update
```

## Build Xim

```
$ cargo build
```

## Run Xim

Test with

```
$ cargo run -- <file>
```

or build in release mode

```
$ cargo build --release
$ ./target/release/xim <file>
```

# Incomplete collection of implemented and missing features

* [x] Open/Create
* [ ] Open/Create with `:e <file>`
* [x] Save with `:w`
* [x] Save as with `:w <file>`
* [x] Save and exit with `:x` or `:wq`
* [x] Exit with `:q` or `:q!`
* [x] Statusbar (State, Position)
* [x] Move
* [x] Absolute Jumps (0b..., 0o..., 0x..., int)
* [ ] Jump to Start/End `gg`, `G`
* [ ] Relative Jumps
* [x] Scroll
* [x] Insert
* [x] Delete
* [x] Replace
* [x] Visual mode
* [x] Yank/Paste
* [x] Copy/Paste (from clipboard)
* [ ] Edit in ASCII mode (partially implemented)
* [x] Undo/Redo
* [ ] Highlite differences
* [ ] Portable colors
* [ ] Optimize drawing (avoid flickering)
* [ ] Persistent rope
* [ ] Lazy loading/unloading of memory pages
* [ ] Efficient saving
