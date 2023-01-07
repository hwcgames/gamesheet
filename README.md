# GameSheet

GameSheet is a vaguely spreadsheet-ish thing for storing game parameters, inspired by [this YouTube video by Masahiro Sakurai](https://www.youtube.com/watch?v=nGaajB8m5Q0).

## Features

* [x] Simple format, based on YAML
* [x] Built in Rust for performance and memory safety.
* [x] Cached evaluation; after the first time reading a given value, reading a parameter is just a hashmap lookup.
* [x] Values are scripts, so you can perform arbitrary computations and read other values.
* [x] Values can be modified at runtime (even complex scripted ones!), with simple dependency tracking to purge cached values that could be affected by the change.
* [x] Reusable functions can be declared in the prelude and used in value scripts.
* [x] Dual-licensed with MIT and Apache to fit into your project.
* [x] Should work OK in WebAssembly and wherever else you put it, as long as a heap is available.
* [ ] Bindings for the Godot game engine via GDNative
* [ ] Graphical editor to be more confident when editing sheets.
* [ ] Bindings for languages other than Rust.

## Non-features

* It doesn't use memory very efficiently. It stores the script in memory as a string, AST, and cached result, and it uses heap allocations quite liberally.
* It uses [Rhai](https://rhai.rs) for scripting, so it's not terribly fast when it does have to execute scripts. It might not be best to change things up too often, so you can take advantage of caching.