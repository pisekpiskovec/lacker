use gtk4::prelude::*;
use gtk4::glib;
use gtk4::{Application, ApplicationWindow, Box, Button, Image, Label, Orientation, ScrolledWindow, Popover, Separator};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

#[cfg(feature = "wayland")]
use gtk4_layer_shell::{Edge, Layer, LayerShell};

#[derive(Clone, Debug)]
struct DesktopApp {
    name: String,
    exec: String,
    icon: Option<String>,
    categories: Vec<String>,
}

#[derive(Clone, Debug)]
struct RunningWindow {
    wid: String,
    title: String,
    app_name: String,
}

#[derive(Clone, Debug)]
struct RunningApp {
    name: String,
    icon: Option<String>,
    windows: Vec<RunningWindow>,
    pid: Option<u32>,
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
                                if de.no_display() || de.terminal() {
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
            break; // Only place in first matching category
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

fn get_app_icon(app_name: &str, desktop_apps: &[DesktopApp]) -> Option<String> {
    let app_lower = app_name.to_lowercase();

    // Try exact match first
    for app in desktop_apps {
        if app.name.to_lowercase() == app_lower {
            return app.icon.clone();
        }
    }

    // Try partial match (app name contains or is contained in desktop app name)
    for app in desktop_apps {
        let desktop_lower = app.name.to_lowercase();
        if desktop_lower.contains(&app_lower) || app_lower.contains(&desktop_lower) {
            return app.icon.clone();
        }
    }

     // Try matching against exec command
     for app in desktop_apps {
         let exec_lower = app.exec.to_lowercase();
         if exec_lower.contains(&app_lower) {
             return app.icon.clone();
         }
     }

     // Fallback: use app name as icon name (GTK will try to find it)
     Some(app_lower)
}

fn get_app_display_name (app_name: &str, desktop_apps: &[DesktopApp]) -> String {
    let app_lower = app_name.to_lowercase();

    // Try exact match first
    for app in desktop_apps {
        if app.name.to_lowercase() == app_lower {
            return app.name.clone();
        }
    }

    // Try partial match (app name contains or is contained in desktop app name)
    for app in desktop_apps {
        let desktop_lower = app.name.to_lowercase();
        if desktop_lower.contains(&app_lower) || app_lower.contains(&desktop_lower) {
            return app.name.clone();
        }
    }

     // Try matching against exec command
     for app in desktop_apps {
         let exec_lower = app.exec.to_lowercase();
         if exec_lower.contains(&app_lower) {
             return app.name.clone();
         }
     }

     // Fallback: capitalize the first letter of app_name
     let mut chars = app_name.chars();
     match chars.next() {
         None => app_name.to_string(),
         Some(first) => first.to_uppercase().chain(chars).collect(),
     }
}

fn get_running_apps() -> Vec<RunningApp> {
    let mut apps_map: HashMap<String, RunningApp> = HashMap::new();

    // Try wmctrl first
    if let Ok(output) = Command::new("wmctrl").arg("-lx").output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);

            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let wid = parts[0].to_string();
                    let wm_class = parts[2];
                    let title = parts[4..].join(" ");

                    // Extract app name from WM_CLASS (format is instance.class)
                    let app_name = wm_class.split('.').last().unwrap_or(wm_class).to_string();

                    let window = RunningWindow {
                        wid: wid.clone(),
                        title: title.clone(),
                        app_name: app_name.clone(),
                    };

                    apps_map.entry(app_name.clone())
                        .or_insert_with(|| RunningApp {
                            name: app_name,
                            icon: None,
                            windows: Vec::new(),
                            pid: None,
                        })
                        .windows.push(window);
                }
            }
        }
    }

    // Alternative: Try using xdotool on X11
    if apps_map.is_empty() {
        if let Ok(output) = Command::new("xdotool").args(&["search", "--onlyvisible", "--class", ""]).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);

                for wid_str in stdout.lines() {
                    if let Ok(name_output) = Command::new("xdotool")
                        .args(&["getwindowname", wid_str])
                        .output()
                    {
                        let title = String::from_utf8_lossy(&name_output.stdout).trim().to_string();

                        if let Ok(class_output) = Command::new("xprop")
                            .args(&["-id", wid_str, "WM_CLASS"])
                            .output()
                        {
                            let class_str = String::from_utf8_lossy(&class_output.stdout);

                            // Parse WM_CLASS format: WM_CLASS(STRING) = "instance", "class"
                            let app_name = if let Some(class_part) = class_str.split('"').nth(3) {
                                class_part.to_string()
                            } else {
                                title.split_whitespace().next().unwrap_or("Unknown").to_string()
                            };

                            let window = RunningWindow {
                                wid: wid_str.to_string(),
                                title,
                                app_name: app_name.clone(),
                            };

                            apps_map.entry(app_name.clone())
                                .or_insert_with(|| RunningApp {
                                    name: app_name,
                                    icon: None,
                                    windows: Vec::new(),
                                    pid: None,
                                })
                                .windows.push(window);
                        }
                    }
                }
            }
        }
    }

    // Wayland fallback: try compositor-specific commands
    if apps_map.is_empty() && is_wayland() {
        // Try swaymsg for Sway/i3
        if let Ok(output) = Command::new("swaymsg").args(&["-t", "get_tree"]).output() {
            if output.status.success() {
                // Todo: JSON parsing
            }
        }

        // Try hyprctl for Hyprland
        if let Ok(output) = Command::new("hyprctl").args(&["clients", "-j"]).output() {
            if output.status.success() {
                // Todo: JSON parsing
            }
        }
    }
    
    let mut apps: Vec<RunningApp> = apps_map.into_values().collect();
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

fn focus_window(wid: &str) {
    std::thread::spawn({
        let wid = wid.to_string();
        move || {
            let _ = Command::new("wmctrl").args(&["-ia", &wid]).spawn();
        }
    });
}

fn close_window(wid: &str) {
    std::thread::spawn({
        let wid = wid.to_string();
        move || {
            let _ = Command::new("wmctrl").args(&["-ic", &wid]).spawn();
        }
    });
}

fn close_app(app: &RunningApp) {
    for window in &app.windows {
        if !window.wid.is_empty() {
            close_window(&window.wid);
        }
    }

    // If we have a PID, try to kill it as fallback
    if let Some(pid) = app.pid {
        std::thread::spawn(move || {
            let _ = Command::new("kill").arg(pid.to_string()).spawn();
        });
    }
}

fn create_app_menu_item(app: &DesktopApp) -> Button {
    let button_box = Box::new(Orientation::Horizontal, 8);

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
            window.set_anchor(Edge::Bottom, true);
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

    // Leaf icon button with popover menu
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

    leaf_btn.connect_clicked(move |_| {
            popover.popup();
    });

    top_box.append(&leaf_btn);
    main_box.append(&top_box);
    main_box.append(&Separator::new(Orientation::Horizontal));

    // System tray area (placeholder)
    let tray_box = Box::new(Orientation::Horizontal, 4);

    // Mock systray icons
    for icon in ["🔊", "🌐", "🔋"] {
        let tray_icon_box = Box::new(Orientation::Horizontal, 4);

        let tray_icon = Button::new();
        tray_icon.set_label(icon);
        tray_icon.set_has_frame(false);
        tray_icon.add_css_class("tray-icon");
        tray_icon_box.append(&tray_icon);

        tray_box.append(&tray_icon_box);
    }

    // Clock in tray area
    let clock_box = Box::new(Orientation::Horizontal, 4);

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
    apps_label.add_css_class("section-label");
    main_box.append(&apps_label);

    let running_apps_box = Box::new(Orientation::Vertical, 2);
    running_apps_box.set_vexpand(true);

    // Function to update running apps list
    let desktop_apps = apps.clone();
    let update_running_apps = {
        let running_apps_box = running_apps_box.clone();
        move || {
            // Clear existing
            while let Some(child) = running_apps_box.first_child() {
                running_apps_box.remove(&child);
            }

            let mut running_apps = get_running_apps();

            // Match icons and display names from desktop apps
            for app in &mut running_apps {
                app.icon = get_app_icon(&app.name, &desktop_apps);
                app.name = get_app_display_name(&app.name, &desktop_apps);
            }

            if running_apps.is_empty() {
                let empty_label = Label::new(Some("No running applications"));
                empty_label.add_css_class("dim-label");
                running_apps_box.append(&empty_label);
            } else {
                for app in running_apps {
                    let app_btn_box = Box::new(Orientation::Horizontal, 8);

                    let icon_name = app.icon.clone().unwrap_or_else(|| "application-x-executable".to_string());
                    let icon = Image::from_icon_name(&icon_name);
                    icon.set_pixel_size(16);
                    app_btn_box.append(&icon);

                    let label = Label::new(Some(&app.name));
                    label.set_xalign(0.0);
                    label.set_hexpand(true);
                    app_btn_box.append(&label);

                    let app_btn = Button::new();
                    app_btn.set_child(Some(&app_btn_box));
                    app_btn.set_has_frame(false);
                    app_btn.add_css_class("running-app");

                    // Create popover menu for this app
                    let menu_box = Box::new(Orientation::Vertical, 0);
                    menu_box.set_width_request(180);

                    // Add window items
                    for (idx, window) in app.windows.iter().enumerate() {
                        let win_btn_box = Box::new(Orientation::Horizontal, 4);

                        let win_label = Label::new(Some(&format!("Window {}", idx + 1)));
                        win_label.set_xalign(0.0);
                        win_label.set_hexpand(true);
                        win_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                        win_label.set_max_width_chars(20);
                        win_label.set_tooltip_text(Some(&window.title));
                        win_btn_box.append(&win_label);

                        let win_btn = Button::new();
                        win_btn.set_child(Some(&win_btn_box));
                        win_btn.set_has_frame(false);
                        win_btn.add_css_class("menu-item");

                        let wid = window.wid.clone();
                        win_btn.connect_clicked(move |_| {
                            focus_window(&wid);
                        });

                        menu_box.append(&win_btn);
                    }

                    // Add separator
                    menu_box.append(&Separator::new(Orientation::Horizontal));

                    // Add "Close all" button
                    let close_btn_box = Box::new(Orientation::Horizontal, 4);

                    let close_label = Label::new(Some("Close all"));
                    close_label.set_xalign(0.0);
                    close_label.set_hexpand(true);
                    close_btn_box.append(&close_label);

                    let close_btn = Button::new();
                    close_btn.set_child(Some(&close_btn_box));
                    close_btn.set_has_frame(false);
                    close_btn.add_css_class("menu-item");

                    let app_clone = app.clone();
                    close_btn.connect_clicked(move |_| {
                        close_app(&app_clone);
                    });

                    menu_box.append(&close_btn);

                    let app_popover = Popover::new();
                    app_popover.set_child(Some(&menu_box));
                    app_popover.set_parent(&app_btn);

                    app_btn.connect_clicked(move |_| {
                        app_popover.popup();
                    });

                    running_apps_box.append(&app_btn);
                }
            }
        }
    };

    // Initial update
    update_running_apps();

    // Update every 3 seconds
    glib::timeout_add_seconds_local(3, move || {
        update_running_apps();
        glib::ControlFlow::Continue
    });

    main_box.append(&running_apps_box);

    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_data("
        .deskbar {
            background-color: @theme_bg_color;
            border-left: 1px solid alpha(@theme_fg_color, 0.15);
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
            min-height: 28px;
        }
        .menu-item:hover {
            background-color: alpha(@theme_selected_bg_color, 0.8);
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
        .dim-label {
            opacity: 0.6;
            font-size: 0.9em;
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
