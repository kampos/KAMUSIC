use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use adw::prelude::*;
use gtk::{gio, glib};

use crate::audio::player::Player;
use crate::config::settings::Settings;
use crate::library::database::LibraryDatabase;
use crate::library::models::{Library, Track};
use crate::library::online::{self, OnlineItem, OnlineKind};
use crate::library::scanner;
use crate::mpris::MprisControls;
use crate::util::{errors::show_toast, paths};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveSection {
    Local,
    Favorites,
    Radio,
}

struct UiState {
    library: Library,
    visible_tracks: Vec<Track>,
    visible_favorites: Vec<Track>,
    visible_radio: Vec<OnlineItem>,
    queue: Vec<Track>,
    online_queue: Vec<OnlineItem>,
    current_index: Option<usize>,
    is_playing: bool,
    active_section: ActiveSection,
    selected_folder: Option<PathBuf>,
    search_token: u64,
    current_query: String,
    settings: Settings,
}

const ONLINE_ITEM_KEY: &str = "kamusic-online-item";

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        install_css();

        let settings_path = paths::config_file();
        let db_path = paths::database_file();
        let settings = Settings::load(&settings_path);
        let database = LibraryDatabase::open(&db_path).ok();
        let library = database
            .as_ref()
            .and_then(|db| db.load().ok())
            .unwrap_or_default();
        let player = Rc::new(Player::new().ok());
        if let Some(player) = player.as_ref() {
            player.set_volume(settings.volume);
        }
        let mpris = MprisControls::default();
        let initial_width = settings.window_width.filter(|value| *value > 0).unwrap_or(1360);
        let initial_height = settings.window_height.filter(|value| *value > 0).unwrap_or(860);

        let state = Rc::new(RefCell::new(UiState {
            visible_tracks: library.tracks.clone(),
            visible_favorites: Vec::new(),
            visible_radio: Vec::new(),
            library,
            queue: Vec::new(),
            online_queue: Vec::new(),
            current_index: None,
            is_playing: false,
            active_section: ActiveSection::Local,
            selected_folder: None,
            search_token: 0,
            current_query: String::new(),
            settings,
        }));
        rebuild_visible_favorites(&state);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("KAMUSIC")
            .default_width(initial_width)
            .default_height(initial_height)
            .build();

        let overlay = adw::ToastOverlay::new();
        let shell = gtk::Box::new(gtk::Orientation::Vertical, 0);
        shell.add_css_class("app-shell");
        overlay.set_child(Some(&shell));
        window.set_content(Some(&overlay));

        {
            let state = Rc::clone(&state);
            let settings_path = settings_path.clone();
            window.connect_close_request(move |window| {
                persist_window_size(window, &state, &settings_path);
                glib::Propagation::Proceed
            });
        }
        {
            let state = Rc::clone(&state);
            let settings_path = settings_path.clone();
            window.connect_notify_local(Some("width"), move |window, _| {
                persist_window_size(window, &state, &settings_path);
            });
        }
        {
            let state = Rc::clone(&state);
            let settings_path = settings_path.clone();
            window.connect_notify_local(Some("height"), move |window, _| {
                persist_window_size(window, &state, &settings_path);
            });
        }

        let drag_click = gtk::GestureClick::new();
        drag_click.set_button(0);
        {
            let window = window.clone();
            let shell_for_drag = shell.clone();
            drag_click.connect_pressed(move |gesture, _n_press, x, y| {
                if gesture.current_button() != 1 {
                    return;
                }

                let Some(picked) = shell_for_drag.pick(x, y, gtk::PickFlags::DEFAULT) else {
                    return;
                };

                if picked.is::<gtk::Button>()
                    || picked.is::<gtk::SearchEntry>()
                    || picked.is::<gtk::Entry>()
                    || picked.is::<gtk::ListBox>()
                    || picked.is::<gtk::ListBoxRow>()
                    || picked.is::<gtk::Label>()
                    || picked.is::<gtk::Image>()
                    || picked.is::<gtk::Picture>()
                {
                    return;
                }

                let device = match gesture.current_event_device() {
                    Some(device) => device,
                    None => return,
                };
                let timestamp = gesture.current_event_time();
                let Some(native) = window.native() else {
                    return;
                };
                let Some(surface) = native.surface() else {
                    return;
                };
                let Ok(toplevel) = surface.dynamic_cast::<gtk::gdk::Toplevel>() else {
                    return;
                };

                toplevel.begin_move(&device, 1, x, y, timestamp);
            });
        }
        shell.add_controller(drag_click);

        let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        body.add_css_class("workspace");
        body.set_hexpand(true);
        body.set_vexpand(true);

        let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 14);
        sidebar.add_css_class("sidebar-panel");
        sidebar.set_width_request(286);

        let brand = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        brand.add_css_class("brand-shell");
        let brand_icon = app_logo_image();
        brand_icon.add_css_class("brand-icon");
        brand_icon.set_pixel_size(26);
        let brand_text = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let title = gtk::Label::new(Some("KAMUSIC"));
        title.add_css_class("brand-title");
        title.set_xalign(0.0);
        let subtitle = gtk::Label::new(Some("Biblioteca local"));
        subtitle.add_css_class("brand-subtitle");
        subtitle.set_xalign(0.0);
        brand_text.append(&title);
        brand_text.append(&subtitle);
        brand.append(&brand_icon);
        brand.append(&brand_text);
        sidebar.append(&brand);

        let sidebar_scroll = gtk::ScrolledWindow::new();
        sidebar_scroll.set_vexpand(true);
        sidebar_scroll.add_css_class("sidebar-scroll");

        let sidebar_stack = gtk::Box::new(gtk::Orientation::Vertical, 16);
        sidebar_stack.add_css_class("sidebar-stack");

        let nav_label = gtk::Label::new(Some("NAVEGACION"));
        nav_label.set_xalign(0.0);
        nav_label.add_css_class("section-kicker");
        sidebar_stack.append(&nav_label);

        let nav_list = gtk::ListBox::new();
        nav_list.add_css_class("sidebar-list");
        nav_list.set_selection_mode(gtk::SelectionMode::Single);
        sidebar_stack.append(&nav_list);

        let playlist_header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let playlist_label = gtk::Label::new(Some("PLAYLISTS"));
        playlist_label.set_xalign(0.0);
        playlist_label.add_css_class("section-kicker");
        let add_playlist = icon_button("plus.svg", "Crear playlist");
        playlist_header.append(&playlist_label);
        playlist_header.append(&add_playlist);
        sidebar_stack.append(&playlist_header);

        let folder_list = gtk::ListBox::new();
        folder_list.add_css_class("playlist-list");
        folder_list.set_selection_mode(gtk::SelectionMode::Single);
        sidebar_stack.append(&folder_list);

        sidebar_scroll.set_child(Some(&sidebar_stack));
        sidebar.append(&sidebar_scroll);

        body.append(&sidebar);

        let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
        content.add_css_class("content-panel");
        content.set_hexpand(true);
        content.set_vexpand(true);

        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        toolbar.add_css_class("topbar");

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some("Buscar canciones, artistas, albumes..."));
        search.add_css_class("global-search");
        search.set_hexpand(true);
        toolbar.append(&search);

        let scan_button = icon_button("refresh.svg", "Escanear musica");
        let folder_button = icon_button("folder-open.svg", "Seleccionar carpeta");
        let close_button = icon_button("close.svg", "Cerrar aplicacion");
        toolbar.append(&scan_button);
        toolbar.append(&folder_button);
        toolbar.append(&close_button);
        content.append(&toolbar);

        let hero = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        hero.add_css_class("hero-panel");
        hero.set_hexpand(true);

        let hero_cover = app_cover_image();
        hero_cover.set_pixel_size(252);
        hero_cover.add_css_class("hero-cover");

        let hero_video = gtk::Picture::new();
        hero_video.set_can_shrink(true);
        hero_video.set_content_fit(gtk::ContentFit::Contain);
        hero_video.set_hexpand(true);
        hero_video.set_vexpand(true);
        hero_video.add_css_class("hero-video");

        let hero_media = gtk::Stack::new();
        hero_media.add_named(&hero_cover, Some("cover"));
        hero_media.add_named(&hero_video, Some("video"));
        hero_media.set_visible_child_name("cover");
        hero.append(&hero_media);

        let hero_copy = gtk::Box::new(gtk::Orientation::Vertical, 10);
        hero_copy.set_hexpand(true);
        hero_copy.set_valign(gtk::Align::Center);
        hero_copy.add_css_class("hero-copy");

        let hero_kicker = gtk::Label::new(Some("BIBLIOTECA"));
        hero_kicker.set_xalign(0.0);
        hero_kicker.add_css_class("hero-kicker");

        let hero_title = gtk::Label::new(Some("Tu musica local"));
        hero_title.set_xalign(0.0);
        hero_title.set_wrap(true);
        hero_title.add_css_class("hero-title");

        let hero_subtitle = gtk::Label::new(Some("Escanea una carpeta para ver tu coleccion"));
        hero_subtitle.set_xalign(0.0);
        hero_subtitle.set_wrap(true);
        hero_subtitle.add_css_class("hero-subtitle");

        let hero_meta = gtk::Label::new(Some("0 canciones · 0 carpetas"));
        hero_meta.set_xalign(0.0);
        hero_meta.add_css_class("hero-meta");

        let status_label = gtk::Label::new(Some("Biblioteca lista"));
        status_label.set_xalign(0.0);
        status_label.add_css_class("hero-status");

        let stat_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        stat_box.add_css_class("hero-stats");
        let stat_number = gtk::Label::new(Some(&state.borrow().library.tracks.len().to_string()));
        stat_number.add_css_class("hero-stat-number");
        let stat_text = gtk::Label::new(Some("canciones"));
        stat_text.add_css_class("hero-stat-text");
        stat_box.append(&stat_number);
        stat_box.append(&stat_text);

        let hero_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        hero_actions.add_css_class("hero-actions");
        let hero_play_button = command_button("Reproducir", "play.svg");
        hero_play_button.add_css_class("suggested-action");
        let hero_shuffle_button = command_button("Aleatorio", "media-playlist-shuffle-symbolic");
        hero_shuffle_button.add_css_class("secondary-action");
        hero_actions.append(&hero_play_button);
        hero_actions.append(&hero_shuffle_button);

        hero_copy.append(&hero_kicker);
        hero_copy.append(&hero_title);
        hero_copy.append(&hero_subtitle);
        hero_copy.append(&hero_meta);
        hero_copy.append(&status_label);
        hero_copy.append(&stat_box);
        hero_copy.append(&hero_actions);
        hero.append(&hero_copy);
        content.append(&hero);

        let track_header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        track_header.add_css_class("track-header");
        let header_number = gtk::Label::new(Some("#"));
        header_number.add_css_class("track-header-cell");
        header_number.add_css_class("track-header-number");
        let header_title = gtk::Label::new(Some("TITULO"));
        header_title.add_css_class("track-header-cell");
        header_title.set_hexpand(true);
        header_title.set_xalign(0.0);
        let header_artist = gtk::Label::new(Some("ARTISTA"));
        header_artist.add_css_class("track-header-cell");
        header_artist.set_width_chars(18);
        header_artist.set_xalign(0.0);
        let header_folder = gtk::Label::new(Some("CARPETA"));
        header_folder.add_css_class("track-header-cell");
        header_folder.set_width_chars(18);
        header_folder.set_xalign(0.0);
        let header_size = gtk::Label::new(Some("TAMANO"));
        header_size.add_css_class("track-header-cell");
        header_size.set_xalign(1.0);
        track_header.append(&header_number);
        track_header.append(&header_title);
        track_header.append(&header_artist);
        track_header.append(&header_folder);
        track_header.append(&header_size);
        content.append(&track_header);

        let track_list = gtk::ListBox::new();
        track_list.set_activate_on_single_click(true);
        track_list.add_css_class("track-list");
        let track_scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .child(&track_list)
            .build();
        content.append(&track_scroll);

        body.append(&content);
        shell.append(&body);

        let player_bar = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        player_bar.add_css_class("player-bar");

        let cover = app_cover_image();
        cover.set_pixel_size(60);
        cover.add_css_class("now-cover");

        let now_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        now_box.set_hexpand(true);
        let now_title = gtk::Label::new(Some("Sin reproduccion"));
        now_title.set_xalign(0.0);
        now_title.add_css_class("heading");
        let now_subtitle = gtk::Label::new(Some("Selecciona una cancion para empezar"));
        now_subtitle.set_xalign(0.0);
        now_subtitle.add_css_class("dim-label");
        now_box.append(&now_title);
        now_box.append(&now_subtitle);

        let player_left = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        player_left.set_hexpand(true);
        player_left.append(&cover);
        player_left.append(&now_box);
        player_bar.append(&player_left);

        let player_controls = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        player_controls.add_css_class("player-controls");
        let control_spacer_left = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        control_spacer_left.set_hexpand(true);
        let control_spacer_right = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        control_spacer_right.set_hexpand(true);

        let prev_button = icon_button("skip-backward.svg", "Anterior");
        let play_button = icon_button("play.svg", "Reproducir");
        let pause_button = icon_button("pause.svg", "Pausar");
        let stop_button = icon_button("stop.svg", "Detener");
        let next_button = icon_button("skip-forward.svg", "Siguiente");
        for button in [
            &prev_button,
            &play_button,
            &pause_button,
            &stop_button,
            &next_button,
        ] {
            button.add_css_class("player-control");
            player_controls.append(button);
        }
        play_button.add_css_class("main-play");
        player_bar.append(&control_spacer_left);
        player_bar.append(&player_controls);
        player_bar.append(&control_spacer_right);

        let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.01);
        volume.set_width_request(120);
        volume.set_value(state.borrow().settings.volume);
        volume.set_draw_value(false);
        let player_right = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        player_right.append(&volume);
        player_bar.append(&player_right);
        shell.append(&player_bar);

        render_sidebar(&nav_list, &folder_list, &state.borrow().library);
        render_tracks(
            &track_list,
            &state.borrow().visible_tracks,
            &state,
            &search,
            &hero_media,
            &hero_kicker,
            &hero_title,
            &hero_subtitle,
            &hero_meta,
            &hero_cover,
            &status_label,
            &stat_number,
            &overlay,
            &settings_path,
        );
        refresh_featured_panel(
            &state.borrow().visible_tracks,
            &state.borrow().library,
            &hero_media,
            &hero_kicker,
            &hero_title,
            &hero_subtitle,
            &hero_meta,
            &hero_cover,
            ActiveSection::Local,
        );

        let library_is_empty = state.borrow().library.tracks.is_empty();
        let initial_music_dir = preferred_music_dir(&state.borrow().settings);

        if library_is_empty {
            if let Some(root) = initial_music_dir {
                {
                    let mut state_mut = state.borrow_mut();
                    state_mut.settings.last_music_dir = Some(root.clone());
                    let _ = state_mut.settings.save(&settings_path);
                }
                start_scan(
                    root,
                    &state,
                    &nav_list,
                    &folder_list,
                    &track_list,
                    &hero_media,
                    search.clone(),
                    hero_kicker.clone(),
                    hero_title.clone(),
                    hero_subtitle.clone(),
                    hero_meta.clone(),
                    hero_cover.clone(),
                    &status_label,
                    &stat_number,
                    &overlay,
                    settings_path.clone(),
                    db_path.clone(),
                );
            }
        }

        {
            let state = Rc::clone(&state);
            let track_list = track_list.clone();
            let hero_kicker_for_search = hero_kicker.clone();
            let hero_title_for_search = hero_title.clone();
            let hero_subtitle_for_search = hero_subtitle.clone();
            let hero_meta_for_search = hero_meta.clone();
            let hero_cover_for_search = hero_cover.clone();
            let hero_media_for_search = hero_media.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let search_for_handler = search.clone();
            let search_for_view = search.clone();
            let overlay_for_search = overlay.clone();
            let settings_path_for_search = settings_path.clone();
            search_for_handler.connect_search_changed(move |entry| {
                let query = entry.text().to_string();
                state.borrow_mut().current_query = query.clone();
                let active_section = state.borrow().active_section;
                match active_section {
                    ActiveSection::Local => {
                        let library = state.borrow().library.clone();
                        let tracks = library.search(&query);
                        state.borrow_mut().visible_tracks = tracks.clone();
                        refresh_current_view(
                            &state,
                            &track_list,
                            &search_for_view,
                            &hero_media_for_search,
                            &hero_kicker_for_search,
                            &hero_title_for_search,
                            &hero_subtitle_for_search,
                            &hero_meta_for_search,
                            &hero_cover_for_search,
                            &status_label,
                            &stat_number,
                            &overlay_for_search,
                            &settings_path_for_search,
                        );
                        status_label.set_label(&format!("{} canciones encontradas", tracks.len()));
                    }
                    ActiveSection::Favorites => {
                        rebuild_visible_favorites(&state);
                        refresh_current_view(
                            &state,
                            &track_list,
                            &search_for_view,
                            &hero_media_for_search,
                            &hero_kicker_for_search,
                            &hero_title_for_search,
                            &hero_subtitle_for_search,
                            &hero_meta_for_search,
                            &hero_cover_for_search,
                            &status_label,
                            &stat_number,
                            &overlay_for_search,
                            &settings_path_for_search,
                        );
                    }
                    ActiveSection::Radio => {
                        if query.trim().is_empty() {
                            let items = state.borrow().visible_radio.clone();
                            if items.is_empty() {
                                render_online_items(&track_list, &[]);
                                stat_number.set_label("0");
                                status_label.set_label("Busca emisoras y pulsa Enter");
                            } else {
                                render_online_items(&track_list, &items);
                                stat_number.set_label(&items.len().to_string());
                                status_label.set_label("Emisoras de España");
                            }
                        } else {
                            status_label.set_label("Pulsa Enter para buscar emisoras");
                        }
                    }
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let track_list = track_list.clone();
            let hero_kicker_for_activate = hero_kicker.clone();
            let hero_title_for_activate = hero_title.clone();
            let hero_subtitle_for_activate = hero_subtitle.clone();
            let hero_meta_for_activate = hero_meta.clone();
            let hero_cover_for_activate = hero_cover.clone();
            let hero_media_for_activate = hero_media.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let overlay_for_activate = overlay.clone();
            let search_for_handler = search.clone();
            let search_for_view = search.clone();
            let settings_path_for_activate = settings_path.clone();
            search_for_handler.connect_activate(move |entry| {
                let query = entry.text().to_string();
                state.borrow_mut().current_query = query.clone();
                let active_section = state.borrow().active_section;
                match active_section {
                    ActiveSection::Local => {
                        let library = state.borrow().library.clone();
                        let tracks = library.search(&query);
                        state.borrow_mut().visible_tracks = tracks.clone();
                        refresh_current_view(
                            &state,
                            &track_list,
                            &search_for_view,
                            &hero_media_for_activate,
                            &hero_kicker_for_activate,
                            &hero_title_for_activate,
                            &hero_subtitle_for_activate,
                            &hero_meta_for_activate,
                            &hero_cover_for_activate,
                            &status_label,
                            &stat_number,
                            &overlay_for_activate,
                            &settings_path_for_activate,
                        );
                        status_label.set_label(&format!("{} canciones encontradas", tracks.len()));
                    }
                    ActiveSection::Favorites => {
                        rebuild_visible_favorites(&state);
                        refresh_current_view(
                            &state,
                            &track_list,
                            &search_for_view,
                            &hero_media_for_activate,
                            &hero_kicker_for_activate,
                            &hero_title_for_activate,
                            &hero_subtitle_for_activate,
                            &hero_meta_for_activate,
                            &hero_cover_for_activate,
                            &status_label,
                            &stat_number,
                            &overlay_for_activate,
                            &settings_path_for_activate,
                        );
                    }
                    ActiveSection::Radio => {
                        start_online_search(
                            ActiveSection::Radio,
                            query,
                            &state,
                            &track_list,
                            &hero_media_for_activate,
                            hero_kicker_for_activate.clone(),
                            hero_title_for_activate.clone(),
                            hero_subtitle_for_activate.clone(),
                            hero_meta_for_activate.clone(),
                            hero_cover_for_activate.clone(),
                            &status_label,
                            &stat_number,
                            &overlay_for_activate,
                        );
                    }
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let track_list = track_list.clone();
            let hero_kicker_for_nav = hero_kicker.clone();
            let hero_title_for_nav = hero_title.clone();
            let hero_subtitle_for_nav = hero_subtitle.clone();
            let hero_meta_for_nav = hero_meta.clone();
            let hero_cover_for_nav = hero_cover.clone();
            let hero_media_for_nav = hero_media.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let settings_path_for_nav = settings_path.clone();
            let search = search.clone();
            let overlay = overlay.clone();
            nav_list.connect_row_activated(move |_list, row| {
                let index = row.index();
                let section = match index {
                    0 => ActiveSection::Local,
                    1 => ActiveSection::Favorites,
                    2 => ActiveSection::Radio,
                    _ => ActiveSection::Local,
                };
                set_active_section(
                    section,
                    &state,
                    &track_list,
                    &hero_media_for_nav,
                    &hero_kicker_for_nav,
                    &hero_title_for_nav,
                    &hero_subtitle_for_nav,
                    &hero_meta_for_nav,
                    &hero_cover_for_nav,
                    &search,
                    &status_label,
                    &stat_number,
                    &overlay,
                    &settings_path_for_nav,
                );
                let visible_radio_empty = state.borrow().visible_radio.is_empty();
                if section == ActiveSection::Radio && visible_radio_empty {
                    start_online_search(
                        ActiveSection::Radio,
                        String::new(),
                        &state,
                        &track_list,
                        &hero_media_for_nav,
                        hero_kicker_for_nav.clone(),
                        hero_title_for_nav.clone(),
                        hero_subtitle_for_nav.clone(),
                        hero_meta_for_nav.clone(),
                        hero_cover_for_nav.clone(),
                        &status_label,
                        &stat_number,
                        &overlay,
                    );
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let track_list = track_list.clone();
            let hero_kicker_for_folder = hero_kicker.clone();
            let hero_title_for_folder = hero_title.clone();
            let hero_subtitle_for_folder = hero_subtitle.clone();
            let hero_meta_for_folder = hero_meta.clone();
            let hero_cover_for_folder = hero_cover.clone();
            let hero_media_for_folder = hero_media.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let search = search.clone();
            let player_for_folder = Rc::clone(&player);
            let overlay_for_folder = overlay.clone();
            let settings_path_for_folder = settings_path.clone();
            let now_title_for_folder = now_title.clone();
            let now_subtitle_for_folder = now_subtitle.clone();
            let cover_for_folder = cover.clone();
            let mpris_for_folder = mpris.clone();
            folder_list.connect_row_activated(move |_list, row| {
                let index = row.index();
                let folders = state.borrow().library.folders();
                if folders.is_empty() {
                    return;
                }
                let tracks = folders
                    .get(index as usize)
                    .map(|folder| state.borrow().library.tracks_in_folder(folder))
                    .unwrap_or_default();
                {
                    let mut state = state.borrow_mut();
                    state.active_section = ActiveSection::Local;
                    state.selected_folder = folders.get(index as usize).cloned();
                    state.visible_tracks = tracks.clone();
                }
                status_label.set_label("Carpeta seleccionada");
                refresh_current_view(
                    &state,
                    &track_list,
                    &search,
                    &hero_media_for_folder,
                    &hero_kicker_for_folder,
                    &hero_title_for_folder,
                    &hero_subtitle_for_folder,
                    &hero_meta_for_folder,
                    &hero_cover_for_folder,
                    &status_label,
                    &stat_number,
                    &overlay_for_folder,
                    &settings_path_for_folder,
                );
                if !tracks.is_empty() {
                    play_visible_index(
                        0,
                        &state,
                        &player_for_folder,
                        &overlay_for_folder,
                        &now_title_for_folder,
                        &now_subtitle_for_folder,
                        &cover_for_folder,
                        &mpris_for_folder,
                    );
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let hero_media_for_track = hero_media.clone();
            let hero_video_for_track = hero_video.clone();
            let mpris = mpris.clone();
            track_list.connect_row_activated(move |_list, row| {
                let active_section = state.borrow().active_section;
                match active_section {
                    ActiveSection::Local | ActiveSection::Favorites => play_visible_index(
                        row.index() as usize,
                        &state,
                        &player,
                        &overlay,
                        &now_title,
                        &now_subtitle,
                        &cover,
                        &mpris,
                    ),
                    ActiveSection::Radio => {
                        let index = row.index().max(0) as usize;
                        if let Some(item) = online_item_from_row(row) {
                            play_online_item(
                                index,
                                item,
                                &state,
                                &player,
                                &overlay,
                                &now_title,
                                &now_subtitle,
                                &cover,
                                &hero_media_for_track,
                                &hero_video_for_track,
                                &hero_kicker,
                                &hero_title,
                                &hero_subtitle,
                                &hero_meta,
                                &hero_cover,
                                &mpris,
                            );
                        }
                    }
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let hero_media = hero_media.clone();
            let hero_video = hero_video.clone();
            let mpris = mpris.clone();
            next_button.connect_clicked(move |_| {
                play_next_active(
                    &state,
                    &player,
                    &overlay,
                    &now_title,
                    &now_subtitle,
                    &cover,
                    &hero_media,
                    &hero_video,
                    &hero_kicker,
                    &hero_title,
                    &hero_subtitle,
                    &hero_meta,
                    &hero_cover,
                    &mpris,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let hero_media = hero_media.clone();
            let hero_video = hero_video.clone();
            let mpris = mpris.clone();
            prev_button.connect_clicked(move |_| {
                play_previous_active(
                    &state,
                    &player,
                    &overlay,
                    &now_title,
                    &now_subtitle,
                    &cover,
                    &hero_media,
                    &hero_video,
                    &hero_kicker,
                    &hero_title,
                    &hero_subtitle,
                    &hero_meta,
                    &hero_cover,
                    &mpris,
                );
            });
        }

        {
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let state = Rc::clone(&state);
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let hero_media = hero_media.clone();
            let hero_video = hero_video.clone();
            let mpris = mpris.clone();
            play_button.connect_clicked(move |_| {
                request_play(
                    &state,
                    &player,
                    &overlay,
                    &now_title,
                    &now_subtitle,
                    &cover,
                    &hero_media,
                    &hero_video,
                    &hero_kicker,
                    &hero_title,
                    &hero_subtitle,
                    &hero_meta,
                    &hero_cover,
                    &mpris,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let hero_media = hero_media.clone();
            let hero_video = hero_video.clone();
            let mpris = mpris.clone();
            hero_play_button.connect_clicked(move |_| {
                request_play(
                    &state,
                    &player,
                    &overlay,
                    &now_title,
                    &now_subtitle,
                    &cover,
                    &hero_media,
                    &hero_video,
                    &hero_kicker,
                    &hero_title,
                    &hero_subtitle,
                    &hero_meta,
                    &hero_cover,
                    &mpris,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let mpris = mpris.clone();
            hero_shuffle_button.connect_clicked(move |_| {
                play_random_visible_index(
                    &state,
                    &player,
                    &overlay,
                    &now_title,
                    &now_subtitle,
                    &cover,
                    &mpris,
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let mpris = mpris.clone();
            pause_button.connect_clicked(move |_| {
                request_pause(&state, &player, &overlay, &mpris);
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let mpris = mpris.clone();
            stop_button.connect_clicked(move |_| {
                request_stop(&state, &player, &overlay, &mpris);
            });
        }

        {
            let player = Rc::clone(&player);
            let state = Rc::clone(&state);
            let settings_path_for_volume = settings_path.clone();
            volume.connect_value_changed(move |scale| {
                let value = scale.value();
                if let Some(player) = player.as_ref() {
                    player.set_volume(value);
                }
                state.borrow_mut().settings.volume = value;
                let _ = state.borrow().settings.save(&settings_path_for_volume);
            });
        }

        {
            let state = Rc::clone(&state);
            let nav_list = nav_list.clone();
            let folder_list = folder_list.clone();
            let track_list = track_list.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let overlay = overlay.clone();
            let database_path = db_path.clone();
            let hero_kicker_for_scan = hero_kicker.clone();
            let hero_title_for_scan = hero_title.clone();
            let hero_subtitle_for_scan = hero_subtitle.clone();
            let hero_meta_for_scan = hero_meta.clone();
            let hero_cover_for_scan = hero_cover.clone();
            let hero_media_for_scan = hero_media.clone();
            let search_for_scan = search.clone();
            let settings_path_for_scan = settings_path.clone();
            scan_button.connect_clicked(move |_| {
                let root = state
                    .borrow()
                    .settings
                    .last_music_dir
                    .as_ref()
                    .filter(|path| path.is_dir())
                    .cloned()
                    .or_else(scanner::default_music_dir);
                match root {
                    Some(root) => start_scan(
                        root,
                        &state,
                        &nav_list,
                        &folder_list,
                        &track_list,
                        &hero_media_for_scan,
                        search_for_scan.clone(),
                        hero_kicker_for_scan.clone(),
                        hero_title_for_scan.clone(),
                        hero_subtitle_for_scan.clone(),
                        hero_meta_for_scan.clone(),
                        hero_cover_for_scan.clone(),
                        &status_label,
                        &stat_number,
                        &overlay,
                        settings_path_for_scan.clone(),
                        database_path.clone(),
                    ),
                    None => show_toast(&overlay, "No se encontro la carpeta de musica"),
                }
            });
        }

        {
            let state = Rc::clone(&state);
            let nav_list = nav_list.clone();
            let folder_list = folder_list.clone();
            let track_list = track_list.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let overlay = overlay.clone();
            let window = window.clone();
            let database_path = db_path.clone();
            let settings_path = paths::config_file();
            let hero_kicker_for_folder_button = hero_kicker.clone();
            let hero_title_for_folder_button = hero_title.clone();
            let hero_subtitle_for_folder_button = hero_subtitle.clone();
            let hero_meta_for_folder_button = hero_meta.clone();
            let hero_cover_for_folder_button = hero_cover.clone();
            let hero_media_for_folder_button = hero_media.clone();
            let search_for_folder_button = search.clone();
            folder_button.connect_clicked(move |_| {
                open_music_folder_dialog(
                    &window,
                    Rc::clone(&state),
                    nav_list.clone(),
                    folder_list.clone(),
                    track_list.clone(),
                    hero_media_for_folder_button.clone(),
                    search_for_folder_button.clone(),
                    hero_kicker_for_folder_button.clone(),
                    hero_title_for_folder_button.clone(),
                    hero_subtitle_for_folder_button.clone(),
                    hero_meta_for_folder_button.clone(),
                    hero_cover_for_folder_button.clone(),
                    status_label.clone(),
                    stat_number.clone(),
                    overlay.clone(),
                    database_path.clone(),
                    settings_path.clone(),
                );
            });
        }

        {
            let state = Rc::clone(&state);
            let nav_list = nav_list.clone();
            let folder_list = folder_list.clone();
            let track_list = track_list.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let overlay = overlay.clone();
            let window = window.clone();
            let database_path = db_path.clone();
            let settings_path = paths::config_file();
            let hero_kicker_for_add = hero_kicker.clone();
            let hero_title_for_add = hero_title.clone();
            let hero_subtitle_for_add = hero_subtitle.clone();
            let hero_meta_for_add = hero_meta.clone();
            let hero_cover_for_add = hero_cover.clone();
            let hero_media_for_add = hero_media.clone();
            let search_for_add_button = search.clone();
            add_playlist.connect_clicked(move |_| {
                open_music_folder_dialog(
                    &window,
                    Rc::clone(&state),
                    nav_list.clone(),
                    folder_list.clone(),
                    track_list.clone(),
                    hero_media_for_add.clone(),
                    search_for_add_button.clone(),
                    hero_kicker_for_add.clone(),
                    hero_title_for_add.clone(),
                    hero_subtitle_for_add.clone(),
                    hero_meta_for_add.clone(),
                    hero_cover_for_add.clone(),
                    status_label.clone(),
                    stat_number.clone(),
                    overlay.clone(),
                    database_path.clone(),
                    settings_path.clone(),
                );
            });
        }

        {
            let window = window.clone();
            close_button.connect_clicked(move |_| {
                window.close();
            });
        }

        {
            let state = Rc::clone(&state);
            let player = Rc::clone(&player);
            let overlay = overlay.clone();
            let now_title = now_title.clone();
            let now_subtitle = now_subtitle.clone();
            let cover = cover.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let mpris_for_play = mpris.clone();
            let mpris_for_pause = mpris.clone();
            let mpris_for_stop = mpris.clone();
            let mpris_for_next = mpris.clone();
            let mpris_for_previous = mpris.clone();

            if std::env::var_os("SNAP").is_none() {
                mpris.setup(
                    {
                        let state = Rc::clone(&state);
                        let player = Rc::clone(&player);
                        let overlay = overlay.clone();
                        let now_title = now_title.clone();
                        let now_subtitle = now_subtitle.clone();
                        let cover = cover.clone();
                        let hero_kicker = hero_kicker.clone();
                        let hero_title = hero_title.clone();
                        let hero_subtitle = hero_subtitle.clone();
                        let hero_meta = hero_meta.clone();
                        let hero_cover = hero_cover.clone();
                        let hero_media = hero_media.clone();
                        let hero_video = hero_video.clone();
                        move || {
                            request_play(
                                &state,
                                &player,
                                &overlay,
                                &now_title,
                                &now_subtitle,
                                &cover,
                                &hero_media,
                                &hero_video,
                                &hero_kicker,
                                &hero_title,
                                &hero_subtitle,
                                &hero_meta,
                                &hero_cover,
                                &mpris_for_play,
                            );
                        }
                    },
                    {
                        let state = Rc::clone(&state);
                        let player = Rc::clone(&player);
                        let overlay = overlay.clone();
                        let now_title = now_title.clone();
                        let now_subtitle = now_subtitle.clone();
                        let cover = cover.clone();
                        let hero_kicker = hero_kicker.clone();
                        let hero_title = hero_title.clone();
                        let hero_subtitle = hero_subtitle.clone();
                        let hero_meta = hero_meta.clone();
                        let hero_cover = hero_cover.clone();
                        let hero_media = hero_media.clone();
                        let hero_video = hero_video.clone();
                        move || {
                            let is_playing = state.borrow().is_playing;
                            if is_playing {
                                request_pause(&state, &player, &overlay, &mpris_for_pause);
                            } else {
                                request_play(
                                    &state,
                                    &player,
                                    &overlay,
                                    &now_title,
                                    &now_subtitle,
                                    &cover,
                                    &hero_media,
                                    &hero_video,
                                    &hero_kicker,
                                    &hero_title,
                                    &hero_subtitle,
                                    &hero_meta,
                                    &hero_cover,
                                    &mpris_for_pause,
                                );
                            }
                        }
                    },
                    {
                        let state = Rc::clone(&state);
                        let player = Rc::clone(&player);
                        let overlay = overlay.clone();
                        move || request_stop(&state, &player, &overlay, &mpris_for_stop)
                    },
                    {
                        let state = Rc::clone(&state);
                        let player = Rc::clone(&player);
                        let overlay = overlay.clone();
                        let now_title = now_title.clone();
                        let now_subtitle = now_subtitle.clone();
                        let cover = cover.clone();
                        let hero_kicker = hero_kicker.clone();
                        let hero_title = hero_title.clone();
                        let hero_subtitle = hero_subtitle.clone();
                        let hero_meta = hero_meta.clone();
                        let hero_cover = hero_cover.clone();
                        let hero_media = hero_media.clone();
                        let hero_video = hero_video.clone();
                        move || {
                            play_next_active(
                                &state,
                                &player,
                                &overlay,
                                &now_title,
                                &now_subtitle,
                                &cover,
                                &hero_media,
                                &hero_video,
                                &hero_kicker,
                                &hero_title,
                                &hero_subtitle,
                                &hero_meta,
                                &hero_cover,
                                &mpris_for_next,
                            );
                        }
                    },
                    {
                        let hero_kicker = hero_kicker.clone();
                        let hero_title = hero_title.clone();
                        let hero_subtitle = hero_subtitle.clone();
                        let hero_meta = hero_meta.clone();
                        let hero_cover = hero_cover.clone();
                        let hero_media = hero_media.clone();
                        let hero_video = hero_video.clone();
                        move || {
                            play_previous_active(
                                &state,
                                &player,
                                &overlay,
                                &now_title,
                                &now_subtitle,
                                &cover,
                                &hero_media,
                                &hero_video,
                                &hero_kicker,
                                &hero_title,
                                &hero_subtitle,
                                &hero_meta,
                                &hero_cover,
                                &mpris_for_previous,
                            );
                        }
                    },
                );
            }
        }

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn persist_window_size(
    window: &adw::ApplicationWindow,
    state: &Rc<RefCell<UiState>>,
    settings_path: &PathBuf,
) {
    let width = window.width();
    let height = window.height();
    if width <= 0 || height <= 0 {
        return;
    }

    let mut state = state.borrow_mut();
    state.settings.window_width = Some(width);
    state.settings.window_height = Some(height);
    let _ = state.settings.save(settings_path);
}

fn start_scan(
    root: PathBuf,
    state: &Rc<RefCell<UiState>>,
    nav_list: &gtk::ListBox,
    folder_list: &gtk::ListBox,
    track_list: &gtk::ListBox,
    hero_media: &gtk::Stack,
    search: gtk::SearchEntry,
    hero_kicker: gtk::Label,
    hero_title: gtk::Label,
    hero_subtitle: gtk::Label,
    hero_meta: gtk::Label,
    hero_cover: gtk::Image,
    status_label: &gtk::Label,
    stat_number: &gtk::Label,
    overlay: &adw::ToastOverlay,
    settings_path: PathBuf,
    database_path: PathBuf,
) {
    status_label.set_label("Escaneando musica...");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let library = scanner::scan(root);
        let _ = tx.send(library);
    });

    let state = Rc::clone(state);
    let nav_list = nav_list.clone();
    let folder_list = folder_list.clone();
    let track_list = track_list.clone();
    let hero_media = hero_media.clone();
    let status_label = status_label.clone();
    let stat_number = stat_number.clone();
    let overlay = overlay.clone();
    glib::timeout_add_local(Duration::from_millis(120), move || match rx.try_recv() {
        Ok(library) => {
            let count = library.tracks.len();
            let root = library
                .root
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "biblioteca".to_string());
            if let Ok(db) = LibraryDatabase::open(&database_path) {
                let _ = db.replace_library(&library);
            }
            {
                let mut state = state.borrow_mut();
                state.visible_tracks = library.tracks.clone();
                state.library = library;
            }
            render_sidebar(&nav_list, &folder_list, &state.borrow().library);
            refresh_current_view(
                &state,
                &track_list,
                &search,
                &hero_media,
                &hero_kicker,
                &hero_title,
                &hero_subtitle,
                &hero_meta,
                &hero_cover,
                &status_label,
                &stat_number,
                &overlay,
                &settings_path,
            );
            stat_number.set_label(&count.to_string());
            status_label.set_label(&format!("{count} canciones encontradas en {root}"));
            show_toast(&overlay, format!("Escaneo completado: {count} canciones"));
            glib::ControlFlow::Break
        }
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
        Err(mpsc::TryRecvError::Disconnected) => {
            status_label.set_label("No se pudo completar el escaneo");
            show_toast(&overlay, "El escaneo se interrumpio");
            glib::ControlFlow::Break
        }
    });
}

fn render_sidebar(nav_list: &gtk::ListBox, folder_list: &gtk::ListBox, library: &Library) {
    clear_listbox(nav_list);
    clear_listbox(folder_list);

        for (label, icon) in [
        ("Música local", "music-note.svg"),
        ("Favoritos", "star-filled.svg"),
        ("Radio online", "radio.svg"),
    ] {
        nav_list.append(&sidebar_row(label, icon));
    }

    let folders = library.folders();
    if folders.is_empty() {
        folder_list.append(&empty_playlist_row());
        return;
    }

    for (index, folder) in folders.iter().enumerate() {
        let name = folder
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| folder.to_str().unwrap_or("Carpeta"));
        let count = library.tracks_in_folder(folder).len();
        folder_list.append(&playlist_row(name, count, index));
    }
}

fn render_tracks(
    list: &gtk::ListBox,
    tracks: &[Track],
    state: &Rc<RefCell<UiState>>,
    search: &gtk::SearchEntry,
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    status_label: &gtk::Label,
    stat_number: &gtk::Label,
    overlay: &adw::ToastOverlay,
    settings_path: &PathBuf,
) {
    clear_listbox(list);
    for (index, track) in tracks.iter().enumerate() {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_box.add_css_class("track-row-inner");
        row_box.set_hexpand(true);

        let number = gtk::Label::new(Some(&(index + 1).to_string()));
        number.add_css_class("track-number");
        number.set_width_chars(3);
        row_box.append(&number);

        let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        text_box.set_hexpand(true);
        let title = gtk::Label::new(Some(&track.title));
        title.set_xalign(0.0);
        title.add_css_class("track-title");
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let subtitle = gtk::Label::new(Some(&track.album.clone().unwrap_or_else(|| {
            track
                .folder
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Carpeta")
                .to_string()
        })));
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("track-subtitle");
        subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text_box.append(&title);
        text_box.append(&subtitle);
        row_box.append(&text_box);

        let artist = gtk::Label::new(Some(track.artist.as_deref().unwrap_or("Desconocido")));
        artist.set_xalign(0.0);
        artist.add_css_class("track-artist");
        artist.set_ellipsize(gtk::pango::EllipsizeMode::End);
        artist.set_width_chars(18);
        row_box.append(&artist);

        let folder = gtk::Label::new(
            track
                .folder
                .file_name()
                .and_then(|name| name.to_str())
                .or_else(|| track.folder.to_str())
                .or(Some("Carpeta")),
        );
        folder.add_css_class("folder-chip");
        folder.set_ellipsize(gtk::pango::EllipsizeMode::End);
        folder.set_max_width_chars(20);
        row_box.append(&folder);

        let size = gtk::Label::new(Some(&format_size(track.size)));
        size.add_css_class("track-size");
        size.set_xalign(1.0);
        row_box.append(&size);

        let row_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let favorite_button = favorite_button(is_track_favorite(&track.path, state));
        {
            let state = Rc::clone(state);
            let settings_path = settings_path.clone();
            let search = search.clone();
            let list = list.clone();
            let hero_media = hero_media.clone();
            let hero_kicker = hero_kicker.clone();
            let hero_title = hero_title.clone();
            let hero_subtitle = hero_subtitle.clone();
            let hero_meta = hero_meta.clone();
            let hero_cover = hero_cover.clone();
            let status_label = status_label.clone();
            let stat_number = stat_number.clone();
            let overlay = overlay.clone();
            let track_path = track.path.clone();
            favorite_button.connect_clicked(move |_| {
                toggle_favorite_track(&state, &settings_path, &track_path);
                refresh_current_view(
                    &state,
                    &list,
                    &search,
                    &hero_media,
                    &hero_kicker,
                    &hero_title,
                    &hero_subtitle,
                    &hero_meta,
                    &hero_cover,
                    &status_label,
                    &stat_number,
                    &overlay,
                    &settings_path,
                );
            });
        }
        row_actions.append(&favorite_button);
        row_box.append(&row_actions);

        let row = gtk::ListBoxRow::new();
        row.add_css_class("track-row");
        row.set_child(Some(&row_box));
        list.append(&row);
    }
}

fn sidebar_row(label: &str, icon: &str) -> gtk::ListBoxRow {
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row_box.add_css_class("sidebar-row-inner");
    row_box.set_hexpand(true);
    let image = image_from_asset(icon, 18);
    image.add_css_class("sidebar-row-icon");
    let label = gtk::Label::new(Some(label));
    label.set_xalign(0.0);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.add_css_class("sidebar-row-label");
    row_box.append(&image);
    row_box.append(&label);
    let row = gtk::ListBoxRow::new();
    row.add_css_class("sidebar-row");
    row.set_child(Some(&row_box));
    row
}

fn playlist_row(label: &str, count: usize, index: usize) -> gtk::ListBoxRow {
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row_box.add_css_class("playlist-row-inner");
    row_box.set_hexpand(true);

    let badge = gtk::Box::new(gtk::Orientation::Vertical, 0);
    badge.add_css_class("playlist-badge");
    badge.add_css_class(match index % 5 {
        0 => "playlist-badge-purple",
        1 => "playlist-badge-orange",
        2 => "playlist-badge-green",
        3 => "playlist-badge-blue",
        _ => "playlist-badge-gold",
    });
    let badge_icon = image_from_asset("folder.svg", 18);
    badge.append(&badge_icon);
    row_box.append(&badge);

    let text = gtk::Box::new(gtk::Orientation::Vertical, 2);
    text.set_hexpand(true);
    let label_widget = gtk::Label::new(Some(label));
    label_widget.set_xalign(0.0);
    label_widget.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label_widget.add_css_class("playlist-title");
    let count_widget = gtk::Label::new(Some(&format!("{count} canciones")));
    count_widget.set_xalign(0.0);
    count_widget.add_css_class("playlist-count");
    text.append(&label_widget);
    text.append(&count_widget);
    row_box.append(&text);

    let row = gtk::ListBoxRow::new();
    row.add_css_class("playlist-row");
    row.set_child(Some(&row_box));
    row
}

fn empty_playlist_row() -> gtk::ListBoxRow {
    let row_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    row_box.add_css_class("playlist-row-inner");
    row_box.set_hexpand(true);

    let title = gtk::Label::new(Some("Sin carpetas"));
    title.set_xalign(0.0);
    title.add_css_class("playlist-title");

    let subtitle = gtk::Label::new(Some("Pulsa + o la carpeta superior para añadir música"));
    subtitle.set_xalign(0.0);
    subtitle.set_wrap(true);
    subtitle.add_css_class("playlist-count");

    row_box.append(&title);
    row_box.append(&subtitle);

    let row = gtk::ListBoxRow::new();
    row.add_css_class("playlist-row");
    row.set_selectable(false);
    row.set_activatable(false);
    row.set_child(Some(&row_box));
    row
}

#[allow(clippy::too_many_arguments)]
fn open_music_folder_dialog(
    window: &adw::ApplicationWindow,
    state: Rc<RefCell<UiState>>,
    nav_list: gtk::ListBox,
    folder_list: gtk::ListBox,
    track_list: gtk::ListBox,
    hero_media: gtk::Stack,
    search: gtk::SearchEntry,
    hero_kicker: gtk::Label,
    hero_title: gtk::Label,
    hero_subtitle: gtk::Label,
    hero_meta: gtk::Label,
    hero_cover: gtk::Image,
    status_label: gtk::Label,
    stat_number: gtk::Label,
    overlay: adw::ToastOverlay,
    database_path: PathBuf,
    settings_path: PathBuf,
) {
    let dialog = gtk::FileDialog::builder()
        .title("Seleccionar carpeta de musica")
        .accept_label("Seleccionar")
        .modal(true)
        .build();

    dialog.select_folder(Some(window), None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result {
            if let Some(root) = file_path(&file) {
                state.borrow_mut().settings.last_music_dir = Some(root.clone());
                let _ = state.borrow().settings.save(&settings_path);
                start_scan(
                    root,
                    &state,
                    &nav_list,
                    &folder_list,
                    &track_list,
                    &hero_media,
                    search.clone(),
                    hero_kicker.clone(),
                    hero_title.clone(),
                    hero_subtitle.clone(),
                    hero_meta.clone(),
                    hero_cover.clone(),
                    &status_label,
                    &stat_number,
                    &overlay,
                    settings_path.clone(),
                    database_path.clone(),
                );
            } else {
                show_toast(
                    &overlay,
                    "No se pudo leer la ruta de la carpeta seleccionada",
                );
            }
        }
    });
}

fn command_button(label: &str, icon: &str) -> gtk::Button {
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let image = image_from_asset(icon, 18);
    let label = gtk::Label::new(Some(label));
    image.add_css_class("command-icon");
    label.add_css_class("command-label");
    content.append(&image);
    content.append(&label);
    let button = gtk::Button::new();
    button.add_css_class("command-button");
    button.set_child(Some(&content));
    button
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&image_from_asset(icon, 18)));
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class("icon-button");
    button
}

fn favorite_button(active: bool) -> gtk::Button {
    let button = gtk::Button::new();
    let icon = if active {
        "favorite-on.svg"
    } else {
        "favorite-off.svg"
    };
    button.set_child(Some(&image_from_asset(icon, 18)));
    button.set_tooltip_text(Some("Favorito"));
    button.add_css_class("icon-button");
    if active {
        button.add_css_class("favorite-button-active");
    } else {
        button.add_css_class("favorite-button-inactive");
    }
    button
}

fn image_from_asset(name: &str, pixel_size: i32) -> gtk::Image {
    if let Some(path) = icon_asset_path(name) {
        let image = gtk::Image::from_file(path);
        image.set_pixel_size(pixel_size);
        return image;
    }

    let image = gtk::Image::from_icon_name("image-missing-symbolic");
    image.set_pixel_size(pixel_size);
    image
}

fn app_cover_image() -> gtk::Image {
    if let Some(path) = app_cover_path() {
        gtk::Image::from_file(path)
    } else {
        gtk::Image::from_icon_name("org.kampos.kamusic")
    }
}

fn set_app_cover(image: &gtk::Image) {
    if let Some(path) = app_cover_path() {
        image.set_from_file(Some(path));
    } else {
        image.set_icon_name(Some("org.kampos.kamusic"));
    }
}

fn app_cover_path() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("data/org.kampos.kamusic.svg"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/org.kampos.kamusic.svg"),
        std::env::var_os("SNAP")
            .map(PathBuf::from)
            .map(|snap| snap.join("usr/share/icons/hicolor/scalable/apps/org.kampos.kamusic.svg"))
            .unwrap_or_default(),
    ];

    candidates.into_iter().find(|path| path.is_file())
}

fn preferred_music_dir(settings: &Settings) -> Option<PathBuf> {
    settings
        .last_music_dir
        .as_ref()
        .filter(|path| path.is_dir())
        .cloned()
        .or_else(scanner::default_music_dir)
}

fn app_logo_image() -> gtk::Image {
    if let Some(path) = app_cover_path() {
        let image = gtk::Image::from_file(path);
        image.set_pixel_size(26);
        return image;
    }

    gtk::Image::from_icon_name("org.kampos.kamusic")
}

fn icon_asset_path(name: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("data/icons").join(name),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/icons")
            .join(name),
        std::env::var_os("SNAP")
            .map(PathBuf::from)
            .map(|snap| snap.join("usr/share/kamusic/icons").join(name))
            .unwrap_or_default(),
    ];

    candidates.into_iter().find(|path| path.is_file())
}

fn file_path(file: &gio::File) -> Option<PathBuf> {
    if let Some(path) = file.path() {
        return Some(path);
    }

    let uri = file.uri();
    url::Url::parse(&uri)
        .ok()
        .and_then(|url| url.to_file_path().ok())
}

fn install_css() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(APP_CSS);
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn clear_listbox(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn set_active_section(
    section: ActiveSection,
    state: &Rc<RefCell<UiState>>,
    track_list: &gtk::ListBox,
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    search: &gtk::SearchEntry,
    status_label: &gtk::Label,
    stat_number: &gtk::Label,
    overlay: &adw::ToastOverlay,
    settings_path: &PathBuf,
) {
    {
        let mut state = state.borrow_mut();
        state.active_section = section;
        state.current_index = None;
        state.is_playing = false;
    }

    match section {
        ActiveSection::Local => {
            let tracks = state.borrow().visible_tracks.clone();
            state.borrow_mut().visible_tracks = tracks.clone();
            search.set_placeholder_text(Some("Buscar canciones, artistas, albumes..."));
            refresh_current_view(
                state,
                track_list,
                search,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
                status_label,
                stat_number,
                overlay,
                settings_path,
            );
        }
        ActiveSection::Favorites => {
            rebuild_visible_favorites(state);
            search.set_placeholder_text(Some("Buscar favoritos"));
            refresh_current_view(
                state,
                track_list,
                search,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
                status_label,
                stat_number,
                overlay,
                settings_path,
            );
        }
        ActiveSection::Radio => {
            let items = state.borrow().visible_radio.clone();
            if items.is_empty() {
                render_online_items(track_list, &[]);
                stat_number.set_label("0");
                status_label.set_label("Busca emisoras y pulsa Enter");
            } else {
                render_online_items(track_list, &items);
                stat_number.set_label(&items.len().to_string());
                status_label.set_label("Emisoras de España");
            }
            search.set_placeholder_text(Some("Buscar emisoras online"));
            let featured = state
                .borrow()
                .current_index
                .and_then(|index| state.borrow().visible_radio.get(index).cloned())
                .or_else(|| state.borrow().visible_radio.first().cloned());
            refresh_online_featured(
                featured.as_ref(),
                ActiveSection::Radio,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
            );
        }
    }
}

fn render_online_items(list: &gtk::ListBox, items: &[OnlineItem]) {
    clear_listbox(list);
    for (index, item) in items.iter().enumerate() {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_box.add_css_class("track-row-inner");
        row_box.set_hexpand(true);

        let number = gtk::Label::new(Some(&(index + 1).to_string()));
        number.add_css_class("track-number");
        number.set_width_chars(3);
        row_box.append(&number);

        let image = if let Some(path) = &item.cover_path {
            gtk::Image::from_file(path)
        } else {
            app_cover_image()
        };
        image.set_pixel_size(48);
        image.add_css_class("track-cover");
        row_box.append(&image);
        if item.cover_path.is_none() {
            fetch_radio_cover_async(item.clone(), image.clone());
        }

        let text_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        text_box.set_hexpand(true);
        let title = gtk::Label::new(Some(&item.title));
        title.set_xalign(0.0);
        title.add_css_class("track-title");
        title.set_ellipsize(gtk::pango::EllipsizeMode::End);
        let subtitle = gtk::Label::new(Some(&item.subtitle));
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("track-subtitle");
        subtitle.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text_box.append(&title);
        text_box.append(&subtitle);
        row_box.append(&text_box);

        let source = gtk::Label::new(Some(match item.kind {
            OnlineKind::Radio { .. } => "Radio",
        }));
        source.add_css_class("folder-chip");
        row_box.append(&source);

        let row = gtk::ListBoxRow::new();
        row.add_css_class("track-row");
        row.set_child(Some(&row_box));
        unsafe {
            row.set_data(ONLINE_ITEM_KEY, item.clone());
        }
        list.append(&row);
    }
}

fn online_item_from_row(row: &gtk::ListBoxRow) -> Option<OnlineItem> {
    unsafe { row.data::<OnlineItem>(ONLINE_ITEM_KEY) }.map(|ptr| unsafe { ptr.as_ref().clone() })
}

fn is_track_favorite(path: &PathBuf, state: &Rc<RefCell<UiState>>) -> bool {
    state
        .borrow()
        .settings
        .favorite_tracks
        .iter()
        .any(|favorite| favorite == path)
}

fn toggle_favorite_track(
    state: &Rc<RefCell<UiState>>,
    settings_path: &PathBuf,
    path: &PathBuf,
) {
    {
        let mut state = state.borrow_mut();
        let favorites = &mut state.settings.favorite_tracks;
        if let Some(index) = favorites.iter().position(|favorite| favorite == path) {
            favorites.remove(index);
        } else {
            favorites.push(path.clone());
        }
        let _ = state.settings.save(settings_path);
    }
    rebuild_visible_favorites(state);
}

fn rebuild_visible_favorites(state: &Rc<RefCell<UiState>>) {
    let mut state_mut = state.borrow_mut();
    let query = state_mut.current_query.clone();
    let favorites = state_mut.settings.favorite_tracks.clone();
    state_mut.visible_favorites = filter_favorites(&state_mut.library, &favorites, &query);
}

fn filter_favorites(library: &Library, favorites: &[PathBuf], query: &str) -> Vec<Track> {
    let needle = query.trim().to_lowercase();
    library
        .tracks
        .iter()
        .filter(|track| favorites.iter().any(|favorite| favorite == &track.path))
        .filter(|track| {
            if needle.is_empty() {
                return true;
            }
            track.title.to_lowercase().contains(&needle)
                || track
                    .artist
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
                || track
                    .album
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
                || track
                    .path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&needle)
                || track
                    .folder
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&needle)
        })
        .cloned()
        .collect()
}

fn refresh_current_view(
    state: &Rc<RefCell<UiState>>,
    track_list: &gtk::ListBox,
    search: &gtk::SearchEntry,
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    status_label: &gtk::Label,
    stat_number: &gtk::Label,
    overlay: &adw::ToastOverlay,
    settings_path: &PathBuf,
) {
    let section = state.borrow().active_section;
    match section {
        ActiveSection::Local => {
            let tracks = state.borrow().visible_tracks.clone();
            render_tracks(
                track_list,
                &tracks,
                state,
                search,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
                status_label,
                stat_number,
                overlay,
                settings_path,
            );
            stat_number.set_label(&tracks.len().to_string());
            status_label.set_label("Biblioteca local");
            refresh_featured_panel(
                &tracks,
                &state.borrow().library,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
                ActiveSection::Local,
            );
        }
        ActiveSection::Favorites => {
            let tracks = state.borrow().visible_favorites.clone();
            {
                let mut state_mut = state.borrow_mut();
                state_mut.visible_tracks = tracks.clone();
            }
            render_tracks(
                track_list,
                &tracks,
                state,
                search,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
                status_label,
                stat_number,
                overlay,
                settings_path,
            );
            stat_number.set_label(&tracks.len().to_string());
            status_label.set_label("Favoritos");
            refresh_favorites_panel(
                &tracks,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
            );
        }
        ActiveSection::Radio => {
            let items = state.borrow().visible_radio.clone();
            render_online_items(track_list, &items);
            stat_number.set_label(&items.len().to_string());
            status_label.set_label("Emisoras de España");
            let featured = items.first().cloned();
            refresh_online_featured(
                featured.as_ref(),
                ActiveSection::Radio,
                hero_media,
                hero_kicker,
                hero_title,
                hero_subtitle,
                hero_meta,
                hero_cover,
            );
        }
    }
}

fn refresh_favorites_panel(
    tracks: &[Track],
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
) {
    hero_media.set_visible_child_name("cover");
    hero_kicker.set_label("FAVORITOS");
    if let Some(track) = tracks.first() {
        hero_title.set_label(&track.title);
        hero_subtitle.set_label(track.display_artist_album().as_str());
        hero_meta.set_label(&format!("{} canciones guardadas", tracks.len()));
        if let Some(path) = &track.cover_path {
            hero_cover.set_from_file(Some(path));
        } else {
            set_app_cover(hero_cover);
        }
    } else {
        hero_title.set_label("Favoritos");
        hero_subtitle.set_label("Marca canciones con el corazon");
        hero_meta.set_label("0 canciones guardadas");
        set_app_cover(hero_cover);
    }
}

fn online_item_at(state: &Rc<RefCell<UiState>>, index: usize) -> Option<OnlineItem> {
    let state_ref = state.borrow();
    match state_ref.active_section {
        ActiveSection::Radio => state_ref.visible_radio.get(index).cloned(),
        ActiveSection::Local | ActiveSection::Favorites => None,
    }
}

fn refresh_featured_panel(
    tracks: &[Track],
    library: &Library,
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    section: ActiveSection,
) {
    match section {
        ActiveSection::Local | ActiveSection::Favorites => {
            hero_media.set_visible_child_name("cover");
            hero_kicker.set_label("MUSICA LOCAL");
            let featured = tracks.first().or_else(|| library.tracks.first());
            if let Some(track) = featured {
                hero_title.set_label(
                    track
                        .album
                        .as_deref()
                        .unwrap_or_else(|| track.title.as_str()),
                );
                hero_subtitle.set_label(track.artist.as_deref().unwrap_or("Coleccion local"));
                let root = library
                    .root
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Sin ruta".to_string());
                hero_meta.set_label(&format!(
                    "{} canciones · {} carpetas · {}",
                    tracks.len(),
                    library.folders().len(),
                    root
                ));
                if let Some(path) = &track.cover_path {
                    hero_cover.set_from_file(Some(path));
                } else {
                    set_app_cover(hero_cover);
                    fetch_track_cover_async(track.clone(), hero_cover.clone());
                }
            } else {
                hero_title.set_label("Tu musica local");
                hero_subtitle.set_label("Escanea una carpeta para ver tu coleccion");
                hero_meta.set_label("0 canciones · 0 carpetas");
                set_app_cover(hero_cover);
            }
        }
        ActiveSection::Radio => {
            hero_media.set_visible_child_name("cover");
            hero_kicker.set_label("RADIO ONLINE");
            hero_title.set_label("Busca emisoras");
            hero_subtitle.set_label("Pulsa Enter para cargar emisoras");
            hero_meta.set_label("Streaming en vivo");
            set_app_cover(hero_cover);
        }
    }
}

fn refresh_online_featured(
    item: Option<&OnlineItem>,
    section: ActiveSection,
    hero_media: &gtk::Stack,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
) {
    match section {
        ActiveSection::Radio => {
            hero_media.set_visible_child_name("cover");
            hero_kicker.set_label("RADIO ONLINE");
            if let Some(item) = item {
                hero_title.set_label(&item.title);
                hero_subtitle.set_label(&item.subtitle);
                hero_meta.set_label("Emision en directo");
                if let Some(path) = &item.cover_path {
                    hero_cover.set_from_file(Some(path));
                } else {
                    set_app_cover(hero_cover);
                    fetch_radio_cover_async(item.clone(), hero_cover.clone());
                }
            } else {
                hero_title.set_label("Selecciona una emisora");
                hero_subtitle.set_label("El listado de radios de España aparecera aqui");
                hero_meta.set_label("Emision en directo");
                set_app_cover(hero_cover);
            }
        }
        ActiveSection::Local | ActiveSection::Favorites => {}
    }
}

fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size = size as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else if size >= KB {
        format!("{:.0} KB", size / KB)
    } else {
        format!("{} B", size as u64)
    }
}

fn play_visible_index(
    index: usize,
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    mpris: &MprisControls,
) {
    {
        let mut state = state.borrow_mut();
        state.queue = state.visible_tracks.clone();
    }
    play_queue_index(
        index,
        state,
        player,
        overlay,
        now_title,
        now_subtitle,
        cover,
        mpris,
    );

    if let Some(track) = state.borrow().queue.get(index).cloned() {
        fetch_track_cover_async(track, cover.clone());
    }
}

fn play_online_item(
    index: usize,
    item: OnlineItem,
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    hero_media: &gtk::Stack,
    hero_video: &gtk::Picture,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let Some(player) = player.as_ref() else {
        show_toast(overlay, "GStreamer no esta disponible");
        return;
    };

    let active_section = state.borrow().active_section;

    refresh_online_featured(
        Some(&item),
        active_section,
        hero_media,
        hero_kicker,
        hero_title,
        hero_subtitle,
        hero_meta,
        hero_cover,
    );

    match &item.kind {
        OnlineKind::Radio { stream_url } => match player.play_uri(stream_url) {
            Ok(()) => {
                {
                    let mut state = state.borrow_mut();
                    state.current_index = Some(index);
                    state.is_playing = true;
                    state.online_queue = state.visible_radio.clone();
                }
                now_title.set_label(&item.title);
                now_subtitle.set_label(&item.subtitle);
                if let Some(path) = &item.cover_path {
                    cover.set_from_file(Some(path));
                } else {
                    set_app_cover(cover);
                    fetch_radio_cover_async(item.clone(), cover.clone());
                }
                hero_video.set_paintable(None::<&gtk::gdk::Paintable>);
                hero_media.set_visible_child_name("cover");
                mpris.set_playing(&Track {
                    title: item.title.clone(),
                    artist: Some(item.subtitle.clone()),
                    album: None,
                    path: PathBuf::new(),
                    folder: PathBuf::new(),
                    extension: String::new(),
                    cover_path: item.cover_path.clone(),
                    size: 0,
                    modified: 0,
                });
            }
            Err(err) => show_toast(overlay, format!("No se pudo reproducir este stream: {err}")),
        },
    }
}

fn fetch_track_cover_async(track: Track, image: gtk::Image) {
    if track.cover_path.is_some() {
        return;
    }

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let cover = crate::library::cover::download_itunes_cover(&track);
        let _ = tx.send(cover);
    });

    glib::timeout_add_local(Duration::from_millis(150), move || match rx.try_recv() {
        Ok(Some(path)) => {
            image.set_from_file(Some(path));
            glib::ControlFlow::Break
        }
        Ok(None) | Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
    });
}

fn fetch_radio_cover_async(item: OnlineItem, image: gtk::Image) {
    let Some(favicon_url) = item.favicon_url.clone() else {
        return;
    };
    if item.cover_path.is_some() {
        return;
    }

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = online::http_client().ok().and_then(|client| {
            online::download_thumbnail(&client, &favicon_url, "radio")
        });
        let _ = tx.send(result);
    });

    glib::timeout_add_local(Duration::from_millis(150), move || match rx.try_recv() {
        Ok(Some(path)) => {
            image.set_from_file(Some(path));
            glib::ControlFlow::Break
        }
        Ok(None) | Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
    });
}

fn start_online_search(
    section: ActiveSection,
    query: String,
    state: &Rc<RefCell<UiState>>,
    track_list: &gtk::ListBox,
    hero_media: &gtk::Stack,
    hero_kicker: gtk::Label,
    hero_title: gtk::Label,
    hero_subtitle: gtk::Label,
    hero_meta: gtk::Label,
    hero_cover: gtk::Image,
    status_label: &gtk::Label,
    stat_number: &gtk::Label,
    overlay: &adw::ToastOverlay,
) {
    let token = {
        let mut state = state.borrow_mut();
        state.search_token = state.search_token.wrapping_add(1);
        state.search_token
    };

    status_label.set_label("Buscando...");
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = match section {
            ActiveSection::Radio => online::search_radio(&query),
            ActiveSection::Local | ActiveSection::Favorites => Ok(Vec::new()),
        };
        let _ = tx.send((token, result));
    });

    let state = Rc::clone(state);
    let track_list = track_list.clone();
    let hero_media = hero_media.clone();
    let hero_kicker = hero_kicker.clone();
    let hero_title = hero_title.clone();
    let hero_subtitle = hero_subtitle.clone();
    let hero_meta = hero_meta.clone();
    let hero_cover = hero_cover.clone();
    let status_label = status_label.clone();
    let stat_number = stat_number.clone();
    let overlay = overlay.clone();

    glib::timeout_add_local(Duration::from_millis(120), move || {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match rx.try_recv() {
                Ok((returned_token, result)) => {
                    if returned_token != state.borrow().search_token {
                        return glib::ControlFlow::Break;
                    }

                    match result {
                        Ok(items) => {
                            {
                                let mut state = state.borrow_mut();
                                state.visible_radio = items.clone();
                                state.online_queue = items.clone();
                                state.current_index = None;
                                state.is_playing = false;
                            }
                            render_online_items(&track_list, &items);
                            stat_number.set_label(&items.len().to_string());
                            if items.is_empty() {
                                status_label.set_label("Busca emisoras y pulsa Enter");
                            } else {
                                status_label.set_label("Emisoras de España");
                            }
                            let featured = items.first().cloned();
                            refresh_online_featured(
                                featured.as_ref(),
                                ActiveSection::Radio,
                                &hero_media,
                                &hero_kicker,
                                &hero_title,
                                &hero_subtitle,
                                &hero_meta,
                                &hero_cover,
                            );
                        }
                        Err(err) => {
                            status_label.set_label(&format!("Error de red: {err}"));
                            show_toast(&overlay, "No se pudo obtener resultados");
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            }));

        match result {
            Ok(flow) => flow,
            Err(_) => {
                show_toast(&overlay, "La busqueda fallo");
                glib::ControlFlow::Break
            }
        }
    });
}

fn play_random_visible_index(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let len = state.borrow().visible_tracks.len();
    if len == 0 {
        show_toast(overlay, "Escanea una carpeta o selecciona una cancion");
        return;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as usize)
        .unwrap_or(0);
    let index = seed % len;
    play_visible_index(
        index,
        state,
        player,
        overlay,
        now_title,
        now_subtitle,
        cover,
        mpris,
    );
}

fn play_queue_index(
    index: usize,
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let Some(player) = player.as_ref() else {
        show_toast(overlay, "GStreamer no esta disponible");
        return;
    };
    let track = state.borrow().queue.get(index).cloned();
    if let Some(track) = track {
        match player.play_file(&track.path) {
            Ok(()) => {
                {
                    let mut state = state.borrow_mut();
                    state.current_index = Some(index);
                    state.is_playing = true;
                }
                now_title.set_label(&track.title);
                now_subtitle.set_label(&track.display_artist_album());
                if let Some(path) = &track.cover_path {
                    cover.set_from_file(Some(path));
                } else {
                    set_app_cover(cover);
                    fetch_track_cover_async(track.clone(), cover.clone());
                }
                mpris.set_playing(&track);
            }
            Err(err) => show_toast(
                overlay,
                format!("No se pudo reproducir este archivo. {err}"),
            ),
        }
    }
}

fn play_next_active(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    hero_media: &gtk::Stack,
    hero_video: &gtk::Picture,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let active_section = state.borrow().active_section;
    match active_section {
        ActiveSection::Local | ActiveSection::Favorites => {
            let next = state
                .borrow()
                .current_index
                .map(|idx| idx.saturating_add(1))
                .unwrap_or(0);
            play_queue_index(
                next,
                state,
                player,
                overlay,
                now_title,
                now_subtitle,
                cover,
                mpris,
            );
        }
        ActiveSection::Radio => {
            let next = state
                .borrow()
                .current_index
                .map(|idx| idx.saturating_add(1))
                .unwrap_or(0);
            if let Some(item) = online_item_at(state, next) {
                play_online_item(
                    next,
                    item,
                    state,
                    player,
                    overlay,
                    now_title,
                    now_subtitle,
                    cover,
                    hero_media,
                    hero_video,
                    hero_kicker,
                    hero_title,
                    hero_subtitle,
                    hero_meta,
                    hero_cover,
                    mpris,
                );
            } else {
                show_toast(overlay, "No hay elementos para reproducir");
            }
        }
    }
}

fn play_previous_active(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    hero_media: &gtk::Stack,
    hero_video: &gtk::Picture,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let active_section = state.borrow().active_section;
    match active_section {
        ActiveSection::Local | ActiveSection::Favorites => {
            let previous = state
                .borrow()
                .current_index
                .and_then(|idx| idx.checked_sub(1))
                .unwrap_or(0);
            play_queue_index(
                previous,
                state,
                player,
                overlay,
                now_title,
                now_subtitle,
                cover,
                mpris,
            );
        }
        ActiveSection::Radio => {
            let previous = state
                .borrow()
                .current_index
                .and_then(|idx| idx.checked_sub(1))
                .unwrap_or(0);
            if let Some(item) = online_item_at(state, previous) {
                play_online_item(
                    previous,
                    item,
                    state,
                    player,
                    overlay,
                    now_title,
                    now_subtitle,
                    cover,
                    hero_media,
                    hero_video,
                    hero_kicker,
                    hero_title,
                    hero_subtitle,
                    hero_meta,
                    hero_cover,
                    mpris,
                );
            } else {
                show_toast(overlay, "No hay elementos para reproducir");
            }
        }
    }
}

fn request_play(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    now_title: &gtk::Label,
    now_subtitle: &gtk::Label,
    cover: &gtk::Image,
    hero_media: &gtk::Stack,
    hero_video: &gtk::Picture,
    hero_kicker: &gtk::Label,
    hero_title: &gtk::Label,
    hero_subtitle: &gtk::Label,
    hero_meta: &gtk::Label,
    hero_cover: &gtk::Image,
    mpris: &MprisControls,
) {
    let player_handle = Rc::clone(player);
    if let Some(active_player) = player_handle.as_ref() {
        let active_section = state.borrow().active_section;
        match active_section {
            ActiveSection::Local | ActiveSection::Favorites => {
                let (current_index, queue, visible_tracks_empty) = {
                    let state_ref = state.borrow();
                    (
                        state_ref.current_index,
                        state_ref.queue.clone(),
                        state_ref.visible_tracks.is_empty(),
                    )
                };

                if let Some(index) = current_index {
                    if let Err(err) = active_player.play() {
                        show_toast(overlay, format!("No se pudo continuar: {err}"));
                    } else if let Some(track) = queue.get(index) {
                        state.borrow_mut().is_playing = true;
                        mpris.set_playing(track);
                    }
                } else if visible_tracks_empty {
                    show_toast(overlay, "Escanea una carpeta o selecciona una cancion");
                } else {
                    play_visible_index(
                        0,
                        state,
                        &player_handle,
                        overlay,
                        now_title,
                        now_subtitle,
                        cover,
                        mpris,
                    );
                }
            }
            ActiveSection::Radio => {
                let (current_index, online_queue) = {
                    let state_ref = state.borrow();
                    (state_ref.current_index, state_ref.online_queue.clone())
                };

                if let Some(index) = current_index {
                    if let Err(err) = active_player.play() {
                        show_toast(overlay, format!("No se pudo continuar: {err}"));
                    } else if let Some(item) = online_queue.get(index) {
                        state.borrow_mut().is_playing = true;
                        now_title.set_label(&item.title);
                        now_subtitle.set_label(&item.subtitle);
                    }
                } else {
                    if let Some(item) = online_item_at(state, 0) {
                        play_online_item(
                            0,
                            item,
                            state,
                            &player_handle,
                            overlay,
                            now_title,
                            now_subtitle,
                            cover,
                            hero_media,
                            hero_video,
                            hero_kicker,
                            hero_title,
                            hero_subtitle,
                            hero_meta,
                            hero_cover,
                            mpris,
                        );
                    } else {
                        show_toast(overlay, "No hay elementos para reproducir");
                    }
                }
            }
        }
    } else {
        show_toast(overlay, "GStreamer no esta disponible");
    }
}

fn request_pause(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    mpris: &MprisControls,
) {
    if let Some(player) = player.as_ref() {
        if let Err(err) = player.pause() {
            show_toast(overlay, format!("No se pudo pausar: {err}"));
        } else {
            state.borrow_mut().is_playing = false;
            mpris.set_paused();
        }
    } else {
        show_toast(overlay, "GStreamer no esta disponible");
    }
}

fn request_stop(
    state: &Rc<RefCell<UiState>>,
    player: &Rc<Option<Player>>,
    overlay: &adw::ToastOverlay,
    mpris: &MprisControls,
) {
    if let Some(player) = player.as_ref() {
        if let Err(err) = player.stop() {
            show_toast(overlay, format!("No se pudo detener: {err}"));
        } else {
            {
                let mut state = state.borrow_mut();
                state.is_playing = false;
                state.current_index = None;
            }
            mpris.set_stopped();
        }
    } else {
        show_toast(overlay, "GStreamer no esta disponible");
    }
}

const APP_CSS: &str = r#"
.app-shell {
  background:
    radial-gradient(circle at top left, rgba(143,99,255,0.14), transparent 30%),
    radial-gradient(circle at bottom right, rgba(76,166,255,0.08), transparent 28%),
    #0b0d12;
  color: #f5f7fb;
  padding: 12px;
}

.workspace {
  min-height: 0;
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 20px;
  background: rgba(13, 15, 20, 0.96);
  box-shadow: 0 24px 60px rgba(0,0,0,0.36);
}

.sidebar-panel {
  background: linear-gradient(180deg, rgba(255,255,255,0.045), rgba(255,255,255,0.02));
  border-right: 1px solid rgba(255,255,255,0.08);
  padding: 16px 14px 14px 14px;
}

.brand-shell {
  padding: 6px 4px 10px 4px;
}

.brand-icon {
  color: #a88cff;
  background: rgba(168, 140, 255, 0.16);
  border-radius: 10px;
  padding: 6px;
}

.brand-title {
  font-size: 16px;
  font-weight: 800;
  color: #ffffff;
}

.brand-subtitle {
  color: rgba(245,247,251,0.62);
  font-size: 11px;
}

.sidebar-scroll {
  background: transparent;
}

.sidebar-stack {
  padding-top: 4px;
}

.section-kicker {
  color: rgba(245,247,251,0.54);
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0;
  text-transform: uppercase;
}

.sidebar-list,
.playlist-list,
.track-list {
  background: transparent;
}

.sidebar-row,
.playlist-row,
.track-row {
  border-radius: 16px;
  margin: 5px 0;
}

.sidebar-row:selected,
.playlist-row:selected,
.track-row:selected {
  background: linear-gradient(135deg, rgba(143,99,255,0.26), rgba(143,99,255,0.12));
  border: 1px solid rgba(168,140,255,0.24);
}

.sidebar-row-inner,
.playlist-row-inner,
.track-row-inner {
  padding: 11px 12px;
  color: rgba(245,247,251,0.90);
}

.sidebar-row-icon {
  opacity: 0.88;
  color: rgba(245,247,251,0.88);
}

.sidebar-row-label {
  font-size: 14px;
  font-weight: 650;
}

.playlist-badge {
  min-width: 38px;
  min-height: 38px;
  border-radius: 10px;
  color: #fff;
  background: rgba(255,255,255,0.08);
}

.playlist-badge-purple { background: linear-gradient(135deg, #9b6dff, #7051d5); }
.playlist-badge-orange { background: linear-gradient(135deg, #ffb24d, #ff7a18); }
.playlist-badge-green { background: linear-gradient(135deg, #48d395, #1d9b67); }
.playlist-badge-blue { background: linear-gradient(135deg, #4ca6ff, #2469db); }
.playlist-badge-gold { background: linear-gradient(135deg, #ffd56b, #f0a91f); }

.playlist-title {
  font-size: 13px;
  font-weight: 700;
}

.playlist-count {
  color: rgba(245,247,251,0.60);
  font-size: 11px;
}

.sidebar-footer {
  padding: 10px 6px 2px 6px;
  border-top: 1px solid rgba(255,255,255,0.08);
  color: rgba(245,247,251,0.86);
}

.sidebar-footer-label {
  font-size: 13px;
}

.content-panel {
  padding: 14px 16px 16px 16px;
  min-width: 0;
}

.topbar {
  min-height: 46px;
}

.global-search {
  min-height: 42px;
  border-radius: 14px;
  background: rgba(255,255,255,0.05);
}

.icon-button {
  min-width: 38px;
  min-height: 38px;
  border-radius: 999px;
  background: rgba(255,255,255,0.055);
  color: rgba(245,247,251,0.92);
}

.favorite-button-inactive {
  color: rgba(245, 247, 251, 0.42);
}

.favorite-button-active {
  color: #ffffff;
}

.command-button {
  min-height: 42px;
  border-radius: 14px;
  background: rgba(255,255,255,0.055);
  color: rgba(245,247,251,0.92);
}

.command-button.suggested-action {
  background: linear-gradient(135deg, #8f63ff, #6a4df0);
  color: #ffffff;
}

.secondary-action {
  background: rgba(255,255,255,0.05);
  color: rgba(245,247,251,0.92);
}

.command-icon {
  opacity: 0.95;
  color: rgba(245,247,251,0.95);
}

.command-label {
  font-weight: 700;
}

.hero-panel {
  min-height: 248px;
  padding: 14px;
  border-radius: 18px;
  border: 1px solid rgba(255,255,255,0.06);
  background: #000000;
  box-shadow: 0 22px 52px rgba(0,0,0,0.30);
}

.hero-cover {
  border-radius: 16px;
  background: #000000;
  border: 1px solid rgba(255,255,255,0.05);
}

.hero-video {
  border-radius: 16px;
  background: #000000;
  border: 1px solid rgba(255,255,255,0.05);
}

.hero-kicker {
  color: #a88cff;
  font-size: 10px;
  font-weight: 800;
  letter-spacing: 0;
  text-transform: uppercase;
}

.hero-title {
  font-size: 30px;
  font-weight: 900;
  color: #ffffff;
}

.hero-subtitle {
  color: rgba(245,247,251,0.74);
  font-size: 15px;
}

.hero-meta {
  color: rgba(245,247,251,0.58);
  font-size: 12px;
}

.hero-copy {
  padding-left: 4px;
}

.hero-actions {
  padding-top: 2px;
}

.hero-status {
  color: rgba(245,247,251,0.62);
  font-size: 12px;
}

.hero-stats {
  padding-top: 2px;
}

.hero-stat-number {
  font-size: 30px;
  font-weight: 900;
  color: #ffffff;
}

.hero-stat-text {
  color: rgba(245,247,251,0.62);
  font-size: 13px;
  padding-top: 10px;
}

.track-header {
  padding: 0 10px 0 10px;
  color: rgba(245,247,251,0.54);
}

.track-header-cell {
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0;
  text-transform: uppercase;
}

.track-header-number {
  min-width: 36px;
}

.track-row {
  background: rgba(255,255,255,0.028);
  border: 1px solid rgba(255,255,255,0.05);
}

.track-row:hover {
  background: rgba(255,255,255,0.07);
}

.track-number {
  color: rgba(245,247,251,0.72);
  font-weight: 700;
  min-width: 28px;
}

.track-title {
  font-size: 16px;
  font-weight: 800;
  color: #ffffff;
}

.track-subtitle,
.track-artist,
.track-size {
  color: rgba(245,247,251,0.68);
  font-size: 13px;
}

.track-subtitle {
  color: rgba(168,140,255,0.92);
}

.folder-chip {
  padding: 5px 10px;
  border-radius: 999px;
  background: rgba(255,255,255,0.08);
  color: rgba(245,247,251,0.72);
  font-size: 12px;
}

.track-list {
  background: transparent;
}

.track-row-inner {
  margin: 0;
  min-height: 50px;
  padding-top: 6px;
  padding-bottom: 6px;
}

.now-cover {
  border-radius: 14px;
  background: #000000;
  border: 1px solid rgba(255,255,255,0.05);
}

.player-bar {
  padding: 16px 18px;
  background: rgba(8, 10, 14, 0.94);
  border-top: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 -16px 38px rgba(0,0,0,0.26);
}

.player-controls {
  padding: 0 12px;
}

.player-control {
  min-width: 40px;
  min-height: 40px;
  border-radius: 999px;
  color: rgba(245,247,251,0.94);
}

.main-play {
  background: linear-gradient(135deg, #8f63ff, #6a4df0);
  color: #ffffff;
}

.dim-label {
  color: rgba(245,247,251,0.58);
}
"#;
