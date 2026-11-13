#[path = "src/branding.rs"]
mod branding;

fn main() {
    println!("cargo:rerun-if-changed=src/branding.rs");
    println!("cargo:rerun-if-env-changed=HOVER_PACKAGE_VERSION");
    println!("cargo:rerun-if-env-changed=HOVER_RC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(output.join("hover.ico"), branding::ico()).unwrap();
    let version = std::env::var("HOVER_PACKAGE_VERSION")
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap());
    let parts = version
        .split('.')
        .map(|part| part.parse::<u16>())
        .collect::<Result<Vec<_>, _>>()
        .expect("Package version must be numeric");
    assert!(
        parts.len() == 3,
        "Package version must contain three numeric components"
    );
    let numeric = format!("{},{},{},0", parts[0], parts[1], parts[2]);
    let icon = output
        .join("hover.ico")
        .to_string_lossy()
        .replace('\\', "/");
    let resource = format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
 FILEVERSION {numeric}
 PRODUCTVERSION {numeric}
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0
 FILEOS 0x40004L
 FILETYPE 1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "CompanyName", "Hover contributors\0"
   VALUE "FileDescription", "Hover development build\0"
   VALUE "FileVersion", "{version}\0"
   VALUE "ProductName", "Hover\0"
   VALUE "ProductVersion", "{version}\0"
   VALUE "LegalCopyright", "MIT License; Hover contributors\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#
    );
    std::fs::write(output.join("hover.rc"), resource).unwrap();
    let rc = std::env::var_os("HOVER_RC")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let kits = std::path::Path::new("C:/Program Files (x86)/Windows Kits/10/bin");
            let mut versions = std::fs::read_dir(kits)
                .ok()?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.join("x64/rc.exe").is_file())
                .collect::<Vec<_>>();
            versions.sort();
            versions.pop().map(|p| p.join("x64/rc.exe"))
        })
        .expect("Windows SDK rc.exe is required; set HOVER_RC to its trusted path");
    let status = std::process::Command::new(rc)
        .args(["/nologo", "/c65001", "/fo"])
        .arg(output.join("hover.res"))
        .arg(output.join("hover.rc"))
        .status()
        .expect("Run Windows SDK resource compiler");
    assert!(status.success(), "Windows resource compilation failed");
    for binary in ["hover", "hover-agent"] {
        println!(
            "cargo:rustc-link-arg-bin={binary}={}",
            output.join("hover.res").display()
        );
    }
}
