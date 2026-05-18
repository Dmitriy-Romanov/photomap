use std::process::Command;

/// Select multiple folders using native OS dialogs (max 5).
/// Returns a vector of selected folder paths (deduplicated).
pub fn select_folders_native() -> Vec<String> {
    let folders = match std::env::consts::OS {
        "macos" => select_folders_macos(),
        "windows" => select_folders_windows(),
        "linux" => select_folders_linux(),
        _ => Vec::new(),
    };

    deduplicate(folders)
}

fn select_folders_macos() -> Vec<String> {
    let script = r#"
set folderList to choose folder with prompt "Select photo folders (max 5, Cmd+Click for multiple)" with multiple selections allowed
set pathList to {}
repeat with aFolder in folderList
    set end of pathList to POSIX path of aFolder
end repeat
return pathList
"#;

    let Ok(output) = Command::new("osascript").arg("-e").arg(script).output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .split(", ")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(5)
        .collect()
}

fn select_folders_windows() -> Vec<String> {
    let script = r#"
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
        $code = @'
        using System;
        using System.Runtime.InteropServices;
        using System.Collections.Generic;

        namespace Win32 {
            [ComImport, Guid("DC1C5A9C-E88A-4dde-A5A1-60F82A20AEF7")]
            class FileOpenDialog { }

            [ComImport, Guid("d57c7288-d4ad-4768-be02-9d969532d960"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IFileOpenDialog {
                void Show(IntPtr parent);
                void SetFileTypes();
                void SetFileTypeIndex();
                void GetFileTypeIndex();
                void Advise();
                void Unadvise();
                void SetOptions(uint fos);
                void GetOptions();
                void SetDefaultFolder();
                void SetFolder(IntPtr psi);
                void GetFolder();
                void GetCurrentSelection();
                void SetFileName();
                void GetFileName();
                void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string title);
                void SetOkButtonLabel();
                void SetFileNameLabel();
                void GetResult();
                void AddPlace();
                void SetDefaultExtension();
                void Close();
                void SetClientGuid();
                void ClearClientData();
                void SetFilter();
                void GetResults(out IShellItemArray ppenum);
            }

            [ComImport, Guid("b63ea76d-1f85-456f-a19c-48159efa858b"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IShellItemArray {
                void BindToHandler();
                void GetPropertyStore();
                void GetPropertyDescriptionList();
                void GetAttributes();
                void GetCount(out uint pdwNumItems);
                void GetItemAt(uint dwIndex, out IShellItem ppsi);
            }

            [ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IShellItem {
                void BindToHandler();
                void GetParent();
                void GetDisplayName(uint sigdnName, out IntPtr ppszName);
                void GetAttributes();
                void Compare();
            }

            public class Dialog {
                [DllImport("user32.dll")]
                private static extern IntPtr GetForegroundWindow();

                public static string[] Show() {
                    var dialog = (IFileOpenDialog)new FileOpenDialog();
                    dialog.SetOptions(0x260);
                    dialog.SetTitle("Select photo folders (Ctrl+Click for multiple)");

                    try {
                        IntPtr hwnd = GetForegroundWindow();
                        dialog.Show(hwnd);

                        IShellItemArray results;
                        dialog.GetResults(out results);

                        uint count;
                        results.GetCount(out count);

                        var paths = new List<string>();
                        for (uint i = 0; i < count; i++) {
                            IShellItem item;
                            results.GetItemAt(i, out item);
                            IntPtr namePtr;
                            item.GetDisplayName(0x80058000, out namePtr);
                            paths.Add(Marshal.PtrToStringAuto(namePtr));
                            Marshal.FreeCoTaskMem(namePtr);
                        }
                        return paths.ToArray();
                    } catch {
                        return null;
                    }
                }
            }
        }
'@

        Add-Type -TypeDefinition $code
        [Win32.Dialog]::Show()
    "#;

    let Ok(output) = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(script)
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(5)
        .collect()
}

fn select_folders_linux() -> Vec<String> {
    let Ok(output) = Command::new("zenity")
        .arg("--file-selection")
        .arg("--directory")
        .arg("--multiple")
        .arg("--separator=|")
        .arg("--title=Select photo folders (max 5)")
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .take(5)
        .collect()
}

fn deduplicate(folders: Vec<String>) -> Vec<String> {
    let mut unique_folders = Vec::new();
    for folder in folders {
        if !unique_folders.contains(&folder) {
            unique_folders.push(folder);
        }
    }
    unique_folders
}
