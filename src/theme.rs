use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};

/// Sentinel for bundled fonts + the original Auto CJK fallback chain.
pub const FONT_AUTO: &str = "auto";

pub const BASE: Color32 = Color32::from_rgb(25, 23, 36);
pub const SURFACE: Color32 = Color32::from_rgb(31, 29, 46);
pub const OVERLAY: Color32 = Color32::from_rgb(38, 35, 58);
pub const MUTED: Color32 = Color32::from_rgb(110, 106, 134);
pub const SUBTLE: Color32 = Color32::from_rgb(144, 140, 170);
pub const TEXT: Color32 = Color32::from_rgb(224, 222, 244);
pub const LOVE: Color32 = Color32::from_rgb(235, 111, 146);
pub const GOLD: Color32 = Color32::from_rgb(246, 193, 119);
pub const ROSE: Color32 = Color32::from_rgb(235, 188, 186);
pub const PINE: Color32 = Color32::from_rgb(49, 116, 143);
pub const FOAM: Color32 = Color32::from_rgb(156, 207, 216);
pub const IRIS: Color32 = Color32::from_rgb(196, 167, 231);
pub const HIGHLIGHT_LOW: Color32 = Color32::from_rgb(33, 32, 46);
pub const HIGHLIGHT_MED: Color32 = Color32::from_rgb(64, 61, 82);
pub const HIGHLIGHT_HIGH: Color32 = Color32::from_rgb(82, 79, 103);
pub const CONTENT_FONT_SIZE: f32 = 13.0;

pub fn icon_data() -> egui::IconData {
    #[cfg(target_os = "macos")]
    const ICON_BYTES: &[u8] = include_bytes!("../assets/pinkdown-macos-icon.png");
    #[cfg(not(target_os = "macos"))]
    const ICON_BYTES: &[u8] = include_bytes!("../assets/pinkdown-icon.png");

    let image = image::load_from_memory_with_format(ICON_BYTES, image::ImageFormat::Png)
        .expect("decode the embedded PinkDown icon")
        .resize_exact(64, 64, image::imageops::FilterType::Lanczos3)
        .into_rgba8();
    egui::IconData {
        rgba: image.into_raw(),
        width: 64_u32,
        height: 64_u32,
    }
}

pub fn configure(ctx: &egui::Context, preferred_font: &str) {
    configure_fonts(ctx, preferred_font);

    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = SURFACE;
    style.visuals.extreme_bg_color = BASE;
    style.visuals.text_edit_bg_color = Some(SURFACE);
    style.visuals.code_bg_color = OVERLAY;
    style.visuals.faint_bg_color = OVERLAY;
    style.visuals.weak_text_color = Some(SUBTLE);
    style.visuals.widgets.noninteractive.bg_fill = SURFACE;
    style.visuals.widgets.noninteractive.weak_bg_fill = BASE;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, TEXT);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_MED);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.inactive.bg_fill = OVERLAY;
    style.visuals.widgets.inactive.weak_bg_fill = OVERLAY;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_MED);
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, SUBTLE);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.weak_bg_fill = HIGHLIGHT_MED;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, HIGHLIGHT_HIGH);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, FOAM);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.active.bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.weak_bg_fill = HIGHLIGHT_HIGH;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, IRIS);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, ROSE);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.open = style.visuals.widgets.active;
    style.visuals.selection.bg_fill = PINE;
    style.visuals.selection.stroke.color = FOAM;
    style.visuals.hyperlink_color = FOAM;
    style.visuals.warn_fg_color = GOLD;
    style.visuals.error_fg_color = LOVE;
    style.visuals.window_corner_radius = egui::CornerRadius::same(16);
    style.visuals.window_shadow = egui::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(15.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Monospace),
    );
    style.url_in_tooltip = true;
    ctx.set_style(style);
}

pub fn configure_preview(ui: &mut egui::Ui) {
    let style = ui.style_mut();

    // Code blocks sit above the preview's base background; all semantic colors
    // continue to come from the single global Rosé Pine palette above.
    style.visuals.extreme_bg_color = SURFACE;
    style.visuals.code_bg_color = HIGHLIGHT_MED;
    style.visuals.faint_bg_color = HIGHLIGHT_LOW;
    style.visuals.widgets.open.fg_stroke.color = IRIS;
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::new(18.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::new(CONTENT_FONT_SIZE, FontFamily::Monospace),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::new(12.0, FontFamily::Proportional),
    );
    for (name, size) in [
        ("Heading1", 26.0),
        ("Heading2", 21.0),
        ("Heading3", 17.0),
        ("Heading4", 16.0),
        ("Heading5", 15.0),
        ("Heading6", 14.0),
    ] {
        style.text_styles.insert(
            egui::TextStyle::Name(name.into()),
            FontId::new(size, FontFamily::Proportional),
        );
    }
    style.spacing.item_spacing.y = 6.0;
    style.spacing.indent = 20.0;
    style.wrap_mode = Some(egui::TextWrapMode::Wrap);
}

/// Extra vertical space between text rows, as a fraction of the font size.
///
/// egui derives the height of every text row from the first font in the
/// family (`epaint::Font::row_height` = `ascent - descent + line_gap`) and
/// exposes no line-spacing knob, so we grow the font's own `lineGap`
/// metric in the loaded bytes instead. That lifts the editor and the
/// preview alike, since both render through the same font families.
///
/// The effect is deliberately app-wide: every text row — editor, preview,
/// buttons, dialogs — grows by the same amount (~2pt at 13pt text), so the
/// whole UI breathes uniformly. Scoping it to the editor and preview alone
/// would require a dedicated patched font family for those surfaces.
const EXTRA_LINE_GAP_EM: f32 = 0.15;

/// A system face discovered on disk (picker entry).
///
/// `id` and `path` are the same store form: absolute path with Windows
/// `\\?\` prefixes stripped, suitable for `settings.json` and `fs::read`.
/// `label` is a friendly name when known, otherwise the file stem.
#[derive(Clone, Debug)]
pub struct FontOption {
    pub id: String,
    pub label: String,
    pub path: String,
}

/// Discovered faces plus O(1) lookup indexes built once at first use.
struct FontCatalog {
    fonts: Vec<FontOption>,
    /// [`font_path_key`] → index into `fonts`.
    by_key: HashMap<String, usize>,
    /// Lowercase file stem → indices sorted by face preference (regular first).
    by_stem: HashMap<String, Vec<usize>>,
}

/// How a system face is installed into the proportional family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallMode {
    /// Append as CJK / missing-glyph fallback (Auto).
    Fallback,
    /// Lead proportional text (explicit user choice).
    Primary,
}

/// Coerce a stored preference to a known, loadable id.
///
/// Accepts absolute paths (current format), bare file stems, and a few legacy
/// short ids from earlier PinkDown releases. A loadable path that discovery
/// missed (custom install dir) is kept in store form rather than forced to
/// Auto. Unknown / missing faces become [`FONT_AUTO`] so the UI label and the
/// loaded face cannot disagree.
pub fn normalize_font_preference(preferred: &str) -> String {
    let preferred = preferred.trim();
    if preferred.is_empty() || preferred == FONT_AUTO {
        return FONT_AUTO.to_owned();
    }
    if let Some(font) = find_font(preferred) {
        return font.id.clone();
    }
    // Out-of-scan but still loadable — same contract as resolve_system_font.
    let path = Path::new(preferred);
    if path.is_file() && is_font_file(path) {
        return store_path(path);
    }
    FONT_AUTO.to_owned()
}

pub fn font_label(id: &str) -> String {
    if id == FONT_AUTO {
        return "Auto".to_owned();
    }
    find_font(id)
        .map(|font| font.label.clone())
        .unwrap_or_else(|| label_from_path(Path::new(id)))
}

/// Every installable typeface found under the OS font directories.
///
/// Built once by scanning the system (no fixed face list). Auto is not
/// included — the settings UI adds it separately.
pub fn available_fonts() -> &'static [FontOption] {
    &font_catalog().fonts
}

fn font_catalog() -> &'static FontCatalog {
    static CATALOG: OnceLock<FontCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let fonts = discover_system_fonts();
        let mut by_key = HashMap::with_capacity(fonts.len());
        let mut by_stem: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, font) in fonts.iter().enumerate() {
            by_key.insert(font_path_key(&font.path), idx);
            if let Some(stem) = Path::new(&font.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
            {
                by_stem.entry(stem).or_default().push(idx);
            }
        }
        for indices in by_stem.values_mut() {
            indices.sort_by(|&a, &b| {
                face_preference_score(&fonts[b].path)
                    .cmp(&face_preference_score(&fonts[a].path))
                    .then_with(|| fonts[a].path.cmp(&fonts[b].path))
            });
        }
        FontCatalog {
            fonts,
            by_key,
            by_stem,
        }
    })
}

/// Resolve a user preference (path, stem, or legacy short id) to a discovered face.
///
/// Path-like values only match via [`font_path_key`] (no fuzzy stem fallback).
/// Bare tokens may match a stem or a legacy short-id mapping.
fn find_font(preferred: &str) -> Option<&'static FontOption> {
    let preferred = preferred.trim();
    if preferred.is_empty() || preferred == FONT_AUTO {
        return None;
    }
    let catalog = font_catalog();

    if let Some(font) = lookup_by_path(catalog, preferred) {
        return Some(font);
    }

    // Path-shaped preferences never fall through to stem heuristics — a missed
    // path key must not silently bind to an unrelated face with the same stem.
    if looks_like_path(preferred) {
        return None;
    }

    let lowered = preferred.to_ascii_lowercase();
    let needle = strip_font_extension(&lowered);
    if let Some(font) = lookup_by_stem(catalog, needle) {
        return Some(font);
    }

    for stem in legacy_id_stems(preferred) {
        if let Some(font) = lookup_by_stem(catalog, stem) {
            return Some(font);
        }
    }

    None
}

fn lookup_by_path<'a>(catalog: &'a FontCatalog, preferred: &str) -> Option<&'a FontOption> {
    let mut keys = Vec::with_capacity(2);
    keys.push(font_path_key(preferred));
    let path = Path::new(preferred);
    if path.exists() {
        let stored = store_path(path);
        let stored_key = font_path_key(&stored);
        if stored_key != keys[0] {
            keys.push(stored_key);
        }
    }
    for key in keys {
        if let Some(&idx) = catalog.by_key.get(&key) {
            let font = &catalog.fonts[idx];
            if Path::new(&font.path).is_file() {
                return Some(font);
            }
        }
    }
    None
}

fn lookup_by_stem<'a>(catalog: &'a FontCatalog, stem: &str) -> Option<&'a FontOption> {
    let key = stem.to_ascii_lowercase();
    let indices = catalog.by_stem.get(&key)?;
    indices.iter().find_map(|&idx| {
        let font = &catalog.fonts[idx];
        Path::new(&font.path).is_file().then_some(font)
    })
}

fn looks_like_path(value: &str) -> bool {
    value.contains('/')
        || value.contains('\\')
        || Path::new(value).is_absolute()
}

fn strip_font_extension(name: &str) -> &str {
    name.strip_suffix(".ttf")
        .or_else(|| name.strip_suffix(".otf"))
        .or_else(|| name.strip_suffix(".ttc"))
        .or_else(|| name.strip_suffix(".otc"))
        .unwrap_or(name)
}

/// Higher is better when several files share a stem (rare) or when ranking
/// legacy multi-stem lists after index sort.
fn face_preference_score(path: &str) -> i32 {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut score = 0;
    if stem.contains("bold")
        || stem.contains("black")
        || stem.contains("heavy")
        || stem.ends_with("bd")
    {
        score -= 20;
    }
    if stem.contains("italic") || stem.contains("oblique") {
        score -= 10;
    }
    if stem.contains("light") || stem.contains("thin") || stem.ends_with('l') && stem.len() > 1 {
        score -= 5;
    }
    if stem.contains("regular") || stem.contains("medium") {
        score += 5;
    }
    score
}

/// Old settings.json short ids → filenames stems to look up after discovery.
fn legacy_id_stems(id: &str) -> &'static [&'static str] {
    match id {
        "simhei" => &["simhei"],
        "yahei" => &["msyh", "msyhbd", "msyhl"],
        "simsun" => &["simsun"],
        "kaiu" => &["simkai"],
        "fangsong" => &["simfang"],
        "segoeui" => &["segoeui"],
        "pingfang" => &["pingfang", "pingfangui"],
        "heiti" => &["STHeiti Light", "STHeiti Medium", "stheiti light", "stheiti medium"],
        "songti" => &["Songti", "songti"],
        "kaiti" => &["Kaiti", "kaiti"],
        "sf-pro" => &["SFNS", "SFNSText", "SFNSDisplay", "SFNSRounded", "sfns", "sf-pro"],
        "noto-cjk" | "noto-cjk-tt" | "noto-sans-cjk" => &[
            "NotoSansCJK-Regular",
            "NotoSansSC-Regular",
            "notosanscjk-regular",
        ],
        "wqy-microhei" => &["wqy-microhei"],
        "dejavu" => &["DejaVuSans", "dejavusans"],
        _ => &[],
    }
}

/// Auto CJK fallback only — not a picker catalog. First existing path wins.
fn auto_candidate_paths() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &[
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyh.ttf",
            r"C:\Windows\Fonts\simsun.ttc",
        ]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/PingFangUI.ttc",
            "/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved/PingFangUI.ttc",
            "/System/Library/Fonts/STHeiti Light.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/Supplemental/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        ]
    }
    #[cfg(target_os = "linux")]
    {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        ]
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        &[]
    }
}

fn font_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            dirs.push(PathBuf::from(windir).join("Fonts"));
        } else {
            dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            dirs.push(
                PathBuf::from(local)
                    .join("Microsoft")
                    .join("Windows")
                    .join("Fonts"),
            );
        }
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/System/Library/Fonts/Supplemental"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Library").join("Fonts"));
        }
        // Newer macOS keeps some CJK faces here instead of /System/Library/Fonts.
        dirs.push(PathBuf::from(
            "/System/Library/PrivateFrameworks/FontServices.framework/Resources/Reserved",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(&home).join(".local").join("share").join("fonts"));
            dirs.push(PathBuf::from(home).join(".fonts"));
        }
    }
    dirs
}

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "ttf" | "otf" | "ttc" | "otc"
            )
        })
}

/// Faces that are not useful as the main UI typeface.
fn is_excluded_font_file(path: &Path) -> bool {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("emoji")
        || name.contains("lastresort")
        || name == "seguemj"
        || name == "seguiemj"
        || name == "wingdings"
        || name == "wingdings2"
        || name == "wingdings3"
        || name == "webdings"
        || name == "marlett"
        || name == "symbol"
}

fn label_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Font")
        });
    match stem.to_ascii_lowercase().as_str() {
        "msyh" => "Microsoft YaHei".to_owned(),
        "msyhbd" => "Microsoft YaHei Bold".to_owned(),
        "msyhl" => "Microsoft YaHei Light".to_owned(),
        "msyhs" => "Microsoft YaHei UI".to_owned(),
        "simhei" => "SimHei".to_owned(),
        "simsun" | "simsunb" => "SimSun".to_owned(),
        "simkai" => "KaiTi".to_owned(),
        "simfang" => "FangSong".to_owned(),
        "segoeui" => "Segoe UI".to_owned(),
        "segoeuib" => "Segoe UI Bold".to_owned(),
        "seguisb" => "Segoe UI Semibold".to_owned(),
        "malgun" => "Malgun Gothic".to_owned(),
        "yugothm" | "yugothr" => "Yu Gothic".to_owned(),
        "notosanscjk-regular" | "notosanscjksc-regular" => "Noto Sans CJK".to_owned(),
        "pingfang" | "pingfangui" => "PingFang".to_owned(),
        "stheiti light" => "Heiti Light".to_owned(),
        "stheiti medium" => "Heiti Medium".to_owned(),
        _ => stem.to_owned(),
    }
}

fn collect_font_files(dir: &Path, out: &mut Vec<PathBuf>, depth: u8) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Nested packages (e.g. /usr/share/fonts/truetype/dejavu).
            collect_font_files(&path, out, depth - 1);
        } else if is_font_file(&path) && !is_excluded_font_file(&path) {
            out.push(path);
        }
    }
}

fn discover_system_fonts() -> Vec<FontOption> {
    let mut discovered = Vec::new();
    for dir in font_search_dirs() {
        collect_font_files(&dir, &mut discovered, 4);
    }
    discovered.sort();
    discovered.dedup();

    let mut fonts = Vec::with_capacity(discovered.len());
    let mut seen = HashSet::new();
    for path in discovered {
        if !path.is_file() {
            continue;
        }
        let load_path = store_path(&path);
        if !seen.insert(font_path_key(&load_path)) {
            continue;
        }
        // Prefer the store form when it is readable; otherwise fall back to the
        // pre-canonical path (some environments break on stripped verbatim paths).
        let load_path = if Path::new(&load_path).is_file() {
            load_path
        } else {
            path.to_string_lossy().into_owned()
        };
        fonts.push(FontOption {
            id: load_path.clone(),
            label: label_from_path(Path::new(&load_path)),
            path: load_path,
        });
    }

    fonts.sort_by(|a, b| {
        a.label
            .to_ascii_lowercase()
            .cmp(&b.label.to_ascii_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    fonts
}

/// Absolute path string written to settings and used for `fs::read`.
///
/// Canonicalizes when possible and always strips Windows extended-length
/// (`\\?\`) prefixes so identity comparison and on-disk IO share one form.
fn store_path(path: &Path) -> String {
    let absolute = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    });
    strip_verbatim_prefix(&absolute.to_string_lossy())
}

fn strip_verbatim_prefix(path: &str) -> String {
    // \\?\UNC\server\share -> \\server\share
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{rest}");
    }
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        return rest.to_owned();
    }
    // Forward-slash variants occasionally appear in mixed tooling.
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        return format!("//{rest}");
    }
    if let Some(rest) = path.strip_prefix("//?/") {
        return rest.to_owned();
    }
    path.to_owned()
}

/// Case- and separator-normalized identity key for font paths.
fn font_path_key(path: &str) -> String {
    let stripped = strip_verbatim_prefix(path);
    #[cfg(target_os = "windows")]
    {
        stripped.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        stripped
    }
}

fn resolve_system_font(preferred: &str) -> Option<(String, InstallMode)> {
    let preferred = preferred.trim();
    if preferred.is_empty() || preferred == FONT_AUTO {
        return auto_candidate_paths()
            .iter()
            .find(|path| Path::new(path).is_file())
            .map(|path| ((*path).to_owned(), InstallMode::Fallback));
    }
    if let Some(font) = find_font(preferred) {
        return Some((font.path.clone(), InstallMode::Primary));
    }
    // Allow a direct path that exists even if discovery missed it.
    let path = Path::new(preferred);
    if path.is_file() && is_font_file(path) {
        return Some((store_path(path), InstallMode::Primary));
    }
    None
}

/// Load egui's bundled fonts (with relaxed line gap) and attach the preferred
/// system face.
///
/// - **Auto**: first existing entry of the original candidate chain is appended
///   as a CJK / missing-glyph fallback so the bundled proportional font stays
///   primary for Latin UI text.
/// - **Explicit choice**: that face leads the proportional family.
/// - **Monospace** always keeps Hack first; the system face is only a fallback
///   so code stays monospaced.
///
/// System faces are also line-gap patched when they are single-font sfnt files.
/// TrueType Collections (`.ttc`) typically lack a top-level `hhea` and are left
/// unchanged by [`patch_line_gap`].
pub fn configure_fonts(ctx: &egui::Context, preferred_font: &str) {
    let mut fonts = FontDefinitions::default();

    for data in fonts.font_data.values_mut() {
        // `FontDefinitions::default()` builds each font in a fresh, unshared `Arc`
        // today; if egui ever deduplicates them, skip instead of panicking — the
        // worst case is line spacing silently reverting to the font default.
        if let Some(data) = Arc::get_mut(data) {
            data.font = Cow::Owned(patch_line_gap(data.font.to_vec(), EXTRA_LINE_GAP_EM));
        }
    }

    if let Some((path, mode)) = resolve_system_font(preferred_font) {
        if let Ok(bytes) = fs::read(&path) {
            let name = "system-ui".to_owned();
            // Single-font TTF/OTF gain the same line gap as bundled faces; TTC
            // collections usually no-op inside patch_line_gap (no top-level hhea).
            let patched = patch_line_gap(bytes, EXTRA_LINE_GAP_EM);
            fonts
                .font_data
                .insert(name.clone(), FontData::from_owned(patched).into());

            let proportional = fonts
                .families
                .get_mut(&FontFamily::Proportional)
                .expect("default proportional family");
            match mode {
                InstallMode::Fallback => proportional.push(name.clone()),
                InstallMode::Primary => proportional.insert(0, name.clone()),
            }

            fonts
                .families
                .get_mut(&FontFamily::Monospace)
                .expect("default monospace family")
                .push(name);
        }
    }

    ctx.set_fonts(fonts);
}

/// Grow the `lineGap` horizontal metric (in font units) inside `sfnt` font
/// bytes, in both `hhea` and `OS/2` (`sTypoLineGap`). ttf-parser — which
/// ab_glyph sits on — reads whichever of the two the font's
/// `USE_TYPO_METRICS` flag selects, so both are bumped.
///
/// The delta is `extra_em` × the font's line height (`ascender − descender`,
/// chosen the same way ttf-parser picks metrics), so text rows grow by
/// exactly `extra_em` × font size.
///
/// The table-directory checksums and `head.checkSumAdjustment` are left
/// stale on purpose: ttf-parser never validates sfnt checksums, and the
/// patched bytes only ever feed egui's own font pipeline. Recompute them
/// if the bytes ever reach a checksum-validating consumer.
fn patch_line_gap(mut bytes: Vec<u8>, extra_em: f32) -> Vec<u8> {
    if bytes.len() < 12 {
        return bytes;
    }
    let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if bytes.len() < 12 + 16 * num_tables {
        return bytes;
    }
    let mut hhea = None;
    let mut os2 = None;
    for i in 0..num_tables {
        let record = 12 + 16 * i;
        let tag = &bytes[record..record + 4];
        let offset = u32::from_be_bytes([
            bytes[record + 8],
            bytes[record + 9],
            bytes[record + 10],
            bytes[record + 11],
        ]) as usize;
        match tag {
            b"hhea" => hhea = Some(offset),
            b"OS/2" => os2 = Some(offset),
            _ => {}
        }
    }
    let Some(hhea) = hhea else {
        return bytes;
    };
    let Some(height) = line_height_units(&bytes, hhea, os2) else {
        return bytes;
    };
    let delta = (extra_em * height).round() as i16;
    if delta == 0 {
        return bytes;
    }
    bump_i16(&mut bytes, hhea + 8, delta); // hhea.lineGap
    if let Some(os2) = os2 {
        bump_i16(&mut bytes, os2 + 74, delta); // OS/2.sTypoLineGap
    }
    bytes
}

/// The font's line height (`ascender − descender`) in font units, selected
/// the same way ttf-parser picks metrics: OS/2 typo metrics when
/// `USE_TYPO_METRICS` (fsSelection bit 7) is set, otherwise hhea — falling
/// back to OS/2 typo metrics when hhea's ascender or descender is zero.
fn line_height_units(bytes: &[u8], hhea: usize, os2: Option<usize>) -> Option<f32> {
    if hhea + 8 > bytes.len() {
        return None;
    }
    let hhea_asc = i16::from_be_bytes([bytes[hhea + 4], bytes[hhea + 5]]);
    let hhea_desc = i16::from_be_bytes([bytes[hhea + 6], bytes[hhea + 7]]);
    let os2 = os2.filter(|&o| o + 74 <= bytes.len());
    let use_typo = os2
        .is_some_and(|o| i16::from_be_bytes([bytes[o + 62], bytes[o + 63]]) & 0x0080 != 0);
    let (asc, desc) = if use_typo || hhea_asc == 0 || hhea_desc == 0 {
        let os2 = os2?;
        (
            i16::from_be_bytes([bytes[os2 + 68], bytes[os2 + 69]]),
            i16::from_be_bytes([bytes[os2 + 72], bytes[os2 + 73]]),
        )
    } else {
        (hhea_asc, hhea_desc)
    };
    Some(f32::from(asc) - f32::from(desc))
}

fn bump_i16(bytes: &mut [u8], offset: usize, delta: i16) {
    if offset + 2 > bytes.len() {
        return;
    }
    let value = i16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
    let value = value.saturating_add(delta);
    bytes[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal valid sfnt: offset table + `hhea`/`OS/2` tables. hhea metrics
    /// are fixed at ascender 1536 / descender -512 (line height 2048); the
    /// OS/2 typo metrics and `USE_TYPO_METRICS` flag are configurable.
    fn fake_sfnt(
        hhea_line_gap: i16,
        typo_asc: i16,
        typo_desc: i16,
        typo_line_gap: i16,
        use_typo: bool,
    ) -> Vec<u8> {
        let mut hhea = vec![0u8; 36];
        hhea[4..6].copy_from_slice(&1536i16.to_be_bytes()); // ascender
        hhea[6..8].copy_from_slice(&(-512i16).to_be_bytes()); // descender
        hhea[8..10].copy_from_slice(&hhea_line_gap.to_be_bytes());

        let mut os2 = vec![0u8; 96];
        os2[62..64].copy_from_slice(&(if use_typo { 0x0080u16 } else { 0u16 }).to_be_bytes()); // fsSelection
        os2[68..70].copy_from_slice(&typo_asc.to_be_bytes()); // sTypoAscender
        os2[72..74].copy_from_slice(&typo_desc.to_be_bytes()); // sTypoDescender
        os2[74..76].copy_from_slice(&typo_line_gap.to_be_bytes());

        let tables = [(b"hhea".to_vec(), hhea), (b"OS/2".to_vec(), os2)];
        let mut bytes = vec![0u8; 12 + 16 * tables.len()];
        bytes[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        bytes[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
        let mut offset = bytes.len();
        for (i, (tag, table)) in tables.into_iter().enumerate() {
            let record = 12 + 16 * i;
            bytes[record..record + 4].copy_from_slice(&tag);
            bytes[record + 8..record + 12].copy_from_slice(&(offset as u32).to_be_bytes());
            bytes[record + 12..record + 16].copy_from_slice(&(table.len() as u32).to_be_bytes());
            bytes.extend_from_slice(&table);
            offset += table.len();
        }
        bytes
    }

    fn read_i16(bytes: &[u8], offset: usize) -> i16 {
        i16::from_be_bytes([bytes[offset], bytes[offset + 1]])
    }

    #[test]
    fn bumps_both_line_gap_metrics_by_the_requested_em_fraction() {
        let patched = patch_line_gap(fake_sfnt(0, 1536, -512, 0, false), 0.15);
        // Tables are appended in order: hhea(36) @44, OS/2(96) @80.
        // hhea line height = 1536 - (-512) = 2048; 0.15 * 2048 = 307.2 -> 307
        assert_eq!(read_i16(&patched, 44 + 8), 307); // hhea.lineGap
        assert_eq!(read_i16(&patched, 80 + 74), 307); // OS/2.sTypoLineGap
    }

    #[test]
    fn adds_to_an_existing_line_gap() {
        let patched = patch_line_gap(fake_sfnt(100, 1536, -512, 50, false), 0.1);
        // 0.1 * 2048 = 204.8 -> 205
        assert_eq!(read_i16(&patched, 44 + 8), 305);
        assert_eq!(read_i16(&patched, 80 + 74), 255);
    }

    #[test]
    fn uses_os2_typo_height_when_use_typo_metrics_is_set() {
        // USE_TYPO_METRICS switches the delta basis to the OS/2 typo metrics:
        // height = 1000 - (-100) = 1100; 0.15 * 1100 = 165, not the hhea 307.
        let patched = patch_line_gap(fake_sfnt(0, 1000, -100, 0, true), 0.15);
        assert_eq!(read_i16(&patched, 44 + 8), 165); // hhea.lineGap
        assert_eq!(read_i16(&patched, 80 + 74), 165); // OS/2.sTypoLineGap
    }

    #[test]
    fn leaves_non_sfnt_bytes_untouched() {
        assert_eq!(patch_line_gap(vec![0u8; 4], 0.15), vec![0u8; 4]);
        assert_eq!(patch_line_gap(Vec::<u8>::new(), 0.15), Vec::<u8>::new());
    }

    #[test]
    fn zero_fraction_is_a_no_op() {
        let original = fake_sfnt(0, 1536, -512, 0, false);
        assert_eq!(patch_line_gap(original.clone(), 0.0), original);
    }

    #[test]
    fn real_egui_fonts_grow_row_height() {
        use ab_glyph::{Font as _, ScaleFont as _};

        for font in [
            epaint_default_fonts::UBUNTU_LIGHT,
            epaint_default_fonts::HACK_REGULAR,
        ] {
            let original = ab_glyph::FontVec::try_from_vec(font.to_vec()).unwrap();
            let patched = ab_glyph::FontVec::try_from_vec(patch_line_gap(
                font.to_vec(),
                EXTRA_LINE_GAP_EM,
            ))
            .unwrap();
            let expected = EXTRA_LINE_GAP_EM * 13.0;
            let actual =
                patched.as_scaled(13.0).line_gap() - original.as_scaled(13.0).line_gap();
            assert!(
                (actual - expected).abs() < 0.1,
                "line gap grew by {actual}pt, expected {expected}pt"
            );
        }
    }

    #[test]
    fn normalize_maps_empty_and_unknown_to_auto() {
        assert_eq!(normalize_font_preference(""), FONT_AUTO);
        assert_eq!(normalize_font_preference("   "), FONT_AUTO);
        assert_eq!(normalize_font_preference("not-a-real-font"), FONT_AUTO);
        assert_eq!(normalize_font_preference(FONT_AUTO), FONT_AUTO);
    }

    #[test]
    fn normalize_keeps_available_catalog_ids() {
        for font in available_fonts() {
            assert_eq!(normalize_font_preference(&font.id), font.id);
        }
    }

    #[test]
    fn font_path_key_strips_verbatim_prefix_and_normalizes() {
        assert_eq!(
            font_path_key(r"\\?\C:\Windows\Fonts\msyh.ttc"),
            font_path_key(r"C:\Windows\Fonts\msyh.ttc")
        );
        assert_eq!(
            font_path_key(r"\\?\UNC\server\share\font.ttf"),
            font_path_key(r"\\server\share\font.ttf")
        );
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                font_path_key(r"C:\Windows\Fonts\MSYH.TTC"),
                font_path_key(r"c:/windows/fonts/msyh.ttc")
            );
        }
    }

    #[test]
    fn normalize_and_resolve_keep_loadable_path_outside_scan() {
        let dir = std::env::temp_dir().join(format!(
            "pinkdown-font-test-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("custom-out-of-scan.ttf");
        fs::write(&path, b"not a real sfnt").expect("write temp font stub");

        let preferred = path.to_string_lossy();
        let normalized = normalize_font_preference(&preferred);
        assert_ne!(
            normalized, FONT_AUTO,
            "loadable path outside scan must not collapse to Auto"
        );
        assert!(
            Path::new(&normalized).is_file(),
            "normalized id must remain a readable path"
        );

        let resolved = resolve_system_font(&normalized).expect("out-of-scan path resolves");
        assert_eq!(resolved.1, InstallMode::Primary);
        assert!(Path::new(&resolved.0).is_file());

        // Path-like garbage must not stem-match into a real system face.
        assert_eq!(
            normalize_font_preference(r"C:\definitely\missing\msyh.ttc"),
            FONT_AUTO
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn plain_path_matches_catalog_entry_via_path_key() {
        let Some(font) = available_fonts().first() else {
            return;
        };
        // Re-key through a slightly different string form when possible.
        let key_match = available_fonts()
            .iter()
            .find(|f| font_path_key(&f.path) == font_path_key(&font.path));
        assert!(key_match.is_some());
        assert_eq!(
            normalize_font_preference(&font.path),
            font.id,
            "catalog path must normalize back to stored id"
        );
    }

    #[test]
    fn auto_resolves_as_fallback_not_primary() {
        match resolve_system_font(FONT_AUTO) {
            None => {}
            Some((_, mode)) => assert_eq!(mode, InstallMode::Fallback),
        }
    }

    #[test]
    fn explicit_available_font_resolves_as_primary() {
        let Some(font) = available_fonts().first() else {
            return;
        };
        let resolved = resolve_system_font(&font.id).expect("available font resolves");
        assert_eq!(resolved.0, font.path);
        assert_eq!(resolved.1, InstallMode::Primary);
    }

    #[test]
    fn missing_explicit_font_does_not_silently_resolve() {
        assert!(resolve_system_font("not-a-real-font").is_none());
    }

    #[test]
    fn auto_candidate_paths_prefer_historical_cjk_chain() {
        let paths = auto_candidate_paths();
        assert!(!paths.is_empty());
        #[cfg(target_os = "windows")]
        {
            assert!(paths[0].ends_with("simhei.ttf"));
            assert!(paths.iter().any(|p| p.contains("msyh")));
            assert!(paths.iter().any(|p| p.contains("simsun")));
        }
        #[cfg(target_os = "macos")]
        {
            assert!(paths.iter().any(|p| p.contains("PingFang")));
            assert!(paths.iter().any(|p| p.contains("STHeiti")));
        }
        #[cfg(target_os = "linux")]
        {
            assert!(paths.iter().any(|p| p.contains("NotoSansCJK")));
        }
    }

    #[test]
    fn discovery_lists_installed_system_fonts_not_a_fixed_catalog() {
        let fonts = available_fonts();
        assert!(
            !fonts.is_empty() || font_search_dirs().iter().all(|d| !d.is_dir()),
            "expected scanned fonts when system font directories exist"
        );
        for font in fonts {
            assert!(
                Path::new(&font.path).is_file(),
                "listed font missing on disk: {} ({})",
                font.id,
                font.path
            );
            assert_eq!(font.id, font.path, "id is the loadable absolute path");
            assert!(!font.label.is_empty());
        }
        // Must not be limited to the old 5–6 hardcoded faces.
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        assert!(
            fonts.len() > 10,
            "expected a broad system scan, got only {} fonts",
            fonts.len()
        );
    }

    #[test]
    fn legacy_short_ids_still_resolve_when_files_exist() {
        #[cfg(target_os = "windows")]
        {
            if Path::new(r"C:\Windows\Fonts\simhei.ttf").is_file() {
                let id = normalize_font_preference("simhei");
                assert_ne!(id, FONT_AUTO);
                assert!(Path::new(&id).is_file());
            }
            if Path::new(r"C:\Windows\Fonts\msyh.ttc").is_file() {
                let id = normalize_font_preference("yahei");
                assert_ne!(id, FONT_AUTO);
            }
        }
        #[cfg(target_os = "macos")]
        {
            // Any discovered PingFang* file should map from the legacy id.
            if available_fonts().iter().any(|f| {
                Path::new(&f.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.to_ascii_lowercase().contains("pingfang"))
            }) {
                assert_ne!(normalize_font_preference("pingfang"), FONT_AUTO);
            }
        }
    }
}
