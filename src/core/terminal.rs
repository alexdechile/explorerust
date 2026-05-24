use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

/// Known terminals with their "open in directory" argument patterns.
/// {path} will be replaced with the actual path.
const KNOWN_TERMINALS: &[(&str, &str)] = &[
    ("kitty", "--directory {path}"),
    ("alacritty", "--working-directory {path}"),
    ("wezterm", "start --cwd {path}"),
    ("foot", "--working-directory {path}"),
    ("tilix", "--working-directory={path}"),
    ("gnome-terminal", "--working-directory={path}"),
    ("konsole", "--workdir {path}"),
    ("xfce4-terminal", "--working-directory={path}"),
    ("lxterminal", "--working-directory={path}"),
    ("mate-terminal", "--working-directory={path}"),
    ("terminator", "--working-directory={path}"),
    ("urxvt", "-cd {path}"),
    ("rxvt", "-cd {path}"),
    ("xterm", "-e sh -c \"cd {path} && exec ${SHELL:-bash}\""),
    ("st", "-e sh -c \"cd {path} && exec ${SHELL:-bash}\""),
];

/// Detect all terminals available on this system.
pub fn detect_terminals() -> Vec<String> {
    KNOWN_TERMINALS
        .iter()
        .filter(|(bin, _)| is_available(bin))
        .map(|(bin, _)| bin.to_string())
        .collect()
}

fn is_available(bin: &str) -> bool {
    which::which(bin).is_ok()
        || Path::new(&format!("/usr/bin/{}", bin)).exists()
        || Path::new(&format!("/usr/local/bin/{}", bin)).exists()
}

/// Launch the preferred terminal in `dir`.
pub fn open_terminal(preferred: &str, custom_cmd: &str, custom_args: &str, dir: &Path) -> Result<()> {
    let dir_str = dir.to_string_lossy();

    let (bin, args_template) = if preferred == "auto" {
        let available = detect_terminals();
        match available.first() {
            Some(t) => {
                let template = KNOWN_TERMINALS
                    .iter()
                    .find(|(b, _)| b == t)
                    .map(|(_, a)| *a)
                    .unwrap_or("");
                (t.clone(), template.to_string())
            }
            None => bail!("No known terminal found. Configure one in ~/.config/explorerust/config.toml"),
        }
    } else if preferred == "custom" {
        (custom_cmd.to_string(), custom_args.to_string())
    } else {
        let template = KNOWN_TERMINALS
            .iter()
            .find(|(b, _)| *b == preferred)
            .map(|(_, a)| *a)
            .unwrap_or("--working-directory {path}");
        (preferred.to_string(), template.to_string())
    };

    // Replace {path} placeholder
    let full_args = args_template.replace("{path}", &dir_str);
    let parts: Vec<&str> = full_args.split_whitespace().collect();

    let mut cmd = Command::new(&bin);
    cmd.args(&parts);
    cmd.spawn()
        .map_err(|e| anyhow::anyhow!("Failed to launch '{}': {}", bin, e))?;

    Ok(())
}
