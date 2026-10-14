fn main() {
    // Gives yarita.exe its icon in Explorer, the taskbar and shortcuts.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("assets/yarita.rc", embed_resource::NONE).manifest_optional().unwrap();
    }
}
