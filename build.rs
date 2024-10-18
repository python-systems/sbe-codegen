use rustc_version::{version_meta, Channel};

fn main() {
    let channel = version_meta()
        .expect("rustc version meta can be resolved")
        .channel;

    if channel != Channel::Nightly {
        println!("cargo::warning=sbe-codegen can only be compiled with nightly Rust >= 1.73.0.");
        std::process::exit(1);
    }
}
