use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=web/");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=vite.config.ts");

    let web_build_status = Command::new("npm")
        .args(["run", "build"])
        .status()
        .expect("failed to run `npm run build`");

    if !web_build_status.success() {
        println!("failed to build web")
    }
}
