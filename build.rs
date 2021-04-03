use std::{
    env,
    fs::{create_dir, read_dir, remove_file, File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "windows")]
const GLSLANG_VALIDATOR: &'static str = "glslangValidator.exe";

#[cfg(not(target_os = "windows"))]
const GLSLANG_VALIDATOR: &'static str = "glslangValidator";

fn main() {
    //match build() {
    //    Ok(()) => {}
    //    Err(e) => {
    //        eprintln!("{}", e);
    //        std::process::exit(1);
    //    }
    //}
}

#[allow(dead_code)]
fn build() -> Result<(), String> {
    // find glslangValidator
    let glslang_validator = find_executable(GLSLANG_VALIDATOR)
        .ok_or(format!("Could not find {} on PATH.\nPlease add it to PATH or set environment variable GLSLANG_VALIDATOR_PATH.", GLSLANG_VALIDATOR))?;

    // get shaders dir and cache
    let shaders_dir = PathBuf::from("shaders");
    let mut cache_dir = shaders_dir.clone();
    cache_dir.push("cache");
    if let Err(e) = create_dir(&cache_dir) {
        if e.kind() != ErrorKind::AlreadyExists {
            return Err(format!("Could not create cache dir: {}", e));
        }
    }

    let mut output = std::collections::HashSet::new();

    // read the dir
    let dir =
        read_dir(&shaders_dir).map_err(|e| format!("Could not read directory shaders/\n{}", e))?;

    // TODO recursive?
    for entry in dir {
        let entry = entry.map_err(|e| format!("Could not read dir entry\n{}", e))?;

        // get the shader in path
        let shader_in = entry.path();

        // output path
        let shader_out = shader_in.with_extension("spv");

        if let Some(extension) = shader_in.extension() {
            if extension.to_string_lossy() == "spv" {
                continue;
            }
        }

        // error out if we already compiled to that name
        if output.contains(&shader_out) {
            return Err(format!(
                "Another shader has already compiled to {}\nPlease rename the shaders.",
                shader_out.display()
            ));
        }

        // read its metadata
        let meta = entry
            .metadata()
            .map_err(|e| format!("Could not read metadata of {}\n{}", shader_in.display(), e))?;

        if meta.is_dir() && shader_in.ends_with("cache") {
            // skip the cache dir
            continue;
        }

        // get last modified date
        let last_modified = format!(
            "{:?}",
            meta.modified().map_err(|e| {
                format!(
                    "Could not read modified date of {}\n{}",
                    shader_in.display(),
                    e
                )
            })?
        );

        // compare to cached
        cache_dir.push(shader_in.file_name().unwrap());
        if let Ok(mut file) = OpenOptions::new().read(true).write(true).open(&cache_dir) {
            let mut cached_modified = String::new();
            file.read_to_string(&mut cached_modified).map_err(|e| {
                format!(
                    "Could not read cached date of {}\n{}",
                    shader_in.display(),
                    e
                )
            })?;

            if cached_modified == last_modified {
                // if they're the same don't bother compiling
                cache_dir.pop();
                continue;
            } else {
                file.seek(SeekFrom::Start(0)).map_err(|e| {
                    format!(
                        "Couldn't seek to beginning of {}\n{}",
                        cache_dir.display(),
                        e
                    )
                })?;
                file.write_all(last_modified.as_bytes())
                    .map_err(|e| format!("Couldn't write {}\n{}", cache_dir.display(), e))?;
            }
        } else if let Ok(mut file) = File::create(&cache_dir) {
            file.write_all(last_modified.as_bytes())
                .map_err(|e| format!("Couldn't write {}\n{}", cache_dir.display(), e))?;
        } else {
            return Err(format!("Could not find or create {}", cache_dir.display()));
        }
        cache_dir.pop();

        // at this point we only have unchanged and unique files left to compile, so remove those spv files
        if let Err(e) = remove_file(&shader_out) {
            if e.kind() != ErrorKind::NotFound {
                return Err(format!(
                    "Could not remove stale out file {}\n{}",
                    shader_out.display(),
                    e
                ));
            }
        }

        // and compile!
        eprintln!(
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
                    "Could not compile {}\nstderr: {}\nstdout: {}",
                    shader_out.display(),
                    String::from_utf8_lossy(&out.stderr),
                    String::from_utf8_lossy(&out.stdout)
                ));
            }
        } else {
            return Err(format!(
                "Could not run compiler for {}\n{}",
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
