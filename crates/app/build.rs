//! On Windows the icon goes into the exe, for the Explorer and the window.

fn main() {
    println!("cargo:rerun-if-changed=windows.rc");
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // a build without a resource compiler still runs, only with the plain icon
    if let Err(e) = embed_resource::compile("windows.rc", embed_resource::NONE).manifest_optional() {
        println!("cargo:warning=no icon in the exe: {e}");
    }
}
