use cmake::Config;

fn main() {
    let dst = Config::new("ooz")
        .build_target("libooz")
        .build();

    println!("cargo:rustc-link-search=native={}", format!("{}/build", dst.display()));
    println!("cargo:rustc-link-lib=static=libooz");
}
