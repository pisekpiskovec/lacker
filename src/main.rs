use gtk4::prelude::*;
use gtk4::glib;
use gtk4::{Application, ApplicationWindow, Box, Button, Image, Label, Orientation, ScrolledWindow, Popover, Separator};
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
        let mut placed = false;
        for cat in &app.categories {
            let category = match cat.as_str() {
                "Utility" => "Utilities",
                "Development" => "Development",
                "Graphics" => "Graphics",
                "Network" => "Internet",
                "Office" => "Office",
                "AudioVideo" => "Multimedia",
                "System" => "System",
                "Game" => "Games",
                "Settings" => "Preferences",
                _ => continue,
            };
            categories.entry(category.to_string())
                .or_insert_with(Vec::new)
                .push(app.clone());
            placed = true;
            break; // Only plce in first matching category
        }

        if !placed {
            categories.entry("Other".to_string())
                .or_insert_with(Vec::new)
                .push(app.clone());
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

fn create_app_menu_item(app: &DesktopApp) -> Button {
    let button_box = Box::new(Orientation::Horizontal, 8);
    button_box.set_margin_start(8);
    button_box.set_margin_end(8);
    button_box.set_margin_top(2);
    button_box.set_margin_bottom(2);

    if let Some(icon_name) = &app.icon {
        let icon = Image::from_icon_name(icon_name);
        icon.set_pixel_size(16);
        button_box.append(&icon);
    }

    let label = Label::new(Some(&app.name));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    button_box.append(&label);

    let button = Button::new();
    button.set_child(Some(&button_box));
    button.set_has_frame(false);
    button.add_css_class("menu-item");

    let exec = app.exec.clone();
    button.connect_clicked(move |_| {
        launch_app(&exec);
    });

    button
}

fn create_apps_menu(categories: &HashMap<String, Vec<DesktopApp>>) -> Box {
    let menu_box = Box::new(Orientation::Vertical, 0);
    menu_box.set_width_request(250);

    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_max_content_height(500);
    scrolled.set_propagate_natural_height(true);

    let content_box = Box::new(Orientation::Vertical, 0);

    let priority_cats = vec![
        "Utilities",
        "Development",
        "Graphics",
        "Internet",
        "Office",
        "Multimedia",
        "System",
        "Games",
        "Preferences",
        "Other",
    ];

    for cat_name in priority_cats {
        if let Some(cat_apps) = categories.get(cat_name) {
            if !cat_apps.is_empty() {
                let cat_label = Label::new(Some(cat_name));
                cat_label.set_xalign(0.0);
                cat_label.set_margin_start(8);
                cat_label.set_margin_top(8);
                cat_label.set_margin_bottom(2);
                cat_label.add_css_class("category-label");
                content_box.append(&cat_label);

                for app in cat_apps {
                    content_box.append(&create_app_menu_item(app));
                }
            }
        }
    }

    scrolled.set_child(Some(&content_box));
    menu_box.append(&scrolled);

    menu_box
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
            window.set_margin(Edge::Top, 0);
            window.set_margin(Edge::Right, 0);
            window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);
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
        .title("Lacker")
        .default_width(180)
        .default_height(600)
        .build();

    // Setup positioning based on display server
    setup_window_positioning(&window);

    let main_box = Box::new(Orientation::Vertical, 0);
    main_box.add_css_class("deskbar");

    // Top section with leaf button
    let top_box = Box::new(Orientation::Horizontal, 0);

    // Leaf menu
    let leaf_btn = Button::new();
    leaf_btn.set_label("🦶");
    leaf_btn.set_has_frame(false);
    leaf_btn.add_css_class("leaf-button");
    leaf_btn.set_hexpand(true);

    // Create apps menu popover
    let apps = scan_applications();
    let categories = categorize_apps(&apps);
    let menu_content = create_apps_menu(&categories);

    let popover = Popover::new();
    popover.set_child(Some(&menu_content));
    popover.set_parent(&leaf_btn);

    leaf_btn.connect_clicked(move |btn| {
        if let Some(popover) = btn.child().and_then(|_| {
            // Get the popover from the button's parent
            btn.ancestor(gtk4::Window::static_type())
                .and_then(|w| w.downcast::<gtk4::Window>().ok())
                .and_then(|_| Some(popover.clone()))
        }) {
            popover.popup();
        } else {
            popover.popup();
        }
    });

    top_box.append(&leaf_btn);
    main_box.append(&top_box);
    main_box.append(&Separator::new(Orientation::Horizontal));

    // System tray area (placeholder)
    let tray_box = Box::new(Orientation::Horizontal, 4);
    // let tray_box = Box::new(Orientation::Horizontal, 0);
    tray_box.set_margin_top(4);
    tray_box.set_margin_bottom(4);

    // Mock systray icons
    for icon in ["🔊", "🌐", "🔋"] {
        let tray_icon_box = Box::new(Orientation::Horizontal, 4);
        tray_icon_box.set_margin_start(4);
        tray_icon_box.set_margin_end(4);

        let tray_icon = Button::new();
        tray_icon.set_label(icon);
        tray_icon.set_has_frame(false);
        tray_icon.add_css_class("tray-icon");
        tray_box.append(&tray_icon);

        tray_box.append(&tray_icon_box);
    }

    // Clock in tray area
    let clock_box = Box::new(Orientation::Horizontal, 4);
    clock_box.set_margin_start(4);
    clock_box.set_margin_end(4);
    clock_box.set_margin_top(4);

    let clock_btn = Button::new();
    let time_label = Label::new(Some(&chrono::Local::now().format("%H:%M").to_string()));
    time_label.add_css_class("clock-label");
    clock_btn.set_child(Some(&time_label));
    clock_btn.set_has_frame(false);
    clock_btn.add_css_class("clock-button");

    // Update time every minute
    let time_label_clone = time_label.clone();
    glib::timeout_add_seconds_local(60, move || {
        time_label_clone.set_text(&chrono::Local::now().format("%H:%M").to_string());
        glib::ControlFlow::Continue
    });

    clock_box.append(&clock_btn);
    tray_box.append(&clock_box);

    main_box.append(&tray_box);
    main_box.append(&Separator::new(Orientation::Horizontal));

    // Running applications area
    let apps_label = Label::new(Some("Running Applications"));
    apps_label.set_xalign(0.0);
    apps_label.set_margin_start(8);
    apps_label.set_margin_top(8);
    apps_label.set_margin_bottom(4);
    apps_label.add_css_class("section-label");
    main_box.append(&apps_label);

    let running_apps_box = Box::new(Orientation::Vertical, 2);
    running_apps_box.set_margin_start(4);
    running_apps_box.set_margin_end(4);
    running_apps_box.set_vexpand(true);

// Mock running applications
    for app_name in ["Terminal", "WebPositive", "Tracker"] {
        let app_btn_box = Box::new(Orientation::Horizontal, 8);
        app_btn_box.set_margin_start(4);
        app_btn_box.set_margin_end(4);
        
        let icon = Image::from_icon_name("application-x-executable");
        icon.set_pixel_size(16);
        app_btn_box.append(&icon);
        
        let label = Label::new(Some(app_name));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        app_btn_box.append(&label);
        
        let app_btn = Button::new();
        app_btn.set_child(Some(&app_btn_box));
        app_btn.set_has_frame(false);
        app_btn.add_css_class("running-app");
        
        running_apps_box.append(&app_btn);
    }

    main_box.append(&running_apps_box);

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data("
        .deskbar {
            background-color: @theme_bg_color;
            border-bottom: 1px solid alpha(@theme_fg_color, 0.15);
        }
        .leaf-button {
            font-size: 1.3em;
            padding: 6px 12px;
            min-width: 40px;
        }
        .leaf-button:hover {
            background-color: alpha(@theme_fg_color, 0.1);
        }
        .category-label {
            font-weight: bold;
            font-size: 0.85em;
            color: alpha(@theme_fg_color, 0.7);
        }
        .menu-item {
            padding: 4px 8px;
            min-height? 28px;
        }
        .menu-item:hover {
            background-color: alpha(@theme_fg_color, 0.8);
        }
        .tray-icon {
            padding: 4px 8px;
            min-width: 32px;
            font-size: 1.1em;
        }
        .tray-icon:hover {
            background-color: alpha(@theme_fg_color, 0.1);
        }
        .clock-button {
            padding: 4px 8px;
            min-width: 60px;
        }
        .clock-button:hover {
            background-color: alpha(@theme_fg_color, 0.1);
        }
        .clock-label {
            font-size: 0.9em;
            font-family: monospace;
        }
        .section-label {
            font-size: 0.85em;
            font-weight: bold;
            color: alpha(@theme_fg_color, 0.7);
        }
        .running-app {
            padding: 4px 8px;
            min-height: 28px;
        }
        .running-app:hover {
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
