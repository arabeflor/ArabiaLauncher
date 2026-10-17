use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=img/app-icon.png");
    if !env::var("TARGET").is_ok_and(|target| target.contains("windows")) {
        return;
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let png_icon = manifest_dir.join("img").join("app-icon.png");
    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    let icon = output_dir.join("arabia-launcher-icon.ico");
    create_windows_icon(&png_icon, &icon);

    let Some(resource_compiler) = find_resource_compiler() else {
        println!(
            "cargo:warning=Windows resource compiler (rc.exe) not found; app icon remains available in the window but will not be embedded in the executable"
        );
        return;
    };

    let resource_script = output_dir.join("arabia-launcher-icon.rc");
    let resource_file = output_dir.join("arabia-launcher-icon.res");
    let escaped_icon_path = icon.to_string_lossy().replace('\\', "\\\\");
    fs::write(
        &resource_script,
        format!("1 ICON \"{escaped_icon_path}\"\n"),
    )
    .expect("write Windows icon resource script");

    let result = Command::new(&resource_compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource_file)
        .arg(&resource_script)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "failed to run Windows resource compiler {}: {error}",
                resource_compiler.display()
            )
        });
    if !result.status.success() {
        panic!(
            "Windows resource compiler failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    println!(
        "cargo:rustc-link-arg-bin=ArabiaLauncher={}",
        resource_file.display()
    );
}

fn create_windows_icon(png_path: &std::path::Path, ico_path: &std::path::Path) {
    use image::codecs::ico::{IcoEncoder, IcoFrame};
    use image::imageops::FilterType;
    use image::{ExtendedColorType, ImageReader};

    let image = ImageReader::open(png_path)
        .unwrap_or_else(|error| panic!("failed to open app logo {}: {error}", png_path.display()))
        .decode()
        .unwrap_or_else(|error| panic!("failed to decode app logo {}: {error}", png_path.display()))
        .to_rgba8();
    let sizes = [16, 24, 32, 48, 64, 128, 256];
    let images = sizes
        .iter()
        .map(|size| image::imageops::resize(&image, *size, *size, FilterType::Lanczos3))
        .collect::<Vec<_>>();
    let frames = images
        .iter()
        .zip(sizes)
        .map(|(image, size)| {
            IcoFrame::as_png(image.as_raw(), size, size, ExtendedColorType::Rgba8)
                .expect("encode app logo icon frame")
        })
        .collect::<Vec<_>>();
    let file = fs::File::create(ico_path).unwrap_or_else(|error| {
        panic!("failed to create app icon {}: {error}", ico_path.display())
    });
    IcoEncoder::new(file)
        .encode_images(&frames)
        .expect("encode Windows app icon");
}

fn find_resource_compiler() -> Option<PathBuf> {
    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    let path_directories = env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    for directory in path_directories {
        let candidate = directory.join("rc.exe");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let mut sdk_roots = Vec::new();
    if let Some(sdk_dir) = env::var_os("WindowsSdkDir") {
        sdk_roots.push(PathBuf::from(sdk_dir).join("bin"));
    }
    if let Some(program_files) = env::var_os("ProgramFiles(x86)") {
        sdk_roots.push(
            PathBuf::from(program_files)
                .join("Windows Kits")
                .join("10")
                .join("bin"),
        );
    }

    for sdk_root in sdk_roots {
        let Ok(entries) = fs::read_dir(&sdk_root) else {
            continue;
        };
        let mut versions = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        versions.sort();
        for version in versions.into_iter().rev() {
            for candidate in [
                version.join(arch).join("rc.exe"),
                version.join("hostx64").join(arch).join("rc.exe"),
            ] {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
