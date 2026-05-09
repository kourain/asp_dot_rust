// build.rs
fn main() {
    let now = chrono::Local::now();
    println!(
        "cargo:rustc-env=BUILD_TIME={}",
        now.format("%Y-%m-%d %H:%M:%S %:z")
    );
}