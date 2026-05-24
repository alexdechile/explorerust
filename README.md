# ExploreRust 📁

Un explorador de archivos moderno para Linux escrito en **Rust puro**, con interfaz gráfica (GUI) y de terminal (TUI).

## Características

- 🖥 **Interfaz dual**: GUI desktop (egui/OpenGL) y TUI completa (ratatui)
- 📋 **Lista detallada ordenable** por nombre, tamaño, tipo, fecha, permisos
- 🗂 **Pestañas** para navegar múltiples carpetas simultáneamente
- ✂️ **Copiar/Cortar/Pegar** con operaciones en background y barra de progreso
- ✏️ **Editor integrado** de texto plano y Markdown (preview en tiempo real)
- ⬛ **"Abrir terminal aquí"** totalmente configurable, detecta automáticamente 15+ terminales
- 🎨 **Adapta el tema** al sistema (claro/oscuro + color de acento de GNOME)
- 🔖 **Bookmarks** editables y persistent
- 💽 **Detección de dispositivos** desde `/proc/mounts`
- 🔍 **Búsqueda en tiempo real** en el directorio actual

## Instalación rápida

```bash
cargo install --path .
# O copiar el binario directamente:
sudo cp target/release/explorerust /usr/local/bin/
```

## Uso

```bash
# Lanzar GUI (modo por defecto)
explorerust

# Lanzar con una ruta específica
explorerust /home/user/Documentos

# Lanzar en modo TUI (terminal)
explorerust --tui

# TUI en una ruta específica
explorerust --tui /var/log
```

## Configuración

La configuración se guarda automáticamente en `~/.config/explorerust/config.toml`:

```toml
[terminal]
# "auto" detecta el terminal instalado, o especifica: kitty, alacritty, 
# gnome-terminal, konsole, xterm, tilix, wezterm, foot, custom
preferred = "auto"
custom_command = ""
custom_args = "--working-directory {path}"  # {path} se reemplaza

[editor]
syntax_highlight = true
tab_size = 4
word_wrap = true
show_line_numbers = true
font_size = 14.0

[gui]
show_hidden = false
confirm_delete = true
sidebar_width = 200.0

[bookmarks]
paths = ["/home/user", "/home/user/Documentos"]
```

## Atajos de teclado GUI

| Acción | Atajo |
|---|---|
| Navegar atrás/adelante | `Alt+←` / `Alt+→` |
| Subir directorio | `Alt+↑` |
| Selección múltiple | `Ctrl+Click` |
| Copiar / Cortar / Pegar | `Ctrl+C` / `Ctrl+X` / `Ctrl+V` |
| Nueva pestaña | `Ctrl+T` |
| Cerrar pestaña | `Ctrl+W` |
| Eliminar | `Delete` |
| Refrescar | `F5` |
| Guardar (editor) | `Ctrl+S` |

## Atajos de teclado TUI

| Acción | Tecla |
|---|---|
| Navegar | `↑↓` o `j/k` |
| Abrir / Entrar | `Enter` o `l` |
| Subir directorio | `←` `h` o `Backspace` |
| Editar archivo | `e` |
| Abrir terminal | `t` |
| Copiar / Cortar / Pegar | `c` / `x` / `p` |
| Selección | `Space` |
| Renombrar | `r` o `F2` |
| Eliminar | `d` o `Delete` |
| Nueva carpeta | `m` |
| Nuevo archivo | `n` |
| Buscar | `/` |
| Mostrar ocultos | `.` |
| Ordenar | `s` / `S` |
| Ayuda | `?` |
| Salir | `q` |

## Terminales soportados automáticamente

kitty, alacritty, wezterm, foot, tilix, gnome-terminal, konsole, xfce4-terminal, lxterminal, mate-terminal, terminator, urxvt, rxvt, xterm, st

## Construido con

| Crate | Propósito |
|---|---|
| `eframe` + `egui` | GUI inmediata sin dependencias del sistema |
| `ratatui` + `crossterm` | TUI interactiva en terminal |
| `clap` | Argumentos CLI |
| `serde` + `toml` | Configuración en TOML |
| `walkdir` | Traversal de directorios |
| `pulldown-cmark` | Render de Markdown |
| `syntect` | Syntax highlighting (pure Rust) |
| `dark-light` | Detección de tema del sistema |
| `open` | Abrir con app por defecto |
| `nix` | Permisos y propietario Unix |
| `which` | Detección de terminales instalados |
