# BG3 Plugin Lib

This Rust support library is made to be used for [BG3 Plugins](https://github.com/MolotovCherry/BG3-Plugin-Template-Rust) and [Yet Another BG3 Native Mod Loader](https://github.com/MolotovCherry/Yet-Another-BG3-Native-Mod-Loader).

If you are a mod manager developer, this also exports a C api (see include directory), cdylib, and staticlib to allow you to get a plugin's info from a dll.
It also exports the same function for Rust library users as well.

For plugin devs, use the `plugin!()` macro to declare your plugin details. For example
```rust
plugin! {
    "MyPLugin",
    "My Plugin Description"
}
```
