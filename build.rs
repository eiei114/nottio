fn main() {
    println!("cargo:rerun-if-changed=assets/windows.rc");
    println!("cargo:rerun-if-changed=assets/nottio.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // GPUI's Windows platform loads the executable's ICON resource #1 for
        // its native window class. Explorer reads that same embedded icon.
        embed_resource::compile_for("assets/windows.rc", ["nottio"], embed_resource::NONE)
            .manifest_required()
            .expect("nottio's Windows icon resource must compile and link");
    }
}
