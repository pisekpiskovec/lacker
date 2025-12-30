use gtk4::prelude::*;
use gtk4::glib;
use gtk4::{Application, ApplicationWindow, Box, Button, Image, Label, Orientation, ScrolledWindow, SearchEntry, Separator};
use std::collections::HashMap;
use std::fs;

#[cfg(feature = "wayland")]
use gtk4_layer_shell::{Edge, Layer, LayerShell};

#[derive(Clone, Debug)]
struct DesktopApp {
    name: String,
    exec: String,
    icon: Option<String>,
    categories: Vec<String>,
}

fn scan_applications() -> Vec<DesktopApp> {
    let mut apps = Vec::new();
    let home_dir = format!("{}/.local/share/applications", std::env::var("HOME").unwrap_or_default());
    let dirs = vec![
        "/usr/share/applications",
        "/usr/local/share/applications",
        &home_dir,
    ];

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(de) = freedesktop_desktop_entry::DesktopEntry::decode(&path, &content) {
                            if let Some(exec) = de.exec() {
                                if de.no_display() {
                                    continue;
                                }

                                let name = de.name(None).unwrap_or(std::borrow::Cow::Borrowed("Unknown")).to_string();
                                let icon = de.icon().map(|s: &str| s.to_string());
                                let categories: Vec<String> = de.categories()
                                    .unwrap_or_default()
                                    .split(';')
                                    .filter(|s: &&str| !s.is_empty())
                                    .map(|s: &str| s.to_string())
                                    .collect();

                                apps.push(DesktopApp {
                                    name,
                                    exec: exec.to_string(),
                                    icon,
                                    categories,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

fn categorize_apps(apps: &[DesktopApp]) -> HashMap<String, Vec<DesktopApp>> {
    let mut categories: HashMap<String, Vec<DesktopApp>> = HashMap::new();

    for app in apps {
        if app.categories.is_empty() {
            categories.entry("Other".to_string())
                .or_insert_with(Vec::new)
                .push(app.clone());
        } else {
            for cat in &app.categories {
                categories.entry(cat.clone())
                    .or_insert_with(Vec::new)
                    .push(app.clone());
            }
        }
    }

    categories
}

fn launch_app(exec: &str) {
    let exec = exec.split_whitespace()
        .filter(|s| !s.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ");

    std::thread::spawn(move || {
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(&exec)
            .spawn();
    });
}

fn create_app_button(app: &DesktopApp) -> Button {
    let button_box = Box::new(Orientation::Horizontal, 8);
    button_box.set_margin_start(4);
    button_box.set_margin_end(4);
    button_box.set_margin_top(2);
    button_box.set_margin_bottom(2);

    if let Some(icon_name) = &app.icon {
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(24);
        button_box.append(&icon);
    }

    let label = Label::new(Some(&app.name));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    button_box.append(&label);

    let button = Button::new();
    button.set_child(Some(&button_box));
    button.set_has_frame(false);

    let exec = app.exec.clone();
    button.connect_clicked(move |_| {
        launch_app(&exec);
    });
    
    button
}

fn rebuild_app_list(apps_box: &Box, apps: &[DesktopApp], categories: &HashMap<String, Vec<DesktopApp>>, query: &str) {
    while let Some(child) = apps_box.first_child() {
        apps_box.remove(&child);
    }

    if query.is_empty() {
        let priority_cats = vec![
            ("Utilities", "Utility"),
            ("Development", "Development"),
            ("Graphics", "Graphics"),
            ("Internet", "Network"),
            ("Office", "Office"),
            ("Multimedia", "AudioVideo"),
            ("System", "System"),
        ];

        for (display_name, cat_name) in priority_cats {
            if let Some(cat_apps) = categories.get(cat_name) {
                if !cat_apps.is_empty() {
                    let cat_label = Label::new(Some(display_name));
                    cat_label.set_halign(gtk4::Align::Start);
                    cat_label.set_margin_start(12);
                    cat_label.set_margin_top(12);
                    cat_label.set_margin_bottom(4);
                    cat_label.add_css_class("heading");
                    apps_box.append(&cat_label);

                    for app in cat_apps.iter().take(8) {
                        apps_box.append(&create_app_button(app));
                    }
                }
            }
        }
    } else {
        let query_lower = query.to_lowercase();
        let mut found_apps: Vec<&DesktopApp> = apps.iter()
            .filter(|app| app.name.to_lowercase().contains(&query_lower))
            .collect();

        found_apps.sort_by_key(|app| {
            let name_lower = app.name.to_lowercase();
            if name_lower.starts_with(&query_lower) {
                0
            } else {
                name_lower.find(&query_lower).unwrap_or(usize::MAX)
            }
        });

        if found_apps.is_empty() {
            let no_results = Label::new(Some("No applications found"));
            no_results.set_margin_top(20);
            no_results.add_css_class("dim-label");
            apps_box.append(&no_results);
        } else {
            for app in found_apps.iter().take(20) {
                apps_box.append(&create_app_button(app));
            }
        }
    }
}

fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok() ||
    std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false)
}

fn setup_window_positioning(window: &ApplicationWindow) {
    #[cfg(feature = "wayland")]
    {
        if is_wayland() {
            // Use layer shell on Wayland
            window.init_layer_shell();
            window.set_layer(Layer::Top);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, true);
            window.set_margin(Edge::Top, 8);
            window.set_margin(Edge::Right, 8);
            window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
            return;
        }
    }

    // Fallback for X11 or when wayland feature is disabled
    window.set_decorated(true);

    // On X11, use window manager rules to position the window
    // The window will appear as a normal window that cam be positioned by the WM
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Deskbar")
        .default_width(280)
        .default_height(500)
        .build();

    // Setup positioning based on display server
    setup_window_positioning(&window);

    let main_box = Box::new(Orientation::Vertical, 0);

    // Header with clock and date
    let header_box = Box::new(Orientation::Vertical, 4);
    header_box.set_margin_top(12);
    header_box.set_margin_bottom(8);
    header_box.set_margin_start(12);
    header_box.set_margin_end(12);

    let time_label = Label::new(Some(&chrono::Local::now().format("%H:%M").to_string()));
    time_label.set_halign(gtk4::Align::Start);
    time_label.add_css_class("title-1");
    header_box.append(&time_label);

    let date_label = Label::new(Some(&chrono::Local::now().format("%A, %B %e").to_string()));
    date_label.set_halign(gtk4::Align::Start);
    date_label.add_css_class("dim-label");
    header_box.append(&date_label);

    // Update time every minute
    let time_label_clone = time_label.clone();
    let date_label_clone = date_label.clone();
    glib::timeout_add_seconds_local(60, move || {
        let now = chrono::Local::now();
        time_label_clone.set_text(&now.format("%H:%M").to_string());
        date_label_clone.set_text(&now.format("%A, %B %e").to_string());
        glib::ControlFlow::Continue
    });

    main_box.append(&header_box);
    main_box.append(&Separator::new(Orientation::Horizontal));

    // Search
    let search_entry = SearchEntry::new();
    search_entry.set_margin_top(12);
    search_entry.set_margin_start(12);
    search_entry.set_margin_end(12);
    search_entry.set_placeholder_text(Some("Search applications..."));
    main_box.append(&search_entry);

    // Apps list
    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_margin_top(8);
    scrolled.set_margin_bottom(8);
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

    let apps_box = Box::new(Orientation::Vertical, 0);

    let apps = scan_applications();
    let categories = categorize_apps(&apps);

    rebuild_app_list(&apps_box, &apps, &categories, "");

    // Search functionality
    let apps_box_clone = apps_box.clone();
    let all_apps = apps.clone();
    let all_categories = categories.clone();
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_string();
        rebuild_app_list(&apps_box_clone, &all_apps, &all_categories, &query);
    });

    scrolled.set_child(Some(&apps_box));
    main_box.append(&scrolled);

    // Footer with system controls
    main_box.append(&Separator::new(Orientation::Horizontal));

    let footer_box = Box::new(Orientation::Horizontal, 8);
    footer_box.set_margin_top(8);
    footer_box.set_margin_bottom(12);
    footer_box.set_margin_start(12);
    footer_box.set_margin_end(12);
    footer_box.set_homogeneous(true);

    let logout_btn = Button::with_label("Log Out");
    logout_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("loginctl")
            .arg("terminate-user")
            .arg(std::env::var("USER").unwrap_or_default())
            .spawn();
    });
    footer_box.append(&logout_btn);

    let shutdown_btn = Button::with_label("Shutdown");
    shutdown_btn.connect_clicked(|_| {
        let _ = std::process::Command::new("systemctl").arg("poweroff").spawn();
    });
    footer_box.append(&shutdown_btn);

    main_box.append(&footer_box);

    // Apply custom CSS
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data("
           window {
               background-color: @theme_bg_color;
           }
           .heading {
               font-weight: bold;
               font-size: 0.9em;
           }
           button:hover {
               background-color: alpha(@theme_fg_color, 0.1);
           }
        ");

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().unwrap(),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    window.set_child(Some(&main_box));
    window.present();
}

fn main() {
    let app = Application::builder()
        .application_id("dpdns.org.pisekpiskovec.lacker")
        .build();

    app.connect_activate(build_ui);
    app.run();
}
