//! Extension → category classification.
//!
//! Deliberately conservative: unknown extensions stay `Unknown` rather than
//! being guessed into a misleading bucket. The mapping is used for treemap
//! coloring, file-type rollups and recommendation ranking.

use crate::models::FileCategory;

/// Map an extension (without dot, lowercase) to a category.
pub fn classify(ext: &str) -> FileCategory {
    match ext {
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "mpg" | "mpeg"
        | "mts" | "m2ts" | "3gp" | "vob" | "ogv" => FileCategory::Video,
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" | "opus" | "aiff" | "alac"
        | "mid" | "midi" => FileCategory::Audio,
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "heic" | "heif"
        | "raw" | "cr2" | "cr3" | "nef" | "arw" | "dng" | "orf" | "rw2" | "psd" | "xcf"
        | "ai" | "svg" | "ico" => FileCategory::Image,
        // Documents
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "txt" | "rtf" | "md" | "csv" | "tsv" | "epub" | "mobi" | "djvu" | "xps" => {
            FileCategory::Document
        }
        // Archives
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "zst" | "lz4" | "cab" | "arj"
        | "lzh" | "ace" => FileCategory::Archive,
        // Disk images
        "iso" | "img" | "vhd" | "vhdx" | "vmdk" | "vdi" | "qcow2" | "bin" | "cue" | "dmg"
        | "wim" | "esd" => FileCategory::DiskImage,
        // Executables / installers
        "exe" | "msi" | "msp" | "bat" | "cmd" | "com" | "scr" | "appx" | "msix" | "apk" => {
            FileCategory::Executable
        }
        // Libraries
        "dll" | "so" | "dylib" | "a" | "lib" | "ocx" => FileCategory::Library,
        // Code
        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp"
        | "cc" | "cs" | "go" | "java" | "kt" | "swift" | "rb" | "php" | "lua" | "pl" | "sh"
        | "ps1" | "psm1" | "vb" | "html" | "css" | "scss" | "vue" | "svelte" | "sql" => {
            FileCategory::Code
        }
        // Databases
        "db" | "sqlite" | "sqlite3" | "db3" | "mdb" | "accdb" | "ldf" | "mdf" => {
            FileCategory::Database
        }
        // Logs
        "log" | "etl" | "evtx" | "dmp" | "mdmp" => FileCategory::Log,
        // System / Windows internals
        "sys" | "drv" | "inf" | "cat" | "mun" | "manifest" | "nls" | "ttf" | "otf" | "fon" => {
            FileCategory::System
        }
        // Developer ecosystems (caches, packages, builds)
        "pdb" | "ilk" | "obj" | "o" | "wasm" | "jar" | "class" | "pyc" | "pyd" | "whl" | "gem"
        | "nupkg" | "node" | "rlib" | "rmeta" | "crate" => FileCategory::Developer,
        _ => FileCategory::Unknown,
    }
}

/// Categorize by extension with well-known full-name overrides.
/// Extension extraction is string-based so Windows paths classify correctly
/// on any host.
pub fn classify_path(path: &std::path::Path) -> FileCategory {
    let lower_full = path.to_string_lossy().to_ascii_lowercase();
    if lower_full.ends_with("pagefile.sys")
        || lower_full.ends_with("hiberfil.sys")
        || lower_full.ends_with("swapfile.sys")
    {
        return FileCategory::System;
    }
    let name = crate::util::paths::name_of(&lower_full);
    let ext = match name.rfind('.') {
        Some(idx) => &name[idx + 1..],
        None => "",
    };
    classify(ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions_map() {
        assert_eq!(classify("mp4"), FileCategory::Video);
        assert_eq!(classify("mkv"), FileCategory::Video);
        assert_eq!(classify("zip"), FileCategory::Archive);
        assert_eq!(classify("iso"), FileCategory::DiskImage);
        assert_eq!(classify("vhd"), FileCategory::DiskImage);
        assert_eq!(classify("vhdx"), FileCategory::DiskImage);
        assert_eq!(classify("dll"), FileCategory::Library);
        assert_eq!(classify("exe"), FileCategory::Executable);
        assert_eq!(classify("dmp"), FileCategory::Log);
    }

    #[test]
    fn unknown_stays_unknown() {
        assert_eq!(classify("qqxyz"), FileCategory::Unknown);
        assert_eq!(classify(""), FileCategory::Unknown);
    }

    #[test]
    fn windows_system_files() {
        assert_eq!(
            classify_path(std::path::Path::new("C:\\pagefile.sys")),
            FileCategory::System
        );
    }
}
