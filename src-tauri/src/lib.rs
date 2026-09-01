use serde::Serialize;
use std::{fs, path::Path, time::UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FilePreview {
    name: String,
    path: String,
    extension: String,
    size: u64,
    modified_time: u64,
    category: &'static str,
    proposed_destination: String,
    will_move: bool,
}

fn classify_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "svg" | "tif" | "tiff" | "heic"
        | "avif" => "Images",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "csv" | "md"
        | "odt" | "ods" | "odp" | "epub" => "Documents",
        "mp4" | "mkv" | "mov" | "avi" | "wmv" | "webm" | "m4v" | "flv" => "Videos",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" => "Audio",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "tgz" => "Archives",
        "exe" | "msi" | "msix" | "dmg" | "pkg" | "deb" | "rpm" | "appimage" => "Installers",
        _ => "Other",
    }
}

fn category_folder(category: &str) -> &'static str {
    match category {
        "Images" => "Photos and Images",
        "Documents" => "Documents and Text",
        "Videos" => "Movies and Videos",
        "Audio" => "Music and Audio",
        "Archives" => "Compressed Archives",
        "Installers" => "Apps and Installers",
        _ => "Miscellaneous Files",
    }
}

fn should_move(category: &str, aggressiveness: u8) -> bool {
    match aggressiveness {
        1 => matches!(category, "Images" | "Documents" | "Videos" | "Audio"),
        2 => category != "Other",
        3 => true,
        _ => false,
    }
}

#[tauri::command]
fn scan_directory(directory: String, aggressiveness: u8) -> Result<Vec<FilePreview>, String> {
    if !(1..=3).contains(&aggressiveness) {
        return Err("Sorting aggressiveness must be between 1 and 3.".to_string());
    }
    let directory_path = Path::new(&directory);
    if !directory_path.is_dir() {
        return Err("The selected path is not a directory.".to_string());
    }

    let entries = fs::read_dir(directory_path)
        .map_err(|error| format!("Could not read the selected directory: {error}"))?;
    let mut previews = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|error| format!("Could not read a directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect a directory entry: {error}"))?;
        if !file_type.is_file() {
            continue;
        }

        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Could not read metadata for {}: {error}", path.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let category = classify_extension(&extension);
        let will_move = should_move(category, aggressiveness);
        let modified_time = metadata
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let proposed_destination = if will_move {
            directory_path
                .join("Organized Files")
                .join(category_folder(category))
                .join(&name)
                .to_string_lossy()
                .into_owned()
        } else {
            path.to_string_lossy().into_owned()
        };

        previews.push(FilePreview {
            name,
            path: path.to_string_lossy().into_owned(),
            extension,
            size: metadata.len(),
            modified_time,
            category,
            proposed_destination,
            will_move,
        });
    }

    previews.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(previews)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![scan_directory])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{category_folder, classify_extension, should_move};

    #[test]
    fn classifies_supported_extensions_case_insensitively() {
        let cases = [
            ("JPG", "Images"),
            ("pdf", "Documents"),
            ("MKV", "Videos"),
            ("flac", "Audio"),
            ("7Z", "Archives"),
            ("msi", "Installers"),
        ];

        for (extension, expected) in cases {
            assert_eq!(classify_extension(extension), expected);
        }
    }

    #[test]
    fn classifies_unknown_and_missing_extensions_as_other() {
        assert_eq!(classify_extension("xyz"), "Other");
        assert_eq!(classify_extension(""), "Other");
    }

    #[test]
    fn aggressiveness_controls_which_categories_move() {
        assert!(!should_move("Archives", 1));
        assert!(should_move("Archives", 2));
        assert!(!should_move("Other", 2));
        assert!(should_move("Other", 3));
    }

    #[test]
    fn uses_descriptive_destination_folders() {
        assert_eq!(category_folder("Archives"), "Compressed Archives");
        assert_eq!(category_folder("Installers"), "Apps and Installers");
        assert_eq!(category_folder("Other"), "Miscellaneous Files");
    }
}
