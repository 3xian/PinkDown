fn main() {
    println!("cargo:rerun-if-changed=assets/pinkdown.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winres::WindowsResource::new()
            .set_icon("assets/pinkdown.ico")
            .set("FileDescription", "PinkDown")
            .set("ProductName", "PinkDown")
            .set("OriginalFilename", "pinkdown.exe")
            .set("InternalName", "pinkdown")
            .compile()
            .expect("embed the PinkDown Windows icon and version info");
    }
}
