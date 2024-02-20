# Native Plugin Lib

This Rust support library is made to be used for [Native Plugins](https://github.com/MolotovCherry/Native-Plugin-Template-Rust).

If you are a mod manager developer, this also exports a C api (see include directory), cdylib, and staticlib to allow you to get a plugin's info from a dll.
It also exports the same function for Rust library users as well.

For plugin devs, use the `plugin!()` macro to declare your plugin details. For example
```rust
plugin! {
    "MyPLugin",
    "My Plugin Description"
}
```
