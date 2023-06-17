extern crate bindgen;

use core::panic;
use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Build our specific libe2fs version!
    let pwd: PathBuf = std::env::current_dir().unwrap();
    let project_root = find_self_proj_dir(&pwd);
    let out_dir = std::env::var("OUT_DIR").unwrap();
    eprintln!("mkdir _build");
    // We don't want to raise just in case it doesn't already exist. This helps
    // ensure clean builds.
    #[allow(unused_must_use)]
    {
        std::fs::remove_dir_all(format!("{out_dir}/_build"));
    }
    std::fs::create_dir_all(format!("{out_dir}/_build")).unwrap();

    eprintln!("copy e2fsprogs");
    let mut cmd = std::process::Command::new("cp");
    cmd.arg("-r")
        .arg(format!("{}/e2fsprogs", project_root.display()));
    cmd.arg(format!("{}/build-e2fs.sh", project_root.display()));
    cmd.arg(format!("{out_dir}/_build"));
    let res = cmd.output().unwrap();

    if !res.status.success() {
        panic!(
            "Failed to copy e2fsprogs:\n--------\n{}\n--------\n{}\n--------\n",
            String::from_utf8(res.stdout).unwrap(),
            String::from_utf8(res.stderr).unwrap()
        );
    }

    // run ./build-e2fs.sh!
    eprintln!("_build/build-e2fs.sh");
    std::env::set_current_dir(format!("{}/_build", &out_dir)).unwrap();
    let mut cmd = std::process::Command::new("bash");
    cmd.arg("build-e2fs.sh");
    let res = cmd.output().unwrap();
    std::env::set_current_dir(&pwd).unwrap();

    if !res.status.success() {
        panic!(
            "Failed to build e2fsprogs:\n--------\n{}\n--------\n{}\n--------\n",
            String::from_utf8(res.stdout).unwrap(),
            String::from_utf8(res.stderr).unwrap()
        );
    }

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={out_dir}/_build/e2fsprogs/build/lib");
    println!("cargo:rustc-link-search=/usr/include");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=static:+verbatim=libext2fs.a");
    println!("cargo:rustc-link-lib=static:+verbatim=libcom_err.a");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .derive_debug(true)
        .derive_copy(true)
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn find_self_proj_dir(pwd: &Path) -> PathBuf {
    eprintln!("searching: {}", pwd.display());
    if pwd.file_name().is_some()
        && pwd
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("libe2fs-sys")
    {
        return pwd.to_path_buf();
    }

    if let Some(parent) = pwd.parent() {
        find_self_proj_dir(parent)
    } else {
        panic!("Could not find libe2fs-sys directory from pwd");
    }
}
