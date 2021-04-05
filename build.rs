use std::{
    env,
    fs::{read_dir, remove_file},
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

use fs_extra::{copy_items, dir::CopyOptions};

#[cfg(target_os = "windows")]
const GLSLANG_VALIDATOR: &'static str = "glslangValidator.exe";

#[cfg(not(target_os = "windows"))]
const GLSLANG_VALIDATOR: &'static str = "glslangValidator";

fn main() {
    match build() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    println!("cargo:rerun-if-changed=res/*");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let paths = vec!["res/"];
    copy_items(&paths, out_dir, &copy_options).unwrap();
}

#[allow(dead_code)]
fn build() -> Result<(), String> {
    // find glslangValidator
    let glslang_validator = find_executable(GLSLANG_VALIDATOR)
        .ok_or(format!("Could not find {} on PATH.\nPlease add it to PATH or set environment variable GLSLANG_VALIDATOR_PATH.", GLSLANG_VALIDATOR))?;

    // get shaders dir
    let shaders_dir = PathBuf::from("shaders");

    let mut output = std::collections::HashSet::new();

    // read the dir
    let dir =
        read_dir(&shaders_dir).map_err(|e| format!("Could not read directory shaders/\n{}", e))?;

    // TODO recursive?
    for entry in dir {
        let entry = entry.map_err(|e| format!("Could not read dir entry\n{}", e))?;

        // only run (the rest of this loop)? if the file is changed
        println!(
            "cargo:rerun-if-changed={}",
            entry.file_name().to_str().unwrap()
        );

        // get the shader in path
        let shader_in = entry.path();

        // output path
        let shader_out = shader_in.with_extension("spv");

        if let Some(extension) = shader_in.extension() {
            if extension.to_string_lossy() == "spv" {
                continue;
            }
        }

        if let Err(e) = remove_file(&shader_out) {
            if e.kind() != ErrorKind::NotFound {
                return Err(format!(
                    "Could not remove stale out file {}\n{}",
                    shader_out.display(),
                    e
                ));
            }
        }

        println!(
            "compile {} to {}",
            shader_in.display(),
            shader_out.display()
        );

        let out = Command::new(&glslang_validator)
            .arg("-V")
            .arg(&shader_in)
            .arg("-o")
            .arg(&shader_out)
            .output();

        output.insert(shader_out.clone());

        if let Ok(out) = out {
            if !out.status.success() {
                return Err(format!(
                    "Could not compile {}\nCompiler said: (stderr) {}\n(stdout) {}",
                    shader_out.display(),
                    String::from_utf8_lossy(&out.stderr),
                    String::from_utf8_lossy(&out.stdout)
                ));
            }
        } else {
            return Err(format!(
                "Could not run compiler for {} -> {}\n{}",
                shader_in.display(),
                shader_out.display(),
                out.unwrap_err()
            ));
        }
    }

    Ok(())
}

fn find_executable<P>(exe_name: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    let exe_name = exe_name.as_ref();
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(&exe_name);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}
