//! The file manager's context menu on Linux and macOS.
//!
//! Nothing here needs signing or administrator rights: every item is a file
//! in the user's home directory that runs `zipmount mount --detach --open`.
//!
//! * macOS: a Quick Action (an Automator service) in `~/Library/Services`,
//!   offered by Finder for archives only — right-click → Quick Actions.
//! * Linux: an "Open With" application entry, which every file manager
//!   honours; a Nautilus script (GNOME Files: right-click → Scripts); and a
//!   Dolphin service menu (KDE, straight in the context menu). Nautilus
//!   scripts cannot be limited to a file type, the other two are.
//!
//! The item's text is written in the language of the moment, so
//! `zipmount language` writes the items again.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use zipmount_i18n::t;

use super::NOTIFY_VAR;

/// MIME types of the archives we open (Linux).
#[cfg(target_os = "linux")]
const MIME_TYPES: &[&str] = &[
    "application/zip",
    "application/x-7z-compressed",
    "application/x-tar",
    "application/x-compressed-tar",
    "application/gzip",
    "application/x-gzip",
];

/// Uniform type identifiers of the archives we open (macOS).
#[cfg(target_os = "macos")]
const UTIS: &[&str] = &[
    "public.zip-archive",
    "org.7-zip.7-zip-archive",
    "public.tar-archive",
    "org.gnu.gnu-zip-archive",
    "org.gnu.gnu-zip-tar-archive",
];

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

/// The path the menu should run: the one on PATH if that is this program
/// (Homebrew's /opt/homebrew/bin/zipmount survives upgrades, the versioned
/// Cellar path it points to does not), else this program's own path.
fn program() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("cannot determine the program's path")?;
    let real = std::fs::canonicalize(&exe).unwrap_or_else(|_| exe.clone());
    let on_path = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join("zipmount"))
            .find(|candidate| std::fs::canonicalize(candidate).is_ok_and(|c| c == real))
    });
    Ok(on_path.unwrap_or(exe))
}

/// A path quoted for a POSIX shell: in single quotes, with any single quote
/// written as '\''.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', r"'\''"))
}

/// Writes a file, creating its directory; optionally executable.
fn write(path: &Path, text: &str, executable: bool) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| t!("err-create-dir", path = dir.display().to_string()))?;
    }
    std::fs::write(path, text)
        .with_context(|| t!("err-write", path = path.display().to_string()))?;
    if executable {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| t!("err-write", path = path.display().to_string()))?;
    }
    Ok(())
}

/// The item's text in the current language.
fn label() -> String {
    t!("menu-mount-unix")
}

/// Installs the items; returns where they went.
pub(crate) fn install() -> Result<Vec<PathBuf>> {
    // Items written under an earlier label (another language) would stay
    // behind next to the new ones.
    uninstall()?;
    platform::install(&home()?, &program()?, &label())
}

/// Removes every item this program installed; returns how many there were.
pub(crate) fn uninstall() -> Result<usize> {
    platform::uninstall(&home()?)
}

/// Whether any item is installed.
pub(crate) fn is_installed() -> bool {
    home().is_ok_and(|home| !platform::installed(&home).is_empty())
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    /// A marker in every file we write, so that uninstalling finds exactly
    /// ours, whatever language their names came out in.
    const MARK: &str = "X-ZipMount-Item=true";

    fn applications(home: &Path) -> PathBuf {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        data.join("applications")
    }

    fn data_dir(home: &Path) -> PathBuf {
        applications(home)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| home.join(".local/share"))
    }

    /// Exec= arguments are quoted by the desktop entry rules: double
    /// quotes, with ", `, $ and \ escaped.
    fn desktop_quote(path: &Path) -> String {
        let mut out = String::from("\"");
        for c in path.to_string_lossy().chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    }

    pub(super) fn install(home: &Path, program: &Path, label: &str) -> Result<Vec<PathBuf>> {
        let exec = format!(
            "env {NOTIFY_VAR}=1 {} mount --detach --open %f",
            desktop_quote(program)
        );
        let mimes = MIME_TYPES.join(";") + ";";
        let mut written = Vec::new();

        // "Open With": every file manager offers the applications that
        // declare the file's type. NoDisplay keeps it out of the app menu.
        let app = applications(home).join("zipmount-mount.desktop");
        write(
            &app,
            &format!(
                "[Desktop Entry]\nType=Application\nName={label}\nIcon=drive-removable-media\n\
                 Exec={exec}\nMimeType={mimes}\nNoDisplay=true\nTerminal=false\n{MARK}\n"
            ),
            false,
        )?;
        written.push(app);
        // Refreshes the "Open With" cache; the entry works without it too,
        // only later.
        let _ = std::process::Command::new("update-desktop-database")
            .arg(applications(home))
            .output();

        // Dolphin: a service menu, straight in the context menu. KDE 6 looks
        // in kio/servicemenus and wants it executable; KDE 5 in
        // kservices5/ServiceMenus.
        let service = format!(
            "[Desktop Entry]\nType=Service\nX-KDE-ServiceTypes=KonqPopupMenu/Plugin\n\
             MimeType={mimes}\nActions=mount;\nX-KDE-Priority=TopLevel\n{MARK}\n\n\
             [Desktop Action mount]\nName={label}\nIcon=drive-removable-media\nExec={exec}\n"
        );
        for dir in ["kio/servicemenus", "kservices5/ServiceMenus"] {
            let path = data_dir(home).join(dir).join("zipmount.desktop");
            write(&path, &service, true)?;
            written.push(path);
        }

        // Nautilus: a script, named by its menu text, gets the selected
        // files as arguments, in their directory.
        let script = home
            .join(".local/share/nautilus/scripts")
            .join(label.replace('/', "-"));
        write(
            &script,
            &format!(
                "#!/bin/sh\n# {MARK}\nexport {NOTIFY_VAR}=1\nfor f in \"$@\"; do\n    {} mount --detach --open \"$f\"\ndone\n",
                shell_quote(program)
            ),
            true,
        )?;
        written.push(script);
        Ok(written)
    }

    pub(super) fn installed(home: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let candidates = [
            applications(home).join("zipmount-mount.desktop"),
            data_dir(home).join("kio/servicemenus/zipmount.desktop"),
            data_dir(home).join("kservices5/ServiceMenus/zipmount.desktop"),
        ];
        found.extend(candidates.into_iter().filter(|p| is_ours(p)));
        if let Ok(entries) = std::fs::read_dir(home.join(".local/share/nautilus/scripts")) {
            found.extend(entries.flatten().map(|e| e.path()).filter(|p| is_ours(p)));
        }
        found
    }

    fn is_ours(path: &Path) -> bool {
        std::fs::read_to_string(path).is_ok_and(|text| text.contains(MARK))
    }

    pub(super) fn uninstall(home: &Path) -> Result<usize> {
        let found = installed(home);
        for path in &found {
            std::fs::remove_file(path)
                .with_context(|| t!("err-write", path = path.display().to_string()))?;
        }
        Ok(found.len())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    /// A marker in every workflow we write, so that uninstalling finds
    /// exactly ours, whatever language their names came out in.
    const MARK: &str = "ZipMountItem";

    fn services(home: &Path) -> PathBuf {
        home.join("Library/Services")
    }

    fn xml_escape(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    pub(super) fn install(home: &Path, program: &Path, label: &str) -> Result<Vec<PathBuf>> {
        let bundle = services(home).join(format!("{}.workflow", label.replace('/', "-")));
        let contents = bundle.join("Contents");

        let types: String = UTIS
            .iter()
            .map(|uti| format!("\t\t\t\t<string>{uti}</string>\n"))
            .collect();
        let info = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>{MARK}</key>
	<true/>
	<key>NSServices</key>
	<array>
		<dict>
			<key>NSMenuItem</key>
			<dict>
				<key>default</key>
				<string>{label}</string>
			</dict>
			<key>NSMessage</key>
			<string>runWorkflowAsService</string>
			<key>NSRequiredContext</key>
			<dict>
				<key>NSApplicationIdentifier</key>
				<string>com.apple.finder</string>
			</dict>
			<key>NSSendFileTypes</key>
			<array>
{types}			</array>
		</dict>
	</array>
</dict>
</plist>
"#,
            label = xml_escape(label)
        );

        // `|| true`: zipmount shows its own error; a failing script would
        // make Automator add a second, vaguer dialog.
        let script = format!(
            "export {NOTIFY_VAR}=1\nfor f in \"$@\"; do\n\t{} mount --detach --open \"$f\" || true\ndone",
            shell_quote(program)
        );
        let workflow = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>AMApplicationBuild</key>
	<string>528</string>
	<key>AMApplicationVersion</key>
	<string>2.10</string>
	<key>AMDocumentVersion</key>
	<string>2</string>
	<key>actions</key>
	<array>
		<dict>
			<key>action</key>
			<dict>
				<key>AMAccepts</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Optional</key>
					<true/>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.path</string>
					</array>
				</dict>
				<key>AMActionVersion</key>
				<string>2.0.3</string>
				<key>AMApplication</key>
				<array>
					<string>Automator</string>
				</array>
				<key>AMParameterProperties</key>
				<dict>
					<key>COMMAND_STRING</key>
					<dict/>
					<key>CheckedForUserDefaultShell</key>
					<dict/>
					<key>inputMethod</key>
					<dict/>
					<key>shell</key>
					<dict/>
					<key>source</key>
					<dict/>
				</dict>
				<key>AMProvides</key>
				<dict>
					<key>Container</key>
					<string>List</string>
					<key>Types</key>
					<array>
						<string>com.apple.cocoa.string</string>
					</array>
				</dict>
				<key>ActionBundlePath</key>
				<string>/System/Library/Automator/Run Shell Script.action</string>
				<key>ActionName</key>
				<string>Run Shell Script</string>
				<key>ActionParameters</key>
				<dict>
					<key>COMMAND_STRING</key>
					<string>{script}</string>
					<key>CheckedForUserDefaultShell</key>
					<true/>
					<key>inputMethod</key>
					<integer>1</integer>
					<key>shell</key>
					<string>/bin/zsh</string>
					<key>source</key>
					<string></string>
				</dict>
				<key>BundleIdentifier</key>
				<string>com.apple.RunShellScript</string>
				<key>CFBundleVersion</key>
				<string>2.0.3</string>
				<key>CanShowSelectedItemsWhenRun</key>
				<false/>
				<key>CanShowWhenRun</key>
				<true/>
				<key>Category</key>
				<array>
					<string>AMCategoryUtilities</string>
				</array>
				<key>Class Name</key>
				<string>RunShellScriptAction</string>
				<key>InputUUID</key>
				<string>5B1F0A6E-4C1B-4E0A-9C3E-2A7D6B8E1F01</string>
				<key>Keywords</key>
				<array>
					<string>Shell</string>
					<string>Script</string>
				</array>
				<key>OutputUUID</key>
				<string>5B1F0A6E-4C1B-4E0A-9C3E-2A7D6B8E1F02</string>
				<key>UUID</key>
				<string>5B1F0A6E-4C1B-4E0A-9C3E-2A7D6B8E1F03</string>
				<key>UnlocalizedApplications</key>
				<array>
					<string>Automator</string>
				</array>
				<key>arguments</key>
				<dict/>
				<key>isViewVisible</key>
				<integer>1</integer>
				<key>location</key>
				<string>300.000000:300.000000</string>
			</dict>
			<key>isViewVisible</key>
			<integer>1</integer>
		</dict>
	</array>
	<key>connectors</key>
	<dict/>
	<key>workflowMetaData</key>
	<dict>
		<key>applicationBundleID</key>
		<string>com.apple.finder</string>
		<key>applicationBundleIDsByPath</key>
		<dict/>
		<key>applicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>applicationPaths</key>
		<array/>
		<key>inputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>outputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>presentationMode</key>
		<integer>15</integer>
		<key>processesInput</key>
		<false/>
		<key>serviceApplicationBundleID</key>
		<string>com.apple.finder</string>
		<key>serviceApplicationPath</key>
		<string>/System/Library/CoreServices/Finder.app</string>
		<key>serviceInputTypeIdentifier</key>
		<string>com.apple.Automator.fileSystemObject</string>
		<key>serviceOutputTypeIdentifier</key>
		<string>com.apple.Automator.nothing</string>
		<key>serviceProcessesInput</key>
		<false/>
		<key>systemImageName</key>
		<string>NSActionTemplate</string>
		<key>useAutomaticInputType</key>
		<false/>
		<key>workflowTypeIdentifier</key>
		<string>com.apple.Automator.servicesMenu</string>
	</dict>
</dict>
</plist>
"#,
            script = xml_escape(&script)
        );

        write(&contents.join("Info.plist"), &info, false)?;
        write(&contents.join("document.wflow"), &workflow, false)?;
        // The services menu is rebuilt lazily; ask for it now so the item
        // shows up without logging out.
        let _ = std::process::Command::new("/System/Library/CoreServices/pbs")
            .arg("-update")
            .output();
        Ok(vec![bundle])
    }

    pub(super) fn installed(home: &Path) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(services(home)) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "workflow"))
            .filter(|p| {
                std::fs::read_to_string(p.join("Contents/Info.plist"))
                    .is_ok_and(|text| text.contains(MARK))
            })
            .collect()
    }

    pub(super) fn uninstall(home: &Path) -> Result<usize> {
        let found = installed(home);
        for path in &found {
            std::fs::remove_dir_all(path)
                .with_context(|| t!("err-write", path = path.display().to_string()))?;
        }
        if !found.is_empty() {
            let _ = std::process::Command::new("/System/Library/CoreServices/pbs")
                .arg("-update")
                .output();
        }
        Ok(found.len())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    use super::*;

    pub(super) fn install(_home: &Path, _program: &Path, _label: &str) -> Result<Vec<PathBuf>> {
        anyhow::bail!(t!("err-mount-unsupported"))
    }

    pub(super) fn installed(_home: &Path) -> Vec<PathBuf> {
        Vec::new()
    }

    pub(super) fn uninstall(_home: &Path) -> Result<usize> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quoting_survives_quotes_and_spaces() {
        assert_eq!(shell_quote(Path::new("/a b/zip")), "'/a b/zip'");
        assert_eq!(shell_quote(Path::new("/it's/zip")), r"'/it'\''s/zip'");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn items_install_and_uninstall_in_a_home_of_their_own() {
        let home = std::env::temp_dir().join(format!("zipmount-menu-test-{}", std::process::id()));
        let program = Path::new("/opt/zip mount/zipmount");
        let written = platform::install(&home, program, "Mount with ZipMount").unwrap();
        assert_eq!(written.len(), 4);
        assert_eq!(platform::installed(&home).len(), 4);

        let app = std::fs::read_to_string(&written[0]).unwrap();
        assert!(app.contains(
            "Exec=env ZIPMOUNT_NOTIFY=1 \"/opt/zip mount/zipmount\" mount --detach --open %f"
        ));
        assert!(app.contains("MimeType=application/zip;"));
        let script = std::fs::read_to_string(&written[3]).unwrap();
        assert!(script.contains("'/opt/zip mount/zipmount' mount --detach --open \"$f\""));

        assert_eq!(platform::uninstall(&home).unwrap(), 4);
        assert!(platform::installed(&home).is_empty());
        let _ = std::fs::remove_dir_all(&home);
    }
}
