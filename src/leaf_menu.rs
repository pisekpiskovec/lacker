use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation, ScrolledWindow, Separator, Popover, Window, Image, SearchEntry};
use gtk4::Box as GtkBox;
use std::fs;
use std::process::Command;

use crate::DesktopApp;

pub fn create_leaf_menu(parent: &Button, apps: &[DesktopApp]) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(200);

    // About Lacker
    let about_item = create_menu_item("About Lacker", None);
    about_item.connect_clicked(|_| {
        show_about_window();
    });
    menu_box.append(&about_item);


    // Find...
    let find_item = create_menu_item("Find...", None);
    find_item.connect_clicked(|_| {
        show_find_window();
    });
    menu_box.append(&find_item);

    // Mount submenu
    let mount_item = create_menu_item("Mount", Some(">"));
    let mount_popover = create_mount_popover(&mount_item);
    mount_item.connect_clicked(move |_| {
        mount_popover.popup();
    });
    menu_box.append(&mount_item);

    // Separator
    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Shutdown...
    let shutdown_item = create_menu_item("Shutdown", Some(">"));
    let shutdown_popover = create_shutdown_popover(&shutdown_item);
    let shutdown_item_clone = shutdown_item.clone();
    shutdown_item.connect_clicked(move |_| {
        shutdown_popover.popup();
    });
    menu_box.append(&shutdown_item_clone);

    // Separator
    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Recent documents
    let recent_docs_item = create_menu_item("Recent documents", Some(">"));
    let recent_docs_popover = create_recent_documents_popover(&recent_docs_item);
    recent_docs_item.connect_clicked(move |_| {
        recent_docs_popover.popup();
    });
    menu_box.append(&recent_docs_item);

    // Recent folders
    let recent_folders_item = create_menu_item("Recent folders", Some(">"));
    let recent_folders_popover = create_recent_folders_popover(&recent_folders_item);
    recent_folders_item.connect_clicked(move |_| {
        recent_folders_popover.popup();
    });
    menu_box.append(&recent_folders_item);

    // Recent applications
    let recent_apps_item = create_menu_item("Recent applications", Some(">"));
    let recent_apps_popover = create_recent_applications_popover(&recent_apps_item, apps);
    recent_apps_item.connect_clicked(move |_| {
        recent_apps_popover.popup();
    });
    menu_box.append(&recent_apps_item);

    // Separator
    menu_box.append(&Separator::new(Orientation::Horizontal));

    // Application
    let apps_item = create_menu_item("Applications", Some(">"));
    let apps_clone = apps.to_vec();
    let apps_popover = create_applications_popover(&apps_item, &apps_clone);
    apps_item.connect_clicked(move |_| {
        apps_popover.popup();
    });
    menu_box.append(&apps_item);

    // Preferences
    let prefs_item = create_menu_item("Preferences", Some(">"));
    let prefs_popover = create_preferences_popover(&prefs_item);
    prefs_item.connect_clicked(move |_| {
        prefs_popover.popup();
    });
    menu_box.append(&prefs_item);


    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);

    popover
}

fn create_menu_item(label: &str, arrow: Option<&str>) -> Button {
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    let label_widget = Label::new(Some(label));
    label_widget.set_xalign(0.0);
    label_widget.set_hexpand(true);
    button_box.append(&label_widget);

    if let Some(arrow_text) = arrow {
        let arrow_label = Label::new(Some(arrow_text));
        button_box.append(&arrow_label);
    }

    let button = Button::new();
    button.set_child(Some(&button_box));
    button.set_has_frame(false);
    button.add_css_class("menu-item");

    button
}

fn show_about_window() {
    let window = Window::new();
    window.set_title(Some("About Lacker"));
    window.set_default_size(450, 350);
    window.set_resizable(false);

    let content_box = GtkBox::new(Orientation::Vertical, 12);

    // Try to show distro logo
    let logo_paths = vec![
        "/usr/share/pixmaps/fedora-logo.png",
        "/usr/share/pixmaps/ubuntu-logo.png",
        "/usr/share/pixmaps/debian-logo.png",
        "/usr/share/pixmaps/arch-logo.png",
    ];

    for logo_path in logo_paths {
        if std::path::Path::new(logo_path).exists() {
            let logo = Image::from_file(logo_path);
            logo.set_pixel_size(64);
            content_box.append(&logo);
            break;
        }
    }

    // Program info
    let program_label = Label::new(Some("Lacker"));
    program_label.add_css_class("title-1");
    content_box.append(&program_label);

    let version_label = Label::new(Some("0.4.0"));
    version_label.add_css_class("dim-label");
    content_box.append(&version_label);

    // Separator
    content_box.append(&Separator::new(Orientation::Horizontal));

    // Distro info
    if let Ok(os_info) = fs::read_to_string("/etc/os-release") {
        for line in os_info.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let distro = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                let distro_label = Label::new(Some(&format!("Distribution: {}", distro)));
                distro_label.set_xalign(0.0);
                content_box.append(&distro_label);
                break;
            }
        }
    }

    // Processor info
    if let Ok(cpu_info) = fs::read_to_string("/proc/cpuinfo") {
        for line in cpu_info.lines() {
            if line.starts_with("model name") {
                if let Some(cpu) = line.split(':').nth(1) {
                    let cpu_label = Label::new(Some(&format!("Processor: {}", cpu.trim())));
                    cpu_label.set_xalign(0.0);
                    cpu_label.set_wrap(true);
                    cpu_label.set_max_width_chars(40);
                    content_box.append(&cpu_label);
                    break;
                }
            }
        }
    }

    // RAM info
    if let Ok(mem_info) = fs::read_to_string("/proc/meminfo") {
        for line in mem_info.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(mem) = line.split_whitespace().nth(1) {
                    if let Ok(mem_kb) = mem.parse::<u64>() {
                        let mem_gb = mem_kb as f64 / 1024.0 / 1024.0;
                        let ram_label = Label::new(Some(&format!("RAM: {:.1} GB", mem_gb)));
                        ram_label.set_xalign(0.0);
                        content_box.append(&ram_label);
                        break;
                    }
                }
            }
        }
    }

    window.set_child(Some(&content_box));
    window.present();
}

fn show_find_window() {
    let window = Window::new();
    window.set_title(Some("Find"));
    window.set_default_size(600, 400);

    let content_box = GtkBox::new(Orientation::Vertical, 8);

    // Search entry
    let search_entry = SearchEntry::new();
    content_box.append(&search_entry);

    // Results area
    let scorlled = ScrolledWindow::new();
    scorlled.set_vexpand(true);

    let results_box = GtkBox::new(Orientation::Vertical, 4);

    let placeholder = Label::new(Some("Enter search terms to find files"));
    placeholder.add_css_class("dim-label");
    results_box.append(&placeholder);

    scorlled.set_child(Some(&results_box));
    content_box.append(&scorlled);

    // Search functionality
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_string();
        if query.len() < 3 {
            return;
        }

        // Clear results
        while let Some(child) = results_box.first_child() {
            results_box.remove(&child);
        }

        // Simple find using 'locate' command
        if let Ok(output) = Command::new("locate")
            .arg("-i")
            .arg("-l")
            .arg("20")
            .arg(&query)
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for path in stdout.lines().take(20) {
                let result_btn = Button::with_label(path);
                result_btn.set_has_frame(false);
                result_btn.add_css_class("menu-item");

                let path_clone = path.to_string();
                result_btn.connect_clicked(move |_| {
                    let _ = Command::new("xdg-open").arg(&path_clone).spawn();
                });

                results_box.append(&result_btn);
            }
        }
    });

    window.set_child(Some(&content_box));
    window.present();
}

fn create_mount_popover(parent: &Button) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(200);


    // Get unmounted drives
    if let Ok(output) = Command::new("lsblk")
        .args(&["-nlo", "NAME,MOUNTPOINT,SIZE,LABEL"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut found_unmounted = false;

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].is_empty() {
                // Unmounted partition
                found_unmounted = true;
                let name = parts[0];
                let size = parts.get(2).unwrap_or(&"");
                let label = parts.get(3..).map(|s| s.join(" ")).unwrap_or_default();

                let display = if !label.is_empty() {
                    format!("{} ({})", label, size)
                } else {
                    format!("/dev/{} ({})", name, size)
                };

                let mount_item = create_menu_item(&display, None);
                let device = format!("/dev/{}", name);
                mount_item.connect_clicked(move |_| {
                    // Try to mound
                    let _ = Command::new("udisksctl")
                        .arg("mount")
                        .arg("-b")
                        .arg(&device)
                        .spawn();
                });
                menu_box.append(&mount_item);
            }
        }

        if !found_unmounted {
            let no_drives = Label::new(Some("No unmounted drives"));
            no_drives.add_css_class("dim-label");
            menu_box.append(&no_drives);
        }
    }

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_shutdown_popover(parent: &Button) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(150);

    // Shutdown
    let shutdown_item = create_menu_item("Power off", None);
    shutdown_item.connect_clicked(|_| {
        let _ = Command::new("systemctl").arg("poweroff").spawn();
    });
    menu_box.append(&shutdown_item);

    // Reboot
    let reboot_item = create_menu_item("Restart system", None);
    reboot_item.connect_clicked(|_| {
        let _ = Command::new("systemctl").arg("reboot").spawn();
    });
    menu_box.append(&reboot_item);

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_recent_documents_popover(parent: &Button) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(250);

    // Read recent files from GTK recent manager
    let recent_file = format!("{}/.local/share/recently-used.xbel",
        std::env::var("HOME").unwrap_or_default());

    if let Ok(content) = fs::read_to_string(&recent_file) {
        let mut count = 0;
        for line in content.lines() {
            if count >= 5 {
                break;
            }

            if line.contains("<bookmark href=\"file://") {
                if let Some(start) = line.find("file://") {
                    if let Some(end) = line[start..].find('"') {
                        let path = &line[start + 7..start + end];
                        let path_decoded = urlencoding::decode(path).unwrap_or_default();

                        if let Some(filename) = std::path::Path::new(path_decoded.as_ref())
                            .file_name()
                            .and_then(|n| n.to_str())
                        {
                            let doc_item = create_menu_item(filename, None);
                            let full_path = path_decoded.to_string();
                            doc_item.connect_clicked(move |_| {
                                let _ = Command::new("xdg-open").arg(&full_path).spawn();
                            });
                            menu_box.append(&doc_item);
                            count += 1;
                        }
                    }
                }
            }
        }

        if count == 0 {
            let no_docs = Label::new(Some("No recent documents"));
            no_docs.add_css_class("dim-label");
            menu_box.append(&no_docs);
        }
    } else {
        let no_docs = Label::new(Some("No recent documents"));
        no_docs.add_css_class("dim-label");
        menu_box.append(&no_docs);
    }

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_recent_folders_popover(parent: &Button) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(250);

    // Get recent directories from bash history or similar
    let recent_dirs = vec![
        std::env::var("HOME").unwrap_or_default() + "/Downloads",
        std::env::var("HOME").unwrap_or_default() + "/Documents",
        std::env::var("HOME").unwrap_or_default() + "/Pictures",
        std::env::var("HOME").unwrap_or_default() + "/Desktop",
    ];

    for dir_path in recent_dirs.iter().take(5) {
        if std::path::Path::new(dir_path).exists() {
            if let Some(dir_name) = std::path::Path::new(dir_path)
                .file_name()
                .and_then(|n| n.to_str())
            {
                let folder_item = create_menu_item(dir_name, None);
                let path_clone = dir_path.clone();
                folder_item.connect_clicked(move |_| {
                    let _ = Command::new("xdg-open").arg(&path_clone).spawn();
                });
                menu_box.append(&folder_item);
            }
        }
    }

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_recent_applications_popover(parent: &Button, apps: &[DesktopApp]) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(200);

    // Show first 5 apps as "recent" (in reality, would track actual usage)
    for app in apps.iter().take(5) {
        let app_item = create_menu_item(&app.name, None);
        let exec = app.exec.clone();
        app_item.connect_clicked(move |_| {
            crate::launch_app(&exec);
        });
        menu_box.append(&app_item);
    }

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_applications_popover(parent: &Button, apps: &[DesktopApp]) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(250);

    let scrolled = ScrolledWindow::new();
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_propagate_natural_height(true);

    let apps_list_box = GtkBox::new(Orientation::Vertical, 0);

    // Sort alphabetically
    let mut sorted_apps = apps.to_vec();
    sorted_apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    for app in sorted_apps {
        let app_box = GtkBox::new(Orientation::Horizontal, 8);
        if let Some(icon_name) = &app.icon {
            let icon = Image::from_icon_name(icon_name);
            icon.set_pixel_size(16);
            app_box.append(&icon);
        }

        let label = Label::new(Some(&app.name));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        app_box.append(&label);

        let app_item = Button::new();
        app_item.set_child(Some(&app_box));
        app_item.set_has_frame(false);
        app_item.add_css_class("menu-item");

        let exec = app.exec.clone();
        app_item.connect_clicked(move |_| {
            crate::launch_app(&exec);
        });

        apps_list_box.append(&app_item);
    }

    scrolled.set_child(Some(&apps_list_box));
    menu_box.append(&scrolled);

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}

fn create_preferences_popover(parent: &Button) -> Popover {
    let menu_box = GtkBox::new(Orientation::Vertical, 0);
    menu_box.set_width_request(200);

    let prefs = vec![
        ("System Settings", "gnome-control-center"),
        ("Network", "nm-connection-editor"),
        ("Display", "arandr"),
        ("Sound", "pavucontrol"),
    ];

    for (name, command) in prefs {
        let pref_item = create_menu_item(name, None);
        let cmd = command.to_string();
        pref_item.connect_clicked(move |_| {
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if !parts.is_empty() {
                let mut command = Command::new(parts[0]);
                if parts.len() > 1 {
                    command.args(&parts[1..]);
                }
                let _ = command.spawn();
            }
        });
        menu_box.append(&pref_item);
    }

    let popover = Popover::new();
    popover.set_child(Some(&menu_box));
    popover.set_parent(parent);
    popover
}
