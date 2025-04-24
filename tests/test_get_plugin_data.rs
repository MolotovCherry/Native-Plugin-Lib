use std::path::Path;

#[test]
fn test_loader() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let loader = manifest_dir.join("tests").join("loader.dll");
    let data = native_plugin_lib::PluginData::new(loader).unwrap();

    let plugin = data.plugin();

    assert!(&*plugin.author == "Cherry");
    assert!(&*plugin.name == "Loader");
    assert!(&*plugin.description == "Plugin loader for Yet-Another-BG3-Native-Mod-Loader");
    assert!(plugin.version.major == 0);
    assert!(plugin.version.minor == 1);
    assert!(plugin.version.patch == 0);
}
