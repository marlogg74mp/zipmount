//! The modern context menu: a sparse package with a COM handler.
//!
//! Registry items (see `shell.rs`) only make it under "Show more options" in
//! Windows 11. The main menu admits only `IExplorerCommand` handlers declared
//! by a package with application identity. This module builds, signs and
//! installs such a package.
//!
//! Three decisions worth explaining.
//!
//! **The package is sparse.** The manifest lives inside the `.msix`, while
//! `zipmount.exe` and `zipmount_shell.dll` stay ordinary files in the
//! "external content" directory. The program does not move into the package
//! and keeps running from the command line as before.
//!
//! **Files are copied to `%ProgramData%\ZipMount\bin` rather than taken from
//! the build directory.** File Explorer loads the DLL into its process and
//! never lets go; were the package pointing at `target\release`, the next
//! build could not overwrite the file. The menu also survives `cargo clean`.
//!
//! **The DLL's name carries a fingerprint of its contents.** For the very same
//! reason: a library loaded by File Explorer cannot be overwritten, but a new
//! one can be put next to it. Reinstalling without changes simply finds its
//! file in place.
//!
//! The price of the main menu is one action with administrator rights: the
//! package needs a signature, and trusting a certificate is a machine-wide
//! decision. Everything else happens as the user.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::process::Command as ProcCommand;

use anyhow::{bail, Context, Result};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::IExplorerCommand;
use zipmount_i18n::t;

use super::shell;

/// The package name and the certificate subject must match `Publisher` in
/// the manifest, or deployment rejects the signature.
const PACKAGE_NAME: &str = "ZipMount";
const PUBLISHER: &str = "CN=ZipMount";

/// Icon for the menu items; `IExplorerCommand::GetIcon` returns its path.
const ICON: &[u8] = include_bytes!("../../assets/ZipMount.ico");

/// Tiles that go into the package.
///
/// The `altform-unplated_targetsize-*` variants are not for tiles but for the
/// "ZipMount" row under which Windows groups our items in the context menu:
/// the shell draws that row's icon itself and takes it from the package. A
/// plain `Square44x44Logo.png` is not enough — it looks for an unplated variant
/// of the right size, by file name. Without these files the row has no icon,
/// though the items under it do.
macro_rules! unplated {
    ($size:literal) => {
        (
            concat!(
                "Square44x44Logo.altform-unplated_targetsize-",
                $size,
                ".png"
            ),
            include_bytes!(concat!(
                "../../assets/Square44x44Logo.altform-unplated_targetsize-",
                $size,
                ".png"
            )) as &[u8],
        )
    };
}

const ASSETS: &[(&str, &[u8])] = &[
    (
        "Square44x44Logo.png",
        include_bytes!("../../assets/Square44x44Logo.png"),
    ),
    (
        "Square150x150Logo.png",
        include_bytes!("../../assets/Square150x150Logo.png"),
    ),
    (
        "StoreLogo.png",
        include_bytes!("../../assets/StoreLogo.png"),
    ),
    unplated!("16"),
    unplated!("24"),
    unplated!("32"),
    unplated!("48"),
    unplated!("64"),
    unplated!("256"),
];

/// The install directory.
///
/// Not `%LOCALAPPDATA%`, even though that is where such a program belongs:
/// package deployment refuses to work with anything inside `AppData`. A
/// `.msix` does not install from there at all, and external content does
/// install, but then COM cannot find the library — both failures look like
/// "path not found" (0x80070003) with the files in place. Checked for both
/// `Local` and `Roaming`. `ProgramData` works and needs no administrator.
fn root() -> Result<PathBuf> {
    let base =
        std::env::var_os("ProgramData").context("cannot determine the ProgramData directory")?;
    Ok(PathBuf::from(base).join("ZipMount"))
}

fn bin_dir() -> Result<PathBuf> {
    Ok(root()?.join("bin"))
}

/// The directory the package is built in.
///
/// A temporary one: the `.msix` itself is not needed after installation — the
/// system copies its contents — so we build and clean up after ourselves.
/// Curiously, `%TEMP%` lies inside `AppData` but escapes the ban described at
/// `root`: the system exempts the temporary directory.
fn staging_dir() -> PathBuf {
    std::env::temp_dir().join("zipmount-package")
}

/// A short fingerprint of contents — only to tell DLL versions apart.
///
/// No cryptography is needed: the file is our own, and the task is not to
/// confuse two builds.
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Finds a Windows SDK tool, taking the newest installed version.
fn sdk_tool(name: &str) -> Result<PathBuf> {
    let roots = [
        PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin"),
        PathBuf::from(r"C:\Program Files\Windows Kits\10\bin"),
    ];

    let mut found: Vec<PathBuf> = Vec::new();
    for base in roots {
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("x64").join(name);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }

    // The directories are named after version numbers, so sorting by name
    // gives the right order.
    found.sort();
    found
        .pop()
        .with_context(|| t!("err-sdk-tool-missing", tool = name.to_string()))
}

/// Runs PowerShell and returns its output.
fn powershell(script: &str) -> Result<String> {
    let output = ProcCommand::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .with_context(|| t!("err-run-failed", tool = "powershell"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(t!(
            "err-tool-failed",
            tool = "powershell",
            output = stderr.trim().to_string()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_tool(tool: &Path, args: &[&str]) -> Result<()> {
    let output = ProcCommand::new(tool)
        .args(args)
        .output()
        .with_context(|| t!("err-run-failed", tool = tool.display().to_string()))?;

    if !output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(t!(
            "err-tool-failed",
            tool = tool.display().to_string(),
            output = format!("{}\n{}", text.trim(), stderr.trim())
                .trim()
                .to_string()
        ));
    }
    Ok(())
}

/// The package manifest.
///
/// Built from the same extension list as the registry items and from the
/// class table next to the handler itself — so that the two cannot drift
/// apart.
fn manifest(dll: &str) -> String {
    let mut classes = String::new();
    for verb in zipmount_shell::VERBS {
        classes.push_str(&format!(
            "              <com:Class Id=\"{}\" Path=\"{dll}\" ThreadingModel=\"STA\" />\n",
            verb.clsid
        ));
    }

    let archive_verbs: String = zipmount_shell::VERBS
        .iter()
        .filter(|verb| matches!(verb.target, zipmount_shell::Target::Archive))
        .map(|verb| {
            format!(
                "              <desktop5:Verb Id=\"{}\" Clsid=\"{}\" />\n",
                verb.id, verb.clsid
            )
        })
        .collect();

    let folder_verbs: String = zipmount_shell::VERBS
        .iter()
        .filter(|verb| matches!(verb.target, zipmount_shell::Target::Folder))
        .map(|verb| {
            format!(
                "              <desktop5:Verb Id=\"{}\" Clsid=\"{}\" />\n",
                verb.id, verb.clsid
            )
        })
        .collect();

    let mut item_types = String::new();
    for extension in shell::extensions() {
        item_types.push_str(&format!(
            "            <desktop5:ItemType Type=\"{extension}\">\n{archive_verbs}            </desktop5:ItemType>\n"
        ));
    }
    // Unmounting is declared on a folder in both senses: a selected one
    // (`Directory`) and empty space inside an open one (`Directory\Background`).
    // A drive is not among the targets the manifest allows, and `Directory` is
    // the closest thing that can reach it. It does no harm: on other folders
    // the handler hides the item itself by checking the volume's filesystem.
    for target in ["Directory", "Directory\\Background"] {
        item_types.push_str(&format!(
            "            <desktop5:ItemType Type=\"{target}\">\n{folder_verbs}            </desktop5:ItemType>\n"
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Package
  xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  xmlns:com="http://schemas.microsoft.com/appx/manifest/com/windows10"
  xmlns:desktop4="http://schemas.microsoft.com/appx/manifest/desktop/windows10/4"
  xmlns:desktop5="http://schemas.microsoft.com/appx/manifest/desktop/windows10/5"
  xmlns:desktop6="http://schemas.microsoft.com/appx/manifest/desktop/windows10/6"
  IgnorableNamespaces="uap uap10 rescap com desktop4 desktop5 desktop6">

  <Identity Name="{PACKAGE_NAME}" Publisher="{PUBLISHER}" Version="1.0.0.0" ProcessorArchitecture="x64" />

  <Properties>
    <DisplayName>ZipMount</DisplayName>
    <PublisherDisplayName>ZipMount</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
    <uap10:AllowExternalContent>true</uap10:AllowExternalContent>
    <!-- The package is only a way to get identity for the menu. Everything
         else must behave like an ordinary program, so write redirection to
         the registry and files is off: otherwise the mount state written by a
         process under the package would land in the package's private storage
         and never be found from the command line. -->
    <desktop6:RegistryWriteVirtualization>disabled</desktop6:RegistryWriteVirtualization>
    <desktop6:FileSystemWriteVirtualization>disabled</desktop6:FileSystemWriteVirtualization>
  </Properties>

  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.19041.0" MaxVersionTested="10.0.26100.0" />
  </Dependencies>

  <!-- Menu texts are not package resources: the handler produces them at run
       time, in the language chosen with `zipmount language`. -->
  <Resources>
    <Resource Language="en-US" />
  </Resources>

  <Capabilities>
    <rescap:Capability Name="runFullTrust" />
    <rescap:Capability Name="unvirtualizedResources" />
  </Capabilities>

  <Applications>
    <Application Id="ZipMount" Executable="zipmount.exe" EntryPoint="Windows.FullTrustApplication" uap10:TrustLevel="mediumIL">
      <uap:VisualElements
        DisplayName="ZipMount"
        Description="Mounts archives as Windows drives"
        BackgroundColor="transparent"
        Square150x150Logo="Assets\Square150x150Logo.png"
        Square44x44Logo="Assets\Square44x44Logo.png"
        AppListEntry="none" />
      <Extensions>
        <com:Extension Category="windows.comServer">
          <com:ComServer>
            <com:SurrogateServer DisplayName="ZipMount">
{classes}            </com:SurrogateServer>
          </com:ComServer>
        </com:Extension>
        <desktop4:Extension Category="windows.fileExplorerContextMenus">
          <desktop4:FileExplorerContextMenus>
{item_types}          </desktop4:FileExplorerContextMenus>
        </desktop4:Extension>
      </Extensions>
    </Application>
  </Applications>
</Package>
"#
    )
}

/// Puts `zipmount.exe` into the install directory.
///
/// The copy may be busy — a mounted drive may be running from it right now.
/// A running file cannot be overwritten but can be renamed, which is what we
/// do.
fn place_exe(source: &Path, target: &Path) -> Result<()> {
    if source == target {
        return Ok(());
    }
    if std::fs::copy(source, target).is_ok() {
        return Ok(());
    }

    let retired = target.with_extension("exe.old");
    let _ = std::fs::remove_file(&retired);
    std::fs::rename(target, &retired)
        .with_context(|| t!("err-exe-busy", path = target.display().to_string()))?;
    std::fs::copy(source, target)
        .with_context(|| t!("err-write", path = target.display().to_string()))?;
    Ok(())
}

/// A built and signed package.
pub struct Package {
    pub msix: PathBuf,
    pub certificate: PathBuf,
    thumbprint: String,
}

/// Builds and signs a package that will look for the library `dll_name` in
/// the external content directory.
///
/// Building is separate from installing not for looks: it needs the Windows
/// SDK, and the machine the installer delivers the program to has no SDK and
/// should not. So the package is built where the program is built, and the
/// installer carries the ready signed file.
pub fn build_package(dll_name: &str, out: &Path) -> Result<Package> {
    let content = out.join("content");
    let _ = std::fs::remove_dir_all(&content);
    std::fs::create_dir_all(content.join("Assets"))
        .with_context(|| t!("err-create-dir", path = content.display().to_string()))?;

    let manifest_path = content.join("AppxManifest.xml");
    std::fs::write(&manifest_path, manifest(dll_name))
        .with_context(|| t!("err-write", path = manifest_path.display().to_string()))?;
    for (name, bytes) in ASSETS {
        std::fs::write(content.join("Assets").join(name), bytes)?;
    }

    let msix = out.join("ZipMount.msix");
    let makeappx = sdk_tool("makeappx.exe")?;
    run_tool(
        &makeappx,
        &[
            "pack",
            "/d",
            &content.display().to_string(),
            "/p",
            &msix.display().to_string(),
            // Validation is skipped on purpose: a sparse package contains
            // neither the exe nor the dll, and ordinary validation complains.
            "/nv",
            "/o",
        ],
    )?;

    let certificate = out.join("ZipMount.cer");
    let thumbprint = ensure_certificate(&certificate)?;
    let signtool = sdk_tool("signtool.exe")?;
    run_tool(
        &signtool,
        &[
            "sign",
            "/fd",
            "SHA256",
            "/sha1",
            &thumbprint,
            &msix.display().to_string(),
        ],
    )?;

    let _ = std::fs::remove_dir_all(&content);
    Ok(Package {
        msix,
        certificate,
        thumbprint,
    })
}

/// Installs a built package, showing it the directory with the program.
///
/// The package and nothing else: the registry item on the drive itself is
/// added by whoever knows where to write it. For a manual install that is
/// HKCU; for the installer it is HKLM, which the installer writes itself.
pub fn register(msix: &Path, external: &Path) -> Result<()> {
    powershell(&format!(
        "$ErrorActionPreference = 'Stop'\n\
         Get-AppxPackage -Name '{PACKAGE_NAME}' | Remove-AppxPackage\n\
         Add-AppxPackage -Path '{}' -ExternalLocation '{}'",
        msix.display(),
        external.display()
    ))?;
    Ok(())
}

/// Removes the package. Prints nothing.
pub fn unregister() -> Result<()> {
    if is_installed() {
        powershell(&format!(
            "$ErrorActionPreference = 'Stop'\n\
             Get-AppxPackage -Name '{PACKAGE_NAME}' | Remove-AppxPackage"
        ))?;
    }
    Ok(())
}

/// Adds the certificate to the trusted ones on this machine.
///
/// Asks for no rights: it is called either from an already elevated process
/// or from the installer, which runs with administrator rights anyway.
pub fn trust_here(certificate: &Path) -> Result<()> {
    powershell(&format!(
        "$ErrorActionPreference = 'Stop'\n\
         Import-Certificate -FilePath '{}' \
         -CertStoreLocation Cert:\\LocalMachine\\TrustedPeople | Out-Null",
        certificate.display()
    ))?;
    Ok(())
}

/// Removes our certificate from the trusted ones. Also needs rights.
pub fn untrust_here() -> Result<()> {
    powershell(&format!(
        "Get-ChildItem Cert:\\LocalMachine\\TrustedPeople -ErrorAction SilentlyContinue |\n\
         Where-Object {{ $_.Subject -eq '{PUBLISHER}' }} | Remove-Item -Force"
    ))?;
    Ok(())
}

pub fn install() -> Result<()> {
    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    let build_dir = exe
        .parent()
        .context("cannot determine the program's directory")?;
    let dll_source = build_dir.join("zipmount_shell.dll");
    if !dll_source.is_file() {
        bail!(t!("err-shell-dll-missing"));
    }

    let bin = bin_dir()?;
    let staging = staging_dir();
    std::fs::create_dir_all(&bin)
        .with_context(|| t!("err-bin-dir", path = bin.display().to_string()))?;

    // --- the files the package will point at ---
    let dll_bytes = std::fs::read(&dll_source)?;
    let dll_name = format!("zipmount_shell_{}.dll", fingerprint(&dll_bytes));
    let dll_target = bin.join(&dll_name);
    if !dll_target.exists() {
        std::fs::write(&dll_target, &dll_bytes)
            .with_context(|| t!("err-write", path = dll_target.display().to_string()))?;
    }
    place_exe(&exe, &bin.join("zipmount.exe"))?;
    std::fs::write(bin.join("ZipMount.ico"), ICON)?;
    drop_stale_libraries(&bin, &dll_name);

    // --- build and sign ---
    println!("{}", t!("modern-building"));
    let package = build_package(&dll_name, &staging)?;

    // The certificate outlives the temporary directory: it stays next to the
    // program, so that it is visible and there is something to delete.
    let certificate = root()?.join("ZipMount.cer");
    std::fs::copy(&package.certificate, &certificate)?;

    if !is_trusted(&package.thumbprint)? {
        println!();
        println!("{}", t!("modern-trust-intro"));
        println!("  {}", certificate.display());
        println!();
        println!("{}", t!("modern-trust-uac"));
        ask_to_trust(&certificate)?;

        if !is_trusted(&package.thumbprint)? {
            bail!(t!("err-cert-not-trusted"));
        }
    }

    println!("{}", t!("modern-registering"));
    let registered = register(&package.msix, &bin);
    // The package is with the system now — the temporary files are no longer
    // needed, even if installation failed.
    let _ = std::fs::remove_dir_all(&staging);
    registered?;

    // Archive items now come from the package; keeping them in the registry
    // as well would show the same thing twice at different menu levels. But
    // unmounting with a click on the drive itself the package cannot declare —
    // only the registry can.
    shell::install_drive_only(&bin.join("zipmount.exe"))?;

    println!();
    println!("{}", t!("modern-done"));
    println!();
    println!(
        "{}",
        t!("modern-installed-to", path = bin.display().to_string())
    );
    println!("{}", t!("modern-rebuild-hint"));
    println!();
    println!("{}", t!("shell-remove-hint"));
    Ok(())
}

/// Removes libraries left by earlier installs.
///
/// One held by File Explorer cannot be deleted — that is fine, it bothers no
/// one and goes away on the next attempt.
fn drop_stale_libraries(bin: &Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(bin) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("zipmount_shell_") && name != keep {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Finds or creates the self-signed certificate; returns its thumbprint.
fn ensure_certificate(export_to: &Path) -> Result<String> {
    let thumbprint = powershell(&format!(
        "$ErrorActionPreference = 'Stop'\n\
         $subject = '{PUBLISHER}'\n\
         $cert = Get-ChildItem Cert:\\CurrentUser\\My |\n\
             Where-Object {{ $_.Subject -eq $subject -and $_.NotAfter -gt (Get-Date) }} |\n\
             Select-Object -First 1\n\
         if (-not $cert) {{\n\
             $cert = New-SelfSignedCertificate -Type Custom -Subject $subject \
                 -KeyUsage DigitalSignature -FriendlyName 'ZipMount' \
                 -CertStoreLocation 'Cert:\\CurrentUser\\My' \
                 -NotAfter (Get-Date).AddYears(5) \
                 -TextExtension @('2.5.29.37={{text}}1.3.6.1.5.5.7.3.3')\n\
         }}\n\
         Export-Certificate -Cert $cert -FilePath '{}' -Force | Out-Null\n\
         $cert.Thumbprint",
        export_to.display()
    ))?;

    if thumbprint.len() != 40 {
        bail!("could not get the certificate thumbprint: {thumbprint:?}");
    }
    Ok(thumbprint)
}

fn is_trusted(thumbprint: &str) -> Result<bool> {
    let answer = powershell(&format!(
        "if (Get-ChildItem Cert:\\LocalMachine\\TrustedPeople -ErrorAction SilentlyContinue |\n\
             Where-Object {{ $_.Thumbprint -eq '{thumbprint}' }}) {{ 'yes' }} else {{ 'no' }}"
    ))?;
    Ok(answer == "yes")
}

/// Asks for administrator rights and adds the certificate to the trusted ones.
fn ask_to_trust(certificate: &Path) -> Result<()> {
    let inner = format!(
        "Import-Certificate -FilePath '{}' -CertStoreLocation Cert:\\LocalMachine\\TrustedPeople | Out-Null",
        certificate.display()
    );
    // Declining elevation is not an execution error but the user's decision;
    // it is dealt with by checking the result, not the exit code.
    let _ = powershell(&format!(
        "Start-Process -FilePath powershell -Verb RunAs -Wait -ArgumentList \
         '-NoProfile','-NonInteractive','-Command',\"{inner}\""
    ));
    Ok(())
}

/// Whether the modern menu package is installed.
pub fn is_installed() -> bool {
    powershell(&format!(
        "if (Get-AppxPackage -Name '{PACKAGE_NAME}') {{ 'yes' }} else {{ 'no' }}"
    ))
    .map(|answer| answer == "yes")
    .unwrap_or(false)
}

/// Asks the registered handler for the title of the first item.
///
/// An installed package is not yet a working menu: between registration and
/// a drawn row lie creating the COM object and File Explorer loading the
/// library. This checks the whole chain except the drawing, and a failure
/// becomes visible at once rather than as an item that silently never shows.
pub fn probe_handler() -> Result<String> {
    let verb = zipmount_shell::VERBS
        .first()
        .context("the verb table is empty")?;
    let clsid = verb.guid().context("damaged class id")?;

    // SAFETY: initializes COM for the current thread; a repeated call with
    // the same mode is safe.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;

    // SAFETY: the class id is valid, no aggregating object is needed.
    //
    // The context is CLSCTX_ALL on purpose: the manifest declares the class
    // through a SurrogateServer, and asking for an in-process server only
    // returns "class not registered" although all is well with it.
    let command: IExplorerCommand = unsafe { CoCreateInstance(&clsid, None, CLSCTX_ALL) }
        .with_context(|| t!("err-handler-create"))?;

    // SAFETY: with no selection the title does not depend on it; the handler
    // allocates the string with CoTaskMemAlloc, freed below.
    let title = unsafe { command.GetTitle(None) }.with_context(|| t!("err-handler-title"))?;
    let text = unsafe { title.to_string() }.with_context(|| t!("err-handler-title"))?;
    unsafe { CoTaskMemFree(Some(title.0 as *const c_void)) };
    Ok(text)
}

pub fn uninstall() -> Result<()> {
    let had_package = is_installed();
    unregister()?;
    shell::uninstall()?;
    println!("{}", t!("shell-removed"));

    if !had_package {
        // The modern menu was never installed — stay quiet rather than scare
        // someone who never used it with a story about certificates.
        return Ok(());
    }

    println!();
    println!(
        "{}",
        t!("modern-files-left", path = bin_dir()?.display().to_string())
    );
    println!("{}", t!("modern-cert-left"));
    println!("  Get-ChildItem Cert:\\LocalMachine\\TrustedPeople |");
    println!("    Where-Object Subject -eq '{PUBLISHER}' | Remove-Item");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_every_verb_for_every_extension() {
        let xml = manifest("zipmount_shell_test.dll");
        for extension in shell::extensions() {
            assert!(
                xml.contains(&format!("Type=\"{extension}\"")),
                "extension {extension} is missing from the manifest"
            );
        }
        for verb in zipmount_shell::VERBS {
            assert!(
                xml.contains(&format!("Id=\"{}\"", verb.id)),
                "verb {} is missing from the manifest",
                verb.id
            );
            assert!(
                xml.contains(verb.clsid),
                "class {} is missing from the manifest",
                verb.clsid
            );
        }
    }

    #[test]
    fn manifest_points_at_the_versioned_library() {
        // The DLL's name changes from build to build, and the manifest must
        // point at the one just put in place.
        let xml = manifest("zipmount_shell_deadbeef.dll");
        assert!(xml.contains("Path=\"zipmount_shell_deadbeef.dll\""));
        assert!(!xml.contains("Path=\"zipmount_shell.dll\""));
    }

    #[test]
    fn unmount_lives_on_both_kinds_of_folder() {
        // A drive is not among the targets the manifest schema allows, so
        // unmounting is declared on a folder — both a selected one and the
        // background of an open one. Losing either is easy, and noticing the
        // loss takes a right-click in exactly the right place.
        let xml = manifest("x.dll");
        for target in ["Type=\"Directory\">", "Type=\"Directory\\Background\">"] {
            let section = xml.split(target).nth(1).expect("folder section");
            let section = section.split("</desktop5:ItemType>").next().unwrap_or("");
            assert!(
                section.contains("Id=\"Unmount\""),
                "unmount is not declared for {target}"
            );
        }
    }

    #[test]
    fn fingerprint_separates_different_builds() {
        assert_ne!(fingerprint(b"one"), fingerprint(b"two"));
        assert_eq!(fingerprint(b"same"), fingerprint(b"same"));
        assert_eq!(fingerprint(b"any").len(), 16);
    }
}
