use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

// ── File entry ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub extension: String,
    pub permissions: String,
    pub owner: String,
}

impl FileEntry {
    pub fn size_display(&self) -> String {
        if self.is_dir {
            return "—".into();
        }
        human_size(self.size)
    }

    pub fn date_display(&self) -> String {
        self.modified
            .and_then(|t| {
                let dt: DateTime<Local> = t.into();
                Some(dt.format("%Y-%m-%d %H:%M").to_string())
            })
            .unwrap_or_else(|| "—".into())
    }

    pub fn kind_display(&self) -> String {
        if self.is_dir {
            "Directory".into()
        } else if self.is_symlink {
            "Symlink".into()
        } else {
            match self.extension.to_lowercase().as_str() {
                "rs" => "Rust source",
                "toml" | "yaml" | "yml" | "json" => "Config",
                "md" | "markdown" => "Markdown",
                "txt" => "Text",
                "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => "Image",
                "mp4" | "mkv" | "avi" | "mov" => "Video",
                "mp3" | "flac" | "ogg" | "wav" => "Audio",
                "zip" | "tar" | "gz" | "xz" | "bz2" | "zst" => "Archive",
                "pdf" => "PDF",
                "sh" | "bash" | "zsh" | "fish" => "Script",
                "py" => "Python",
                "js" | "ts" => "JavaScript",
                "html" | "htm" => "HTML",
                "css" => "CSS",
                "" => "File",
                ext => ext,
            }
            .into()
        }
    }
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

fn format_permissions(mode: u32) -> String {
    let mut s = String::with_capacity(9);
    let bits = [(6, 'r'), (5, 'w'), (4, 'x'), (3, 'r'), (2, 'w'), (1, 'x'), (0, 'r'), (8, 'w'), (7, 'x')];
    // owner
    for shift in [8u32, 7, 6] {
        let c = match shift {
            8 => 'r', 7 => 'w', 6 => 'x', _ => '-'
        };
        s.push(if mode & (1 << shift) != 0 { c } else { '-' });
    }
    // group
    for shift in [5u32, 4, 3] {
        let c = match shift {
            5 => 'r', 4 => 'w', 3 => 'x', _ => '-'
        };
        s.push(if mode & (1 << shift) != 0 { c } else { '-' });
    }
    // other
    for shift in [2u32, 1, 0] {
        let c = match shift {
            2 => 'r', 1 => 'w', 0 => 'x', _ => '-'
        };
        s.push(if mode & (1 << shift) != 0 { c } else { '-' });
    }
    let _ = bits;
    s
}

pub fn read_dir(path: &Path, show_hidden: bool) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    let read = std::fs::read_dir(path)
        .with_context(|| format!("Cannot read directory: {}", path.display()))?;

    for entry in read.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let symlink_meta = entry.path().symlink_metadata().ok();
        let is_symlink = symlink_meta.map(|m| m.file_type().is_symlink()).unwrap_or(false);
        let extension = entry
            .path()
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        let permissions = format_permissions(meta.permissions().mode());
        let uid = meta.uid();
        let owner = nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid))
            .ok()
            .flatten()
            .map(|u| u.name)
            .unwrap_or_else(|| uid.to_string());

        entries.push(FileEntry {
            name,
            path: entry.path(),
            is_dir: meta.is_dir(),
            is_symlink,
            size: meta.len(),
            modified: meta.modified().ok(),
            extension,
            permissions,
            owner,
        });
    }

    Ok(entries)
}

// ── Sort ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SortColumn {
    Name,
    Size,
    Modified,
    Kind,
    Permissions,
}

impl SortColumn {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Size => "Size",
            Self::Modified => "Modified",
            Self::Kind => "Type",
            Self::Permissions => "Permissions",
        }
    }
}

pub fn sort_entries(entries: &mut Vec<FileEntry>, col: SortColumn, ascending: bool) {
    entries.sort_by(|a, b| {
        // Directories always first
        let dir_cmp = b.is_dir.cmp(&a.is_dir);
        if dir_cmp != std::cmp::Ordering::Equal {
            return dir_cmp;
        }
        let ord = match col {
            SortColumn::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortColumn::Size => a.size.cmp(&b.size),
            SortColumn::Modified => a.modified.cmp(&b.modified),
            SortColumn::Kind => a.kind_display().cmp(&b.kind_display()),
            SortColumn::Permissions => a.permissions.cmp(&b.permissions),
        };
        if ascending { ord } else { ord.reverse() }
    });
}

// ── File operations ──────────────────────────────────────────────────────────

pub struct OperationProgress {
    pub bytes_done: Arc<AtomicU64>,
    pub bytes_total: Arc<AtomicU64>,
    pub finished: Arc<AtomicBool>,
    pub error: Arc<std::sync::Mutex<Option<String>>>,
}

impl OperationProgress {
    pub fn new(total: u64) -> Self {
        Self {
            bytes_done: Arc::new(AtomicU64::new(0)),
            bytes_total: Arc::new(AtomicU64::new(total)),
            finished: Arc::new(AtomicBool::new(false)),
            error: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    pub fn fraction(&self) -> f32 {
        let total = self.bytes_total.load(Ordering::Relaxed);
        if total == 0 {
            return 1.0;
        }
        self.bytes_done.load(Ordering::Relaxed) as f32 / total as f32
    }

    pub fn is_done(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
}

pub fn delete_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path).with_context(|| format!("Delete dir: {}", path.display()))
    } else {
        std::fs::remove_file(path).with_context(|| format!("Delete file: {}", path.display()))
    }
}

pub fn rename_path(from: &Path, to: &Path) -> Result<()> {
    std::fs::rename(from, to).with_context(|| {
        format!("Rename {} -> {}", from.display(), to.display())
    })
}

pub fn create_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Create dir: {}", path.display()))
}

pub fn create_file(path: &Path) -> Result<()> {
    std::fs::write(path, "")
        .with_context(|| format!("Create file: {}", path.display()))
}

/// Start background copy. Returns a progress handle.
pub fn copy_async(sources: Vec<PathBuf>, dest: PathBuf, progress: Arc<OperationProgress>) {
    std::thread::spawn(move || {
        let result = copy_entries(&sources, &dest, &progress);
        if let Err(e) = result {
            *progress.error.lock().unwrap() = Some(e.to_string());
        }
        progress.finished.store(true, Ordering::Relaxed);
    });
}

/// Start background move. Returns a progress handle.
pub fn move_async(sources: Vec<PathBuf>, dest: PathBuf, progress: Arc<OperationProgress>) {
    std::thread::spawn(move || {
        let result = move_entries(&sources, &dest, &progress);
        if let Err(e) = result {
            *progress.error.lock().unwrap() = Some(e.to_string());
        }
        progress.finished.store(true, Ordering::Relaxed);
    });
}

fn copy_entries(sources: &[PathBuf], dest: &Path, prog: &Arc<OperationProgress>) -> Result<()> {
    for src in sources {
        let dst = dest.join(src.file_name().unwrap_or_default());
        if src.is_dir() {
            copy_dir_recursive(src, &dst, prog)?;
        } else {
            copy_file_progress(src, &dst, prog)?;
        }
    }
    Ok(())
}

fn move_entries(sources: &[PathBuf], dest: &Path, prog: &Arc<OperationProgress>) -> Result<()> {
    for src in sources {
        let dst = dest.join(src.file_name().unwrap_or_default());
        // Try atomic rename first (same filesystem)
        if std::fs::rename(src, &dst).is_err() {
            if src.is_dir() {
                copy_dir_recursive(src, &dst, prog)?;
                std::fs::remove_dir_all(src)?;
            } else {
                copy_file_progress(src, &dst, prog)?;
                std::fs::remove_file(src)?;
            }
        } else {
            let size = if src.is_file() {
                src.metadata().map(|m| m.len()).unwrap_or(0)
            } else { 0 };
            prog.bytes_done.fetch_add(size, Ordering::Relaxed);
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path, prog: &Arc<OperationProgress>) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let dst_entry = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dst_entry, prog)?;
        } else {
            copy_file_progress(&entry.path(), &dst_entry, prog)?;
        }
    }
    Ok(())
}

fn copy_file_progress(src: &Path, dst: &Path, prog: &Arc<OperationProgress>) -> Result<()> {
    use std::io::{Read, Write};
    let mut src_f = std::fs::File::open(src)?;
    let mut dst_f = std::fs::File::create(dst)?;
    let mut buf = vec![0u8; 65536];
    loop {
        let n = src_f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        dst_f.write_all(&buf[..n])?;
        prog.bytes_done.fetch_add(n as u64, Ordering::Relaxed);
    }
    Ok(())
}

pub fn total_size(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|p| path_size(p)).sum()
}

fn path_size(p: &Path) -> u64 {
    if p.is_file() {
        p.metadata().map(|m| m.len()).unwrap_or(0)
    } else if p.is_dir() {
        walkdir::WalkDir::new(p)
            .into_iter()
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
            .sum()
    } else {
        0
    }
}
