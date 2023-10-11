use std::ffi::OsStr;
use std::fs::read_dir;
use std::io::Result;
use std::process::Command;
use std::path::PathBuf;
use std::env;

fn xcrun(cmd: &str) -> Command {
    let mut out = Command::new("xcrun");
    out.args([
        "-sdk",
        "macosx",
        cmd
    ]);
    out
}

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let metal_files: Vec<_> = read_dir("metal/")?
        .filter_map(|f| f.ok())
        .map(|de| de.path())
        .filter(|f| {
            if let Some(ext) = f.extension().and_then(OsStr::to_str) {
                matches!(ext, "metal" | "hpp")
            } else {
                false
            }
        })
        .collect();

    metal_files.iter().for_each(|p| {
        println!("cargo:rerun-if-changed={}", p.to_str().unwrap());
    });

    let c_args: Vec<_> = metal_files
        .iter()
        .filter(|p| {
            p
                .extension()
                .and_then(OsStr::to_str)
                .map(|ext| ext == "metal")
                .unwrap_or_default()
        })
        .map(|p| p.to_str().unwrap())
        .collect();

    let mut child = xcrun("metal")
        .arg("-o")
        .arg(out_dir.join("metalsha.ir"))
        .arg("-c")
        .args(&c_args)
        .spawn()?;

    if !child.wait()?.success() {
        todo!();
    }

    xcrun("metallib")
        .arg("-o")
        .arg(out_dir.join("metalsha.metallib"))
        .arg(out_dir.join("metalsha.ir"))
        .spawn()?;

    if !child.wait()?.success() {
        todo!();
    }

    Ok(())
}