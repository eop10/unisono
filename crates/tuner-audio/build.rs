fn main() {
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // oboe-sys 0.6.1 не линкует libc++_static/libc++abi сам (закомментировано в upstream build.rs),
        // без этого в cdylib остаются неразрешённые C++-символы (__cxa_pure_virtual и др.).
        println!("cargo:rustc-link-lib=c++_static");
        println!("cargo:rustc-link-lib=c++abi");
    }
}
