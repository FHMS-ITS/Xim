# Xim (unstable)

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

# Usecases

* [x] Open/Create
* [ ] Open/Create with `:e <file>`
* [x] Save with `:w`
* [ ] Save as with `:w <file>`
* [x] Save and exit with `:x` or `:wq`
* [x] Exit with `:q` or `:q!`
* [x] Statusbar (State, Position)
* [x] Move
* [x] Jump
* [ ] Jump to Start/End `gg`, `G`
* [ ] Relative Jumps
* [x] Scroll
* [x] Insert
* [x] Delete
* [x] Replace
* [x] Visual mode
* [ ] Copy/Paste
* [ ] Edit in ASCII mode
* [x] Undo/Redo
* [ ] Highlite differences
* [ ] Portable colors
* [ ] Persistent rope
* [ ] Lazy Loading
* [ ] Efficient saving
