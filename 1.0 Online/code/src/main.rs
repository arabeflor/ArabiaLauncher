#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod i18n;
mod news;

use eframe::egui::{
    self, Color32, CornerRadius, FontId, Image, Layout, Margin, RichText, Stroke, Vec2,
};
use eframe::egui::{TextureHandle, TextureOptions};
use i18n::{Language, Texts};
use serde::{Deserialize, Serialize};
use std::fs;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const GAME_VERSION: &str = "1.12.2";
const FORGE_VERSION: &str = "14.23.5.2859";
const REQUIRED_JARS: &[&str] = &[
    "forge-1.12.2-14.23.5.2859-universal.jar",
    "launchwrapper-1.12.jar",
    "asm-debug-all-5.2.jar",
    "jopt-simple-5.0.3.jar",
    "guava-21.0.jar",
    "gson-2.8.0.jar",
    "authlib-1.5.25.jar",
    "realms-1.10.22.jar",
    "commons-lang3-3.5.jar",
    "commons-io-2.5.jar",
    "commons-codec-1.10.jar",
    "commons-compress-1.8.1.jar",
    "httpclient-4.3.3.jar",
    "commons-logging-1.1.3.jar",
    "httpcore-4.3.2.jar",
    "netty-all-4.1.9.Final.jar",
    "fastutil-7.1.0.jar",
    "jline-3.5.1.jar",
    "jna-4.4.0.jar",
    "jna-platform-4.4.0.jar",
    "oshi-core-1.1.jar",
    "icu4j-core-mojang-51.2.jar",
    "patchy-1.3.9.jar",
    "trove4j-3.0.3.jar",
    "vecmath-1.5.2.jar",
    "lzma-0.0.1.jar",
    "jinput-2.0.5.jar",
    "jutils-1.0.0.jar",
    "lwjgl-2.9.4-nightly-20150209.jar",
    "lwjgl_util-2.9.4-nightly-20150209.jar",
    "codecjorbis-20101023.jar",
    "codecwav-20101023.jar",
    "libraryjavasound-20101123.jar",
    "librarylwjglopenal-20100824.jar",
    "soundsystem-20120107.jar",
    "text2speech-1.10.3.jar",
    "log4j-api-2.8.1.jar",
    "log4j-core-2.8.1.jar",
    "akka-actor_2.11-2.3.3.jar",
    "config-1.2.1.jar",
    "scala-actors-migration_2.11-1.1.0.jar",
    "scala-compiler-2.11.1.jar",
    "scala-continuations-library_2.11-1.0.2_mc.jar",
    "scala-continuations-plugin_2.11.1-1.0.2_mc.jar",
    "scala-library-2.11.1.jar",
    "scala-parser-combinators_2.11-1.0.1.jar",
    "scala-reflect-2.11.1.jar",
    "scala-swing_2.11-1.0.1.jar",
    "scala-xml_2.11-1.0.2.jar",
    "maven-artifact-3.5.3.jar",
];

const BACKGROUND: Color32 = Color32::from_rgb(14, 19, 16);
const PANEL: Color32 = Color32::from_rgb(21, 29, 23);
const CARD: Color32 = Color32::from_rgb(28, 38, 30);
const MUTED: Color32 = Color32::from_rgb(157, 174, 157);
const TEXT: Color32 = Color32::from_rgb(235, 241, 226);
const DEFAULT_ACCENT: Color32 = Color32::from_rgb(105, 190, 76);
const BORDER: Color32 = Color32::from_rgb(48, 65, 49);
const HOVER: Color32 = Color32::from_rgb(39, 53, 40);

#[derive(Clone, Copy)]
enum LauncherIcon {
    Home,
    Mods,
    Settings,
    Library,
    News,
    Game,
    World,
    ResourcePack,
    Folder,
    Config,
    Theme,
    Language,
    Play,
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Home,
    Mods,
    Settings,
    Library,
    News,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    language: String,
    theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".to_owned(),
            theme: "default".to_owned(),
        }
    }
}

struct LauncherApp {
    root: PathBuf,
    game_dir: PathBuf,
    settings_path: PathBuf,
    settings: Settings,
    page: Page,
    screen_transition_started: Instant,
    player_name: String,
    mods: Vec<String>,
    disabled_mods: Vec<String>,
    mods_unlocked: bool,
    world_count: usize,
    resource_pack_count: usize,
    mod_search: String,
    status_message: Option<String>,
    status_is_error: bool,
    child: Option<Child>,
    launch_started_at: Option<Instant>,
    hero_texture: Option<TextureHandle>,
    news_texture: Option<TextureHandle>,
    logo_texture: Option<TextureHandle>,
    assets_loaded: bool,
}

impl LauncherApp {
    fn new() -> Self {
        let executable_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        let root = executable_dir
            .as_ref()
            .and_then(|directory| {
                directory
                    .ancestors()
                    .find(|ancestor| ancestor.join("mcdata").is_dir())
                    .map(Path::to_path_buf)
            })
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .filter(|directory| directory.join("mcdata").is_dir())
            })
            .or(executable_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        let game_dir = root.join("mcdata");
        let settings_path = settings_path(&root);
        let (settings, status_message) = load_settings(&settings_path);
        let status_is_error = status_message.is_some();
        let mut app = Self {
            root,
            game_dir,
            settings_path,
            settings,
            page: Page::Home,
            screen_transition_started: Instant::now(),
            player_name: "Player".to_owned(),
            mods: Vec::new(),
            disabled_mods: Vec::new(),
            mods_unlocked: false,
            world_count: 0,
            resource_pack_count: 0,
            mod_search: String::new(),
            status_message,
            status_is_error,
            child: None,
            launch_started_at: None,
            hero_texture: None,
            news_texture: None,
            logo_texture: None,
            assets_loaded: false,
        };
        if let Err(error) = app.refresh_mods() {
            app.set_error(format!("Could not read installed mods: {error}"));
        }
        app
    }

    fn language(&self) -> Language {
        Language::from_code(&self.settings.language)
    }

    fn refresh_mods(&mut self) -> Result<(), String> {
        let mods_dir = self.game_dir.join("mods");
        let entries =
            fs::read_dir(&mods_dir).map_err(|error| format!("{}: {error}", mods_dir.display()))?;
        let mut mods = Vec::new();
        let mut disabled_mods = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    match mod_filename_state(name) {
                        Some((_, true)) => disabled_mods.push(name.to_owned()),
                        Some((_, false)) => mods.push(name.to_owned()),
                        None => {}
                    }
                }
            }
        }
        mods.sort_by_key(|name| name.to_lowercase());
        disabled_mods.sort_by_key(|name| name.to_lowercase());
        self.world_count = count_entries(&self.game_dir.join("saves"), true)?;
        self.resource_pack_count = count_entries(&self.game_dir.join("resourcepacks"), false)?;
        self.mods = mods;
        self.disabled_mods = disabled_mods;
        Ok(())
    }

    fn toggle_mod(&mut self, filename: &str) {
        if !self.mods_unlocked {
            return;
        }
        match toggle_mod_file(&self.game_dir.join("mods"), filename) {
            Ok(disabled) => match self.refresh_mods() {
                Ok(()) => {
                    let message = if disabled {
                        self.language().texts().disabled_mod
                    } else {
                        self.language().texts().enabled_mod
                    };
                    self.set_notice(format!("{message}: {}", display_mod_name(filename)));
                }
                Err(error) => self.set_error(format!(
                    "Mod was renamed, but the mod list could not be refreshed: {error}"
                )),
            },
            Err(error) => self.set_error(error),
        }
    }

    fn set_error(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_is_error = true;
    }

    fn set_notice(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_is_error = false;
    }

    fn save_settings(&mut self) {
        let result = (|| {
            if let Some(parent) = self.settings_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            let contents =
                serde_json::to_string_pretty(&self.settings).map_err(|error| error.to_string())?;
            fs::write(&self.settings_path, contents).map_err(|error| error.to_string())
        })();
        match result {
            Ok(()) => self.status_message = None,
            Err(error) => self.set_error(format!("Could not save launcher settings: {error}")),
        }
    }

    fn load_assets(&mut self, ctx: &egui::Context) {
        if self.assets_loaded {
            return;
        }
        self.assets_loaded = true;
        let logo = image::load_from_memory(include_bytes!("../../img/app-icon.png"))
            .map_err(|error| format!("Could not load ArabiaLauncher logo: {error}"))
            .map(|image| {
                let image = image.to_rgba8();
                let size = [image.width() as usize, image.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
                ctx.load_texture("arabia-launcher-logo", color_image, TextureOptions::LINEAR)
            });
        match logo {
            Ok(texture) => self.logo_texture = Some(texture),
            Err(error) => {
                self.set_error(error);
                return;
            }
        }
        let hero_path = self.root.join("img").join("hero.png");
        let news_path = self.root.join("img").join("latest-news.png");
        match load_texture(ctx, &hero_path, "launcher-hero") {
            Ok(texture) => self.hero_texture = texture,
            Err(error) => {
                self.set_error(error);
                return;
            }
        }
        match load_texture(ctx, &news_path, "launcher-news") {
            Ok(texture) => self.news_texture = texture,
            Err(error) => self.set_error(error),
        }
    }

    fn launch_game(&mut self) {
        let texts = self.language().texts();
        if self.child.is_some() {
            self.set_notice(texts.already_running.to_owned());
            return;
        }
        let java = self
            .game_dir
            .join("java")
            .join("jre-legacy")
            .join("bin")
            .join("java.exe");
        if !java.is_file() {
            self.set_error(format!("{}: {}", texts.java_missing, java.display()));
            return;
        }

        let classpath = match build_classpath(&self.game_dir) {
            Ok(classpath) => classpath,
            Err(error) => {
                self.set_error(format!("{}: {error}", texts.launch_failed));
                return;
            }
        };
        let natives = self.game_dir.join("natives").join(GAME_VERSION);
        let assets = self.game_dir.join("assets");
        let username = if self.player_name.trim().is_empty() {
            "Player"
        } else {
            self.player_name.trim()
        };
        let mut command = Command::new(&java);
        command
            .current_dir(&self.game_dir)
            .arg(format!("-Djava.library.path={}", natives.display()))
            .arg("-Dfml.ignoreInvalidMinecraftCertificates=true")
            .arg("-Dfml.ignorePatchDiscrepancies=true")
            .arg("-Dlog4j2.formatMsgNoLookups=true")
            .arg("-cp")
            .arg(classpath)
            .arg("net.minecraft.launchwrapper.Launch")
            .arg("--username")
            .arg(username)
            .arg("--version")
            .arg(GAME_VERSION)
            .arg("--gameDir")
            .arg(".")
            .arg("--assetsDir")
            .arg(assets)
            .arg("--assetIndex")
            .arg("1.12")
            .arg("--accessToken")
            .arg("0")
            .arg("--userProperties")
            .arg("{}")
            .arg("--uuid")
            .arg("00000000000000000000000000000000")
            .arg("--userType")
            .arg("legacy")
            .arg("--versionType")
            .arg("Forge")
            .arg("--tweakClass")
            .arg("net.minecraftforge.fml.common.launcher.FMLTweaker");
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let result = command.spawn();

        match result {
            Ok(child) => {
                self.child = Some(child);
                self.launch_started_at = Some(Instant::now());
                self.set_notice(texts.launching.to_owned());
            }
            Err(error) => self.set_error(format!("{}: {error}", texts.launch_failed)),
        }
    }

    fn open_folder(&mut self, folder: &str, label: &str) {
        let path = self.game_dir.join(folder);
        if let Err(error) = Command::new("explorer").arg(&path).spawn() {
            self.set_error(format!("{label}: {error} ({})", path.display()));
        }
    }

    fn set_theme(&mut self, theme: &str) {
        self.settings.theme = theme.to_owned();
        self.save_settings();
    }
}

fn count_entries(directory: &Path, directories_only: bool) -> Result<usize, String> {
    let mut entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("{}: {error}", directory.display())),
    };
    entries.try_fold(0, |count, entry| {
        let entry = entry.map_err(|error| error.to_string())?;
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        let included = if directories_only {
            file_type.is_dir()
        } else {
            file_type.is_file() || file_type.is_dir()
        };
        Ok(count + usize::from(included))
    })
}

fn mod_filename_state(filename: &str) -> Option<(&str, bool)> {
    if ends_with_ascii_case_insensitive(filename, ".jar.disabled") {
        Some((&filename[..filename.len() - ".disabled".len()], true))
    } else if ends_with_ascii_case_insensitive(filename, ".jar") {
        Some((filename, false))
    } else {
        None
    }
}

fn ends_with_ascii_case_insensitive(value: &str, suffix: &str) -> bool {
    value
        .len()
        .checked_sub(suffix.len())
        .and_then(|start| value.get(start..))
        .is_some_and(|ending| ending.eq_ignore_ascii_case(suffix))
}

fn toggle_mod_file(directory: &Path, filename: &str) -> Result<bool, String> {
    let Some((_, currently_disabled)) = mod_filename_state(filename) else {
        return Err(format!("Unsupported mod filename: {filename}"));
    };
    let source = directory.join(filename);
    let destination_name = if currently_disabled {
        filename[..filename.len() - ".disabled".len()].to_owned()
    } else {
        format!("{filename}.disabled")
    };
    let destination = directory.join(destination_name);
    if destination.exists() {
        return Err(format!(
            "Could not rename mod: destination already exists ({})",
            destination.display()
        ));
    }
    fs::rename(&source, &destination)
        .map_err(|error| format!("Could not rename mod {}: {error}", source.display()))?;
    Ok(!currently_disabled)
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.load_assets(ctx);
        if let Some(child) = self.child.as_mut() {
            ctx.request_repaint_after(Duration::from_millis(250));
            match child.try_wait() {
                Ok(Some(exit)) => {
                    self.child = None;
                    self.launch_started_at = None;
                    self.set_notice(format!(
                        "{} ({})",
                        self.language().texts().game_closed,
                        exit
                    ));
                }
                Ok(None) => {}
                Err(error) => {
                    self.child = None;
                    self.launch_started_at = None;
                    self.set_error(format!("Could not monitor Minecraft: {error}"));
                }
            }
        }
        if self
            .launch_started_at
            .is_some_and(|started| started.elapsed() >= Duration::from_secs(30))
        {
            self.launch_started_at = None;
            self.set_notice(self.language().texts().game_running.to_owned());
        }

        let accent = match self.settings.theme.as_str() {
            "midnight" => Color32::from_rgb(65, 145, 255),
            "violet" => Color32::from_rgb(177, 91, 255),
            _ => DEFAULT_ACCENT,
        };
        ctx.set_visuals(egui::Visuals::dark());
        let mut visuals = ctx.style().visuals.clone();
        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = PANEL;
        visuals.widgets.noninteractive.bg_fill = PANEL;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(27, 37, 29);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(27, 37, 29);
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
        visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
        visuals.widgets.hovered.bg_fill = HOVER;
        visuals.widgets.hovered.weak_bg_fill = HOVER;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, accent.gamma_multiply(0.75));
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
        visuals.widgets.active.bg_fill = accent.gamma_multiply(0.34);
        visuals.widgets.active.weak_bg_fill = accent.gamma_multiply(0.34);
        visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, accent);
        visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
        visuals.widgets.active.corner_radius = CornerRadius::same(8);
        visuals.widgets.open.bg_fill = Color32::from_rgb(34, 47, 36);
        visuals.widgets.open.weak_bg_fill = Color32::from_rgb(34, 47, 36);
        visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, accent.gamma_multiply(0.65));
        visuals.widgets.open.corner_radius = CornerRadius::same(8);
        visuals.selection.bg_fill = accent;
        visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
        ctx.set_visuals(visuals);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(9.0, 9.0);
        style.spacing.button_padding = Vec2::new(12.0, 8.0);
        style.spacing.interact_size.y = 34.0;
        ctx.set_style(style);

        let texts = self.language().texts();
        self.show_sidebar(ctx, texts, accent);
        self.show_topbar(ctx, texts);
        self.show_footer(ctx);
        self.show_error_dialog(ctx);
        let transition =
            (self.screen_transition_started.elapsed().as_secs_f32() / 0.32).clamp(0.0, 1.0);
        if transition < 1.0 {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BACKGROUND)
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_opacity(0.35 + transition * 0.65);
                ui.add_space((1.0 - transition) * 14.0);
                match self.page {
                    Page::Home => self.show_home(ui, texts, accent),
                    Page::Mods => self.show_mods(ui, texts, accent),
                    Page::Settings => self.show_settings(ui, texts, accent),
                    Page::Library => self.show_library_page(ui, texts, accent),
                    Page::News => self.show_news_page(ui, texts, accent),
                }
            });
    }
}

impl LauncherApp {
    fn show_error_dialog(&mut self, ctx: &egui::Context) {
        if !self.status_is_error {
            return;
        }
        let Some(message) = self.status_message.clone() else {
            return;
        };
        egui::Window::new("ArabiaLauncher")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(RichText::new(message).color(Color32::from_rgb(255, 160, 150)));
                ui.add_space(8.0);
                if ui.button("OK").clicked() {
                    self.status_message = None;
                    self.status_is_error = false;
                }
            });
    }

    fn show_sidebar(&mut self, ctx: &egui::Context, texts: &Texts, accent: Color32) {
        egui::SidePanel::left("navigation")
            .exact_width(208.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(14, 18))
                    .stroke(Stroke::new(1.0_f32, BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.logo_texture {
                        ui.add(
                            Image::from_texture(texture)
                                .fit_to_exact_size(Vec2::splat(42.0))
                                .corner_radius(CornerRadius::same(7)),
                        );
                    }
                    ui.add_space(7.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("ArabiaLauncher")
                                .size(17.0)
                                .strong()
                                .color(TEXT),
                        );
                    });
                });
                ui.add_space(25.0);
                ui.label(
                    RichText::new(texts.library_section)
                        .size(10.0)
                        .strong()
                        .color(MUTED),
                );
                ui.add_space(7.0);
                for (page, label, glyph) in [
                    (Page::Home, texts.home, LauncherIcon::Home),
                    (Page::Mods, texts.mods, LauncherIcon::Mods),
                    (Page::Settings, texts.settings, LauncherIcon::Settings),
                    (Page::Library, texts.library, LauncherIcon::Library),
                    (Page::News, texts.latest_news, LauncherIcon::News),
                ] {
                    self.nav_button(ui, page, label, glyph, accent);
                    ui.add_space(4.0);
                }
                ui.with_layout(Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Forge {FORGE_VERSION}"))
                            .size(10.0)
                            .color(MUTED),
                    );
                    ui.label(
                        RichText::new(format!("Minecraft {GAME_VERSION}"))
                            .size(11.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.add_space(3.0);
                    ui.label(
                        RichText::new(format!(
                            "{} {}  ·  {} {}",
                            self.mods.len(),
                            texts.enabled_mods,
                            self.disabled_mods.len(),
                            texts.disabled_mods
                        ))
                        .size(10.0)
                        .color(MUTED),
                    );
                });
            });
    }

    fn nav_button(
        &mut self,
        ui: &mut egui::Ui,
        page: Page,
        label: &str,
        glyph: LauncherIcon,
        accent: Color32,
    ) {
        let active = self.page == page;
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), egui::Sense::click());
        let hovered = ui
            .ctx()
            .animate_bool_with_time(response.id, response.hovered(), 0.16);
        let fill = if active {
            accent.gamma_multiply(0.16)
        } else {
            HOVER.gamma_multiply(hovered * 0.7)
        };
        ui.painter().rect_filled(rect, CornerRadius::same(7), fill);
        if active {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
                CornerRadius::same(2),
                accent,
            );
        }
        draw_icon(
            ui.painter(),
            glyph,
            egui::pos2(rect.left() + 23.0, rect.center().y),
            17.0,
            if active || response.hovered() {
                accent
            } else {
                MUTED
            },
        );
        ui.painter().text(
            egui::pos2(rect.left() + 43.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            if active { TEXT } else { MUTED },
        );
        let clicked = response.clicked();
        response.on_hover_text(label);
        if clicked {
            self.navigate_to(page);
        }
    }

    fn navigate_to(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.screen_transition_started = Instant::now();
        }
        if page != Page::Mods {
            self.mods_unlocked = false;
        }
        if page == Page::Mods || page == Page::Library || page == Page::Home {
            if let Err(error) = self.refresh_mods() {
                self.set_error(format!("{}: {error}", self.language().texts().mods_failed));
            }
        }
    }

    fn show_topbar(&mut self, ctx: &egui::Context, texts: &Texts) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(68.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(Margin::symmetric(22, 12))
                    .stroke(Stroke::new(1.0_f32, BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let heading = match self.page {
                        Page::Home => texts.home,
                        Page::Mods => texts.mods,
                        Page::Settings => texts.settings,
                        Page::Library => texts.library,
                        Page::News => texts.latest_news,
                    };
                    ui.label(RichText::new(heading).size(17.0).strong().color(TEXT));
                    if self.page == Page::Home {
                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("language")
                                .selected_text(self.language().label())
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for language in Language::ALL {
                                        if ui
                                            .selectable_label(
                                                self.language() == language,
                                                language.label(),
                                            )
                                            .clicked()
                                        {
                                            self.settings.language = language.code().to_owned();
                                            self.save_settings();
                                        }
                                    }
                                });
                            ui.label(RichText::new(texts.language).color(MUTED).size(12.0));
                        });
                    }
                });
            });
    }

    fn show_footer(&self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .exact_height(42.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(35, 79, 41))
                    .inner_margin(Margin::symmetric(22, 8))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(72, 132, 67))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            "ArabiaLauncher 1.12.2 -  Public Standard Edition - Portable",
                        )
                        .size(12.0)
                        .strong()
                        .color(Color32::WHITE),
                    );
                });
            });
    }

    fn show_home(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let available_width = ui.available_width();
            let side_width = 292.0_f32.min((available_width * 0.29).max(250.0));
            let main_width = (available_width - side_width - 18.0).max(330.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    Vec2::new(main_width, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.show_hero(ui, texts, accent);
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(texts.latest_news)
                                    .size(16.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        });
                        ui.add_space(8.0);
                        self.show_news(ui, texts, accent);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(texts.library_title)
                                    .size(16.0)
                                    .strong()
                                    .color(TEXT),
                            );
                        });
                        ui.add_space(4.0);
                        self.show_library_tiles(ui, texts, accent);
                    },
                );
                ui.allocate_ui_with_layout(
                    Vec2::new(side_width, 0.0),
                    Layout::top_down(egui::Align::Min),
                    |ui| self.show_quick_panel(ui, texts),
                );
            });
        });
    }

    fn show_library_tiles(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        let row_width = ui.available_width();
        let tile_width = ((row_width - 18.0) / 3.0).max(120.0);
        let (worlds_clicked, mods_clicked, packs_clicked) = ui
            .horizontal_top(|ui| {
                let worlds_clicked = library_tile(
                    ui,
                    tile_width,
                    texts.worlds,
                    self.world_count,
                    texts.worlds_hint,
                    texts.open_saves,
                    LauncherIcon::World,
                    accent,
                );
                let mods_clicked = library_tile(
                    ui,
                    tile_width,
                    texts.mods,
                    self.mods.len(),
                    texts.mods_card_hint,
                    texts.manage_mods,
                    LauncherIcon::Mods,
                    accent,
                );
                let packs_clicked = library_tile(
                    ui,
                    tile_width,
                    texts.resource_packs,
                    self.resource_pack_count,
                    texts.resource_packs_hint,
                    texts.open_resourcepacks,
                    LauncherIcon::ResourcePack,
                    accent,
                );
                (worlds_clicked, mods_clicked, packs_clicked)
            })
            .inner;
        if worlds_clicked {
            self.open_folder("saves", texts.folder_failed);
        } else if mods_clicked {
            self.navigate_to(Page::Mods);
        } else if packs_clicked {
            self.open_folder("resourcepacks", texts.folder_failed);
        }
    }

    fn show_hero(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(31, 45, 32))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(66, 91, 57)))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(26));
        frame.show(ui, |ui| {
            ui.set_min_height(244.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("ARABIALAUNCHER  /  MINECRAFT {GAME_VERSION}"))
                            .size(11.0)
                            .strong()
                            .color(accent),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Minecraft {GAME_VERSION}"))
                            .size(25.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.label(RichText::new(texts.welcome_back).size(14.0).color(MUTED));
                    ui.add_space(8.0);
                    egui::Frame::new()
                        .fill(Color32::from_rgb(18, 28, 20))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(68, 91, 59)))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(9, 4))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.player_name)
                                    .desired_width(187.0)
                                    .font(FontId::proportional(25.0))
                                    .text_color(accent)
                                    .frame(false)
                                    .hint_text(texts.player),
                            );
                        });
                    ui.label(RichText::new(texts.adventure).size(15.0).color(MUTED));
                    ui.add_space(12.0);
                    if self.child.is_some() {
                        ui.horizontal(|ui| {
                            let loading = self.launch_started_at.is_some();
                            if loading {
                                ui.spinner();
                            }
                            let label = if loading {
                                texts.launching
                            } else {
                                texts.game_running
                            };
                            let button = egui::Button::new(
                                RichText::new(label)
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(accent)
                            .corner_radius(CornerRadius::same(4));
                            ui.add_enabled_ui(false, |ui| {
                                ui.add_sized([ui.available_width().min(180.0), 50.0], button);
                            });
                            if compact_icon_button(
                                ui,
                                LauncherIcon::Settings,
                                texts.settings,
                                accent,
                            ) {
                                self.navigate_to(Page::Settings);
                            }
                        });
                    } else {
                        let play_button = egui::Button::new(
                            RichText::new(format!("▶   {}", texts.play))
                                .size(16.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(accent)
                        .stroke(Stroke::new(1.0_f32, accent.gamma_multiply(0.82)))
                        .corner_radius(CornerRadius::same(4));
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([ui.available_width().min(208.0), 50.0], play_button)
                                .clicked()
                            {
                                self.launch_game();
                            }
                            if compact_icon_button(
                                ui,
                                LauncherIcon::Settings,
                                texts.settings,
                                accent,
                            ) {
                                self.navigate_to(Page::Settings);
                            }
                        });
                    }
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(texture) = &self.hero_texture {
                        let image_width = (ui.available_width() * 0.48).clamp(150.0, 340.0);
                        ui.add(
                            Image::from_texture(texture)
                                .fit_to_exact_size(Vec2::new(image_width, 206.0))
                                .corner_radius(CornerRadius::same(4)),
                        );
                    } else {
                        landscape_art(ui, accent);
                    }
                });
            });
        });
    }

    fn show_quick_panel(&mut self, ui: &mut egui::Ui, texts: &Texts) {
        card(ui, |ui| {
            section_heading(ui, LauncherIcon::Game, texts.game_info, DEFAULT_ACCENT);
            ui.add_space(12.0);
            info_row(
                ui,
                LauncherIcon::Library,
                texts.version,
                GAME_VERSION,
                DEFAULT_ACCENT,
            );
            info_row(
                ui,
                LauncherIcon::Mods,
                texts.forge,
                FORGE_VERSION,
                DEFAULT_ACCENT,
            );
            info_row(
                ui,
                LauncherIcon::Config,
                texts.java,
                "8 (x64)",
                DEFAULT_ACCENT,
            );
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            section_heading(ui, LauncherIcon::Folder, texts.quick_access, DEFAULT_ACCENT);
            ui.add_space(8.0);
            self.folder_button(ui, "mods", texts.open_mods, LauncherIcon::Mods);
            self.folder_button(
                ui,
                "resourcepacks",
                texts.open_resourcepacks,
                LauncherIcon::ResourcePack,
            );
            self.folder_button(ui, "config", texts.open_config, LauncherIcon::Config);
            self.folder_button(ui, "saves", texts.open_saves, LauncherIcon::World);
        });
        ui.add_space(10.0);
        card(ui, |ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::Min), |ui| {
                section_heading(ui, LauncherIcon::Theme, texts.theme, DEFAULT_ACCENT);
            });
            ui.add_space(4.0);
            let selected_theme = self.settings.theme.clone();
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                theme_swatches(ui, &selected_theme, texts, |theme| self.set_theme(theme));
            });
        });
        ui.add_space(6.0);
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(format!("↻  {}", texts.refresh)).clicked() {
                match self.refresh_mods() {
                    Ok(()) => self.set_notice(texts.mods_refreshed.to_owned()),
                    Err(error) => self.set_error(format!("{}: {error}", texts.mods_failed)),
                }
            }
        });
    }

    fn folder_button(&mut self, ui: &mut egui::Ui, folder: &str, label: &str, glyph: LauncherIcon) {
        if icon_action_button(ui, label, glyph, DEFAULT_ACCENT) {
            self.open_folder(folder, label);
        }
    }

    fn show_news(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        egui::Frame::new()
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(44, 58, 45)))
            .corner_radius(CornerRadius::same(11))
            .inner_margin(Margin::same(13))
            .show(ui, |ui| {
                ui.set_min_height(115.0);
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.news_texture {
                        ui.add(
                            Image::from_texture(texture)
                                .fit_to_exact_size(Vec2::new(144.0, 100.0))
                                .corner_radius(CornerRadius::same(8)),
                        );
                    } else {
                        news_art(ui, accent);
                    }
                    ui.add_space(9.0);
                    ui.vertical(|ui| {
                        ui.add_space(10.0);
                        icon_label(
                            ui,
                            LauncherIcon::News,
                            texts.latest_news,
                            15.0,
                            11.0,
                            accent,
                            true,
                        );
                        ui.label(
                            RichText::new(news::LATEST_NEWS.title)
                                .size(18.0)
                                .strong()
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format!("v{}", news::LATEST_NEWS.version))
                                .size(20.0)
                                .strong()
                                .color(accent),
                        );
                        ui.label(
                            RichText::new(texts.news_description)
                                .size(12.0)
                                .color(MUTED),
                        );
                    });
                });
            });
    }

    fn show_settings(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            page_intro(
                ui,
                LauncherIcon::Settings,
                texts.settings,
                texts.settings_hint,
                accent,
            );
            ui.add_space(14.0);
            card(ui, |ui| {
                section_heading(ui, LauncherIcon::Game, texts.account_section, accent);
                ui.add_space(10.0);
                ui.label(RichText::new(texts.player_name).size(12.0).color(MUTED));
                ui.add_sized(
                    [ui.available_width().min(460.0), 40.0],
                    egui::TextEdit::singleline(&mut self.player_name)
                        .hint_text(texts.player)
                        .margin(Margin::symmetric(11, 8)),
                );
            });
            ui.add_space(10.0);
            card(ui, |ui| {
                section_heading(ui, LauncherIcon::Theme, texts.theme, accent);
                ui.label(RichText::new(texts.theme_hint).size(11.0).color(MUTED));
                ui.add_space(8.0);
                let selected_theme = self.settings.theme.clone();
                theme_swatches(ui, &selected_theme, texts, |theme| self.set_theme(theme));
            });
            ui.add_space(10.0);
            card(ui, |ui| {
                section_heading(ui, LauncherIcon::Language, texts.language, accent);
                ui.label(RichText::new(texts.language_hint).size(11.0).color(MUTED));
                ui.add_space(8.0);
                egui::ComboBox::from_id_salt("settings_language")
                    .selected_text(self.language().label())
                    .width(ui.available_width().min(340.0))
                    .show_ui(ui, |ui| {
                        for language in Language::ALL {
                            if ui
                                .selectable_label(self.language() == language, language.label())
                                .clicked()
                            {
                                self.settings.language = language.code().to_owned();
                                self.save_settings();
                            }
                        }
                    });
            });
            ui.add_space(10.0);
            card(ui, |ui| {
                section_heading(ui, LauncherIcon::Folder, texts.manage_files, accent);
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    if icon_action_button(ui, texts.open_mods, LauncherIcon::Mods, accent) {
                        self.open_folder("mods", texts.folder_failed);
                    }
                    ui.add_space(6.0);
                    if icon_action_button(
                        ui,
                        texts.open_resourcepacks,
                        LauncherIcon::ResourcePack,
                        accent,
                    ) {
                        self.open_folder("resourcepacks", texts.folder_failed);
                    }
                    ui.add_space(6.0);
                    if icon_action_button(ui, texts.open_config, LauncherIcon::Config, accent) {
                        self.open_folder("config", texts.folder_failed);
                    }
                    ui.add_space(6.0);
                    if icon_action_button(ui, texts.open_saves, LauncherIcon::World, accent) {
                        self.open_folder("saves", texts.folder_failed);
                    }
                });
            });
        });
    }

    fn show_library_page(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            page_intro(ui, LauncherIcon::Library, texts.library, "", accent);
            ui.add_space(12.0);
            self.show_library_tiles(ui, texts, accent);
            ui.add_space(16.0);
            card(ui, |ui| {
                section_heading(ui, LauncherIcon::Folder, texts.quick_access, accent);
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    if icon_action_button(ui, texts.open_mods, LauncherIcon::Mods, accent) {
                        self.open_folder("mods", texts.folder_failed);
                    }
                    if icon_action_button(
                        ui,
                        texts.open_resourcepacks,
                        LauncherIcon::ResourcePack,
                        accent,
                    ) {
                        self.open_folder("resourcepacks", texts.folder_failed);
                    }
                    if icon_action_button(ui, texts.open_config, LauncherIcon::Config, accent) {
                        self.open_folder("config", texts.folder_failed);
                    }
                    if icon_action_button(ui, texts.open_saves, LauncherIcon::World, accent) {
                        self.open_folder("saves", texts.folder_failed);
                    }
                });
            });
        });
    }

    fn show_news_page(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            page_intro(
                ui,
                LauncherIcon::News,
                texts.latest_news,
                texts.news_hint,
                accent,
            );
            ui.add_space(12.0);
            let mut launch_clicked = false;
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        if let Some(texture) = &self.news_texture {
                            ui.add(
                                Image::from_texture(texture)
                                    .fit_to_exact_size(Vec2::new(
                                        (ui.available_width() * 0.34).clamp(200.0, 320.0),
                                        210.0,
                                    ))
                                    .corner_radius(CornerRadius::same(7)),
                            );
                        } else {
                            news_art(ui, accent);
                        }
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            icon_label(
                                ui,
                                LauncherIcon::News,
                                &format!("v{}", news::LATEST_NEWS.version),
                                17.0,
                                12.0,
                                accent,
                                true,
                            );
                            ui.add_space(7.0);
                            ui.label(
                                RichText::new(news::LATEST_NEWS.title)
                                    .size(25.0)
                                    .strong()
                                    .color(TEXT),
                            );
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(texts.news_description)
                                    .size(14.0)
                                    .color(MUTED),
                            );
                            ui.add_space(16.0);
                            launch_clicked =
                                icon_action_button(ui, texts.play, LauncherIcon::Play, accent);
                        });
                    });
                });
            if launch_clicked {
                self.launch_game();
            }
        });
    }

    fn show_mods(&mut self, ui: &mut egui::Ui, texts: &Texts, accent: Color32) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_heading(ui, LauncherIcon::Mods, texts.mods, accent);
                ui.label(
                    RichText::new(format!(
                        "{} {}  ·  {} {}",
                        self.mods.len(),
                        texts.enabled_mods,
                        self.disabled_mods.len(),
                        texts.disabled_mods
                    ))
                    .color(MUTED),
                );
            });
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(texts.open_mods).clicked() {
                    self.open_folder("mods", texts.folder_failed);
                }
                if ui.button(format!("↻  {}", texts.refresh)).clicked() {
                    match self.refresh_mods() {
                        Ok(()) => self.set_notice(texts.mods_refreshed.to_owned()),
                        Err(error) => self.set_error(format!("{}: {error}", texts.mods_failed)),
                    }
                }
                if lock_icon_button(ui, self.mods_unlocked, texts, accent) {
                    self.mods_unlocked = !self.mods_unlocked;
                    let message = if self.mods_unlocked {
                        texts.mods_unlocked
                    } else {
                        texts.mods_locked
                    };
                    self.set_notice(message.to_owned());
                }
            });
        });
        ui.add_space(14.0);
        ui.add_sized(
            [ui.available_width().min(440.0), 36.0],
            egui::TextEdit::singleline(&mut self.mod_search)
                .hint_text(texts.search_mods)
                .margin(Margin::symmetric(10, 8)),
        );
        ui.add_space(11.0);
        let search = self.mod_search.trim().to_lowercase();
        let mods: Vec<(String, bool)> = self
            .mods
            .iter()
            .map(|name| (name.as_str(), false))
            .chain(self.disabled_mods.iter().map(|name| (name.as_str(), true)))
            .filter(|(name, _)| name.to_lowercase().contains(&search))
            .map(|(name, disabled)| (name.to_owned(), disabled))
            .collect();
        egui::ScrollArea::vertical().show(ui, |ui| {
            if mods.is_empty() {
                egui::Frame::new()
                    .fill(CARD)
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::same(24))
                    .show(ui, |ui| {
                        ui.label(RichText::new(texts.no_mods).size(17.0).strong().color(TEXT));
                        ui.add_space(5.0);
                        ui.label(RichText::new(texts.mods_hint).color(MUTED));
                    });
            } else {
                for (name, disabled) in mods {
                    let filename_for_toggle = name.clone();
                    let mut toggle_clicked = false;
                    egui::Frame::new()
                        .fill(CARD)
                        .stroke(Stroke::new(1.0_f32, BORDER))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(14, 10))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                draw_inline_icon(
                                    ui,
                                    if disabled {
                                        LauncherIcon::Settings
                                    } else {
                                        LauncherIcon::Mods
                                    },
                                    16.0,
                                    if disabled {
                                        Color32::from_rgb(225, 165, 83)
                                    } else {
                                        accent
                                    },
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(display_mod_name(&name)).strong().color(TEXT),
                                    );
                                    ui.label(
                                        RichText::new(if disabled {
                                            texts.disabled_mod
                                        } else {
                                            texts.enabled_mod
                                        })
                                        .size(11.0)
                                        .color(MUTED),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                    toggle_clicked = toggle_switch(
                                        ui,
                                        !disabled,
                                        self.mods_unlocked,
                                        accent,
                                        if disabled {
                                            texts.enable_mod
                                        } else {
                                            texts.disable_mod
                                        },
                                    );
                                });
                            });
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&name)
                                        .size(10.0)
                                        .color(Color32::from_rgb(122, 141, 124)),
                                );
                            });
                        });
                    if toggle_clicked {
                        self.toggle_mod(&filename_for_toggle);
                        break;
                    }
                    ui.add_space(6.0);
                }
            }
        });
    }
}

fn library_tile(
    ui: &mut egui::Ui,
    width: f32,
    title: &str,
    count: usize,
    hint: &str,
    action_label: &str,
    glyph: LauncherIcon,
    accent: Color32,
) -> bool {
    let mut clicked = false;
    let tile = ui.allocate_ui_with_layout(
        Vec2::new(width, 168.0),
        Layout::top_down(egui::Align::Min),
        |ui| {
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0_f32, BORDER))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(13))
                .show(ui, |ui| {
                    ui.set_min_height(138.0);
                    section_heading(ui, glyph, title, accent);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        draw_inline_icon(ui, glyph, 20.0, accent);
                        ui.label(
                            RichText::new(count.to_string())
                                .size(23.0)
                                .strong()
                                .color(accent),
                        );
                    });
                    ui.label(RichText::new(hint).size(10.5).color(MUTED));
                    ui.add_space(4.0);
                    clicked = icon_action_button(ui, action_label, glyph, accent);
                });
        },
    );
    let tile_rect = tile.response.rect;
    let hover_response = ui.interact(
        tile_rect,
        ui.id().with(("library-tile", title)),
        egui::Sense::hover(),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(hover_response.id, hover_response.hovered(), 0.18);
    if hover > 0.0 {
        ui.painter().rect_stroke(
            tile_rect,
            CornerRadius::same(10),
            Stroke::new(1.5 * hover, accent.gamma_multiply(hover)),
            egui::StrokeKind::Inside,
        );
    }
    clicked
}

fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::same(15))
        .show(ui, add_contents);
}

fn info_row(ui: &mut egui::Ui, glyph: LauncherIcon, label: &str, value: &str, accent: Color32) {
    ui.horizontal(|ui| {
        draw_inline_icon(ui, glyph, 13.0, accent);
        ui.label(RichText::new(label).size(12.0).color(MUTED));
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(value).size(12.0).color(TEXT));
        });
    });
}

fn draw_inline_icon(ui: &mut egui::Ui, glyph: LauncherIcon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(22.0, 24.0), egui::Sense::hover());
    draw_icon(ui.painter(), glyph, rect.center(), size, color);
}

fn page_intro(
    ui: &mut egui::Ui,
    glyph: LauncherIcon,
    title: &str,
    subtitle: &str,
    accent: Color32,
) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(9), accent.gamma_multiply(0.16));
        draw_icon(ui.painter(), glyph, rect.center(), 21.0, accent);
        ui.add_space(7.0);
        ui.vertical(|ui| {
            ui.heading(RichText::new(title).color(TEXT));
            if !subtitle.is_empty() {
                ui.label(RichText::new(subtitle).color(MUTED));
            }
        });
    });
}

fn lock_icon_button(ui: &mut egui::Ui, unlocked: bool, texts: &Texts, accent: Color32) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(34.0), egui::Sense::click());
    let hovered = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.14);
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(6),
        if hovered > 0.0 { HOVER } else { CARD },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        if unlocked { "🔓" } else { "🔒" },
        FontId::proportional(17.0),
        if unlocked { accent } else { TEXT },
    );
    response
        .on_hover_text(if unlocked {
            texts.lock_mod_changes
        } else {
            texts.unlock_mod_changes
        })
        .clicked()
}

fn toggle_switch(
    ui: &mut egui::Ui,
    enabled: bool,
    editable: bool,
    accent: Color32,
    tooltip: &str,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(46.0, 26.0),
        if editable {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let hovered = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.14);
    let position = ui
        .ctx()
        .animate_bool_with_time(response.id.with("position"), enabled, 0.16);
    let active_color = if editable {
        accent
    } else {
        MUTED.gamma_multiply(0.55)
    };
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(13),
        if enabled {
            active_color.gamma_multiply(0.58)
        } else if hovered > 0.0 && editable {
            HOVER
        } else {
            Color32::from_rgb(20, 27, 22)
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(13),
        Stroke::new(1.0_f32, if enabled { active_color } else { BORDER }),
        egui::StrokeKind::Inside,
    );
    let knob_x = egui::lerp((rect.left() + 12.0)..=(rect.right() - 12.0), position);
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        8.0,
        if editable {
            Color32::WHITE
        } else {
            Color32::from_rgb(167, 176, 166)
        },
    );
    response.on_hover_text(tooltip).clicked() && editable
}

fn section_heading(ui: &mut egui::Ui, glyph: LauncherIcon, label: &str, accent: Color32) {
    icon_label(ui, glyph, label, 17.0, 13.0, accent, true);
}

fn icon_label(
    ui: &mut egui::Ui,
    glyph: LauncherIcon,
    label: &str,
    icon_size: f32,
    text_size: f32,
    color: Color32,
    strong: bool,
) {
    ui.horizontal(|ui| {
        draw_inline_icon(ui, glyph, icon_size, color);
        let text = RichText::new(label).size(text_size).color(TEXT);
        if strong {
            ui.label(text.strong());
        } else {
            ui.label(text);
        }
    });
}

fn theme_swatches(ui: &mut egui::Ui, current: &str, texts: &Texts, mut choose: impl FnMut(&str)) {
    ui.horizontal_wrapped(|ui| {
        for (theme, color, label) in [
            ("default", DEFAULT_ACCENT, texts.theme_default),
            (
                "midnight",
                Color32::from_rgb(65, 145, 255),
                texts.theme_midnight,
            ),
            (
                "violet",
                Color32::from_rgb(177, 91, 255),
                texts.theme_violet,
            ),
        ] {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            let selected = current == theme;
            let hovered = ui
                .ctx()
                .animate_bool_with_time(response.id, response.hovered(), 0.14);
            ui.painter()
                .circle_filled(rect.center(), 9.0 + hovered * 1.5, color);
            if selected {
                ui.painter()
                    .circle_stroke(rect.center(), 13.0, Stroke::new(1.5_f32, TEXT));
            }
            let clicked = response.clicked();
            response.on_hover_text(label);
            if clicked && !selected {
                choose(theme);
            }
        }
    });
}

fn icon_action_button(
    ui: &mut egui::Ui,
    label: &str,
    glyph: LauncherIcon,
    accent: Color32,
) -> bool {
    let size = Vec2::new(ui.available_width().min(258.0), 36.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.16);
    ui.painter().rect(
        rect,
        CornerRadius::same(5),
        if hovered > 0.0 {
            HOVER
        } else {
            Color32::from_rgb(19, 28, 21)
        },
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    draw_icon(
        ui.painter(),
        glyph,
        egui::pos2(rect.left() + 17.0, rect.center().y),
        15.0,
        accent,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        TEXT,
    );
    draw_icon(
        ui.painter(),
        LauncherIcon::Play,
        egui::pos2(rect.right() - 14.0, rect.center().y),
        10.0,
        accent,
    );
    response.clicked()
}

fn compact_icon_button(
    ui: &mut egui::Ui,
    glyph: LauncherIcon,
    label: &str,
    accent: Color32,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(44.0, 50.0), egui::Sense::click());
    let hovered = ui
        .ctx()
        .animate_bool_with_time(response.id, response.hovered(), 0.16);
    ui.painter().rect(
        rect,
        CornerRadius::same(5),
        if hovered > 0.0 {
            HOVER
        } else {
            Color32::from_rgb(19, 28, 21)
        },
        Stroke::new(1.0_f32, BORDER),
        egui::StrokeKind::Inside,
    );
    draw_icon(
        ui.painter(),
        glyph,
        rect.center(),
        18.0,
        if hovered > 0.0 { accent } else { TEXT },
    );
    response.on_hover_text(label).clicked()
}

fn draw_icon(
    painter: &egui::Painter,
    glyph: LauncherIcon,
    center: egui::Pos2,
    size: f32,
    color: Color32,
) {
    let stroke = Stroke::new((size / 10.0).max(1.2), color);
    let half = size * 0.42;
    let point = |x: f32, y: f32| egui::pos2(center.x + x * half, center.y + y * half);
    let line = |from: (f32, f32), to: (f32, f32)| {
        painter.line_segment([point(from.0, from.1), point(to.0, to.1)], stroke);
    };
    let outline = |rect: egui::Rect| {
        painter.rect_stroke(
            rect,
            CornerRadius::same(2),
            stroke,
            egui::StrokeKind::Inside,
        );
    };
    match glyph {
        LauncherIcon::Home => {
            line((-1.0, -0.05), (0.0, -0.9));
            line((0.0, -0.9), (1.0, -0.05));
            outline(egui::Rect::from_center_size(
                point(0.0, 0.42),
                Vec2::new(half * 1.55, half * 1.1),
            ));
            line((0.0, 0.32), (0.0, 0.95));
        }
        LauncherIcon::Mods => {
            outline(egui::Rect::from_center_size(
                center,
                Vec2::splat(half * 0.82),
            ));
            line((-1.0, 0.0), (-0.42, 0.0));
            line((0.42, 0.0), (1.0, 0.0));
            line((0.0, -1.0), (0.0, -0.42));
            line((0.0, 0.42), (0.0, 1.0));
        }
        LauncherIcon::Settings | LauncherIcon::Config => {
            for (y, knob) in [(-0.65, -0.3), (0.0, 0.42), (0.65, -0.05)] {
                line((-0.9, y), (0.9, y));
                painter.circle_filled(point(knob, y), size * 0.105, BACKGROUND);
                painter.circle_stroke(point(knob, y), size * 0.105, stroke);
            }
        }
        LauncherIcon::Library => {
            line((0.0, -0.8), (0.0, 0.8));
            line((-0.9, -0.65), (-0.15, -0.85));
            line((-0.15, -0.85), (-0.15, 0.65));
            line((-0.15, 0.65), (-0.9, 0.48));
            line((-0.9, 0.48), (-0.9, -0.65));
            line((0.15, -0.85), (0.9, -0.65));
            line((0.9, -0.65), (0.9, 0.48));
            line((0.9, 0.48), (0.15, 0.65));
            line((0.15, 0.65), (0.15, -0.85));
        }
        LauncherIcon::News => {
            outline(egui::Rect::from_center_size(
                center,
                Vec2::new(half * 1.45, half * 1.75),
            ));
            line((-0.48, -0.35), (0.48, -0.35));
            line((-0.48, 0.05), (0.48, 0.05));
            line((-0.48, 0.45), (0.23, 0.45));
        }
        LauncherIcon::Game => {
            painter.rect_stroke(
                egui::Rect::from_center_size(center, Vec2::new(half * 1.8, half * 1.08)),
                CornerRadius::same(5),
                stroke,
                egui::StrokeKind::Inside,
            );
            line((-0.6, 0.0), (-0.15, 0.0));
            line((-0.38, -0.25), (-0.38, 0.25));
            painter.circle_filled(point(0.4, -0.1), size * 0.08, color);
            painter.circle_filled(point(0.64, 0.16), size * 0.08, color);
        }
        LauncherIcon::World | LauncherIcon::Language => {
            painter.circle_stroke(center, half * 0.9, stroke);
            painter.circle_stroke(
                center,
                Vec2::new(half * 0.4, half * 0.9).length(),
                Stroke::new(stroke.width * 0.75, color),
            );
            line((-0.82, 0.0), (0.82, 0.0));
            line((-0.65, -0.48), (0.65, -0.48));
            line((-0.65, 0.48), (0.65, 0.48));
        }
        LauncherIcon::ResourcePack => {
            line((0.0, -0.92), (0.86, -0.42));
            line((0.86, -0.42), (0.0, 0.05));
            line((0.0, 0.05), (-0.86, -0.42));
            line((-0.86, -0.42), (0.0, -0.92));
            line((-0.86, -0.42), (-0.86, 0.48));
            line((-0.86, 0.48), (0.0, 0.94));
            line((0.0, 0.94), (0.86, 0.48));
            line((0.86, 0.48), (0.86, -0.42));
            line((0.0, 0.05), (0.0, 0.94));
        }
        LauncherIcon::Folder => {
            line((-0.9, -0.55), (-0.25, -0.55));
            line((-0.25, -0.55), (-0.03, -0.25));
            line((-0.03, -0.25), (0.9, -0.25));
            line((0.9, -0.25), (0.77, 0.72));
            line((0.77, 0.72), (-0.9, 0.72));
            line((-0.9, 0.72), (-0.9, -0.55));
        }
        LauncherIcon::Theme => {
            painter.circle_stroke(center, half * 0.85, stroke);
            for (x, y) in [(-0.25, -0.22), (0.28, -0.28), (-0.3, 0.3), (0.28, 0.3)] {
                painter.circle_filled(point(x, y), size * 0.085, color);
            }
        }
        LauncherIcon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![point(-0.5, -0.72), point(0.72, 0.0), point(-0.5, 0.72)],
                color,
                Stroke::NONE,
            ));
        }
    }
}

fn landscape_art(ui: &mut egui::Ui, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(240.0, 150.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(9), Color32::from_rgb(19, 29, 68));
    painter.circle_filled(
        egui::pos2(rect.right() - 47.0, rect.top() + 40.0),
        23.0,
        Color32::from_rgb(255, 147, 160),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(rect.left(), rect.bottom() - 52.0),
            egui::pos2(rect.left() + 56.0, rect.top() + 51.0),
            egui::pos2(rect.left() + 112.0, rect.bottom() - 42.0),
            egui::pos2(rect.left() + 165.0, rect.top() + 73.0),
            egui::pos2(rect.right(), rect.bottom() - 45.0),
            egui::pos2(rect.right(), rect.bottom()),
            egui::pos2(rect.left(), rect.bottom()),
        ],
        Color32::from_rgb(31, 47, 91),
        Stroke::NONE,
    ));
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(rect.left(), rect.bottom() - 31.0),
            rect.right_bottom(),
        ),
        0,
        accent.gamma_multiply(0.22),
    );
}

fn news_art(ui: &mut egui::Ui, accent: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(144.0, 100.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(8), Color32::from_rgb(34, 17, 63));
    painter.circle_filled(
        egui::pos2(rect.center().x, rect.center().y),
        30.0,
        accent.gamma_multiply(0.32),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "N",
        FontId::proportional(42.0),
        Color32::from_rgb(202, 151, 255),
    );
}

fn display_mod_name(filename: &str) -> String {
    let jar_filename = mod_filename_state(filename)
        .map(|(jar_filename, _)| jar_filename)
        .unwrap_or(filename);
    jar_filename
        .strip_suffix(".jar")
        .or_else(|| jar_filename.strip_suffix(".JAR"))
        .unwrap_or(jar_filename)
        .replace(['_', '-'], " ")
}

fn settings_path(root: &Path) -> PathBuf {
    root.join("launcher.json")
}

fn load_settings(path: &Path) -> (Settings, Option<String>) {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<Settings>(&contents) {
            Ok(settings) => (settings, None),
            Err(error) => (
                Settings::default(),
                Some(format!("Could not read launcher settings: {error}")),
            ),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (Settings::default(), None),
        Err(error) => (
            Settings::default(),
            Some(format!("Could not read launcher settings: {error}")),
        ),
    }
}

fn load_texture(
    ctx: &egui::Context,
    path: &Path,
    texture_name: &str,
) -> Result<Option<TextureHandle>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let image = image::open(path)
        .map_err(|error| format!("Could not load {}: {error}", path.display()))?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    Ok(Some(ctx.load_texture(
        texture_name,
        color_image,
        TextureOptions::LINEAR,
    )))
}

fn build_classpath(game_dir: &Path) -> Result<String, String> {
    let client = game_dir
        .join("libraries")
        .join("minecraft-1.12.2-client.jar");
    if !client.is_file() {
        return Err(format!(
            "Minecraft client library is missing: {}",
            client.display()
        ));
    }
    let libraries = game_dir.join("libraries");
    let mut classpath = vec![client.to_string_lossy().into_owned()];
    let mut missing = Vec::new();
    for name in REQUIRED_JARS {
        match find_jar(&libraries, name)? {
            Some(path) => classpath.push(path.to_string_lossy().into_owned()),
            None => missing.push(*name),
        }
    }
    if !missing.is_empty() {
        return Err(format!("Missing libraries: {}", missing.join(", ")));
    }
    let separator = if cfg!(windows) { ";" } else { ":" };
    Ok(classpath.join(separator))
}

fn find_jar(directory: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let entries =
        fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|file| file.to_str()) == Some(name) {
            return Ok(Some(path));
        }
        if path.is_dir() {
            if let Some(found) = find_jar(&path, name)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ArabiaLauncher 1.12.2")
            .with_inner_size([1365.0, 768.0])
            .with_min_inner_size([920.0, 620.0])
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "ArabiaLauncher",
        options,
        Box::new(|_cc| Ok(Box::new(LauncherApp::new()))),
    )
}

fn window_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../../img/app-icon.png"))
        .expect("bundled launcher icon should be a valid PNG")
        .to_rgba8();
    let width = image.width();
    let height = image.height();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn mod_display_name_removes_archive_suffix_and_separators() {
        assert_eq!(
            display_mod_name("OptiFine_1.12.2_HD_U_G5.JAR"),
            "OptiFine 1.12.2 HD U G5"
        );
        assert_eq!(display_mod_name("Example_Mod.jar.disabled"), "Example Mod");
    }

    #[test]
    fn mod_filename_state_recognizes_enabled_and_disabled_archives() {
        assert_eq!(mod_filename_state("mod.jar"), Some(("mod.jar", false)));
        assert_eq!(
            mod_filename_state("mod.JAR.DISABLED"),
            Some(("mod.JAR", true))
        );
        assert_eq!(mod_filename_state("not-a-mod.zip"), None);
    }

    #[test]
    fn disabling_and_enabling_a_mod_round_trips_its_filename() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let temporary_root = std::env::temp_dir().join(format!(
            "arabia-launcher-mod-toggle-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&temporary_root).expect("create temporary mods folder");
        fs::write(temporary_root.join("example.jar"), []).expect("create mod file");

        assert!(toggle_mod_file(&temporary_root, "example.jar").expect("disable mod"));
        assert!(temporary_root.join("example.jar.disabled").is_file());
        assert!(!temporary_root.join("example.jar").exists());
        assert!(!toggle_mod_file(&temporary_root, "example.jar.disabled").expect("enable mod"));
        assert!(temporary_root.join("example.jar").is_file());
        fs::remove_dir_all(temporary_root).expect("remove temporary mods folder");
    }

    #[test]
    fn mod_toggle_refuses_to_overwrite_an_existing_destination() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let temporary_root = std::env::temp_dir().join(format!(
            "arabia-launcher-mod-collision-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&temporary_root).expect("create temporary mods folder");
        fs::write(temporary_root.join("example.jar"), []).expect("create mod file");
        fs::write(
            temporary_root.join("example.jar.disabled"),
            b"preserve existing disabled file",
        )
        .expect("create conflicting destination");

        assert!(toggle_mod_file(&temporary_root, "example.jar").is_err());
        assert!(temporary_root.join("example.jar").is_file());
        assert_eq!(
            fs::read(temporary_root.join("example.jar.disabled"))
                .expect("read existing disabled file"),
            b"preserve existing disabled file"
        );
        fs::remove_dir_all(temporary_root).expect("remove temporary mods folder");
    }

    #[test]
    fn launcher_settings_are_kept_in_the_portable_folder() {
        let portable_root = Path::new("portable-launcher");
        assert_eq!(
            settings_path(portable_root),
            portable_root.join("launcher.json")
        );
    }

    #[test]
    fn classpath_includes_the_client_and_all_required_libraries() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();
        let temporary_root = std::env::temp_dir().join(format!(
            "arabia-launcher-classpath-test-{}-{unique}",
            std::process::id()
        ));
        let libraries = temporary_root.join("libraries");
        fs::create_dir_all(&libraries).expect("create temporary libraries");
        fs::write(libraries.join("minecraft-1.12.2-client.jar"), []).expect("create client jar");
        for jar in REQUIRED_JARS {
            fs::write(libraries.join(jar), []).expect("create dependency jar");
        }

        let classpath = build_classpath(&temporary_root).expect("all classpath entries exist");
        assert!(classpath.contains("minecraft-1.12.2-client.jar"));
        for jar in REQUIRED_JARS {
            assert!(classpath.contains(jar));
        }
        fs::remove_dir_all(temporary_root).expect("remove temporary test folder");
    }
}
