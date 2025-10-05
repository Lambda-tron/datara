use eframe::egui;
use std::path::PathBuf;
use std::time::SystemTime;
use std::process::Command;

struct DataraApp {
    current_dir: PathBuf,
    entries: Vec<std::fs::DirEntry>,
    history: Vec<PathBuf>,
    future: Vec<PathBuf>,
    grid_view: bool,
    error: Option<String>,
    ui_scale: f32,
    max_items_per_row: i32,
    show_scanlines: bool,
    show_hidden: bool,
    folder_icon: Option<egui::TextureHandle>,
    file_icon: Option<egui::TextureHandle>,
    last_hovered_item: Option<usize>,
    scrolling_text: Option<(usize, f32)>, // (item_index, scroll_offset)
    horizontal_spacing: f32,
    vertical_spacing: f32,
    show_settings: bool,
}

impl DataraApp {
    fn new(start_dir: PathBuf) -> Self {
        let mut app = Self {
            current_dir: start_dir,
            entries: Vec::new(),
            history: Vec::new(),
            future: Vec::new(),
            grid_view: true,
            error: None,
            ui_scale: 1.0,
            max_items_per_row: 3,
            show_scanlines: false,
            show_hidden: false,
            folder_icon: None,
            file_icon: None,
            last_hovered_item: None,
            scrolling_text: None,
            horizontal_spacing: 16.0,
            vertical_spacing: 12.0,
            show_settings: false,
        };
        app.read_dir();
        app.load_settings();
        app
    }

    fn read_dir(&mut self) {
        self.entries.clear();
        self.error = None;
        match std::fs::read_dir(&self.current_dir) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    if !self.show_hidden {
                        let name = entry.file_name();
                        if let Some(s) = name.to_str() {
                            if s.starts_with('.') { continue; }
                        }
                    }
                    self.entries.push(entry);
                }
                self.entries.sort_by(|a, b| {
                    let a_meta = a.metadata();
                    let b_meta = b.metadata();
                    let a_is_dir = a_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let b_is_dir = b_meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.file_name().to_string_lossy().to_lowercase().cmp(&b.file_name().to_string_lossy().to_lowercase()),
                    }
                });
            }
            Err(err) => {
                self.error = Some(format!("Failed to read dir: {}", err));
            }
        }
    }

    fn navigate_to(&mut self, path: PathBuf, push_history: bool) {
        if push_history {
            self.history.push(self.current_dir.clone());
            self.future.clear();
        }
        self.current_dir = path;
        self.read_dir();
    }

    fn navigate_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.navigate_to(parent.to_path_buf(), true);
        }
    }

    fn navigate_back(&mut self) {
        if let Some(prev) = self.history.pop() {
            self.future.push(self.current_dir.clone());
            self.current_dir = prev;
            self.read_dir();
        }
    }

    fn navigate_forward(&mut self) {
        if let Some(next) = self.future.pop() {
            self.history.push(self.current_dir.clone());
            self.current_dir = next;
            self.read_dir();
        }
    }

    fn entry_label(entry: &std::fs::DirEntry) -> String {
        let name = entry.file_name().to_string_lossy().to_string();
        match entry.metadata() {
            Ok(meta) if meta.is_dir() => format!("📁 {}", name),
            Ok(_) => format!("📄 {}", name),
            Err(_) => name,
        }
    }

    fn entry_name(entry: &std::fs::DirEntry) -> String {
        entry.file_name().to_string_lossy().to_string()
    }

    fn entry_info(entry: &std::fs::DirEntry) -> (bool, Option<u64>, Option<SystemTime>) {
        match entry.metadata() {
            Ok(meta) => {
                let is_dir = meta.is_dir();
                let len = if is_dir { None } else { Some(meta.len()) };
                let modified = meta.modified().ok();
                (is_dir, len, modified)
            }
            Err(_) => (false, None, None),
        }
    }

    fn format_size(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        let b = bytes as f64;
        if b >= GB { format!("{} GB", (b / GB) as u64) }
        else if b >= MB { format!("{} MB", (b / MB) as u64) }
        else if b >= KB { format!("{} KB", (b / KB) as u64) }
        else { format!("{} B", bytes) }
    }

    fn format_date(time: SystemTime) -> String {
        // Fallback to RFC3339-like; egui has no tz; keep simple
        let datetime: chrono::DateTime<chrono::Local> = time.into();
        datetime.format("%b %d, %Y %H:%M").to_string()
    }

    fn load_icons(&mut self, ctx: &egui::Context) {
        if self.folder_icon.is_none() {
            // Load folder icon
            if let Ok(image_data) = std::fs::read("src/icons/Folder/icons8-folder-48.png") {
                if let Ok(image) = image::load_from_memory(&image_data) {
                    let rgba_image = image.to_rgba8();
                    let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                    let pixels = rgba_image.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    self.folder_icon = Some(ctx.load_texture("folder_icon", color_image, egui::TextureOptions::default()));
                }
            }
        }
        
        if self.file_icon.is_none() {
            // Load file icon
            if let Ok(image_data) = std::fs::read("src/icons/File/icons8-file-48.png") {
                if let Ok(image) = image::load_from_memory(&image_data) {
                    let rgba_image = image.to_rgba8();
                    let size = [rgba_image.width() as usize, rgba_image.height() as usize];
                    let pixels = rgba_image.into_raw();
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                    self.file_icon = Some(ctx.load_texture("file_icon", color_image, egui::TextureOptions::default()));
                }
            }
        }
    }

    fn play_hover_sound(&self) {
        // Use system beep command for hover sound (high frequency)
        let _ = Command::new("beep")
            .args(&["-f", "800", "-l", "100"])
            .spawn();
    }

    fn play_click_sound(&self) {
        // Use system beep command for click sound (lower frequency)
        let _ = Command::new("beep")
            .args(&["-f", "600", "-l", "150"])
            .spawn();
    }

    fn open_file(&self, path: &std::path::Path) {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            
            match ext_lower.as_str() {
                // Text files - open in VIM
                "txt" | "md" | "rs" | "py" | "js" | "html" | "css" | "json" | "xml" | "yml" | "yaml" | "toml" | "ini" | "cfg" | "conf" | "log" | "c" | "cpp" | "h" | "hpp" | "java" | "go" | "php" | "rb" | "sh" | "bash" | "zsh" | "fish" => {
                    let _ = Command::new("gnome-terminal")
                        .args(&["--", "vim", path.to_str().unwrap_or("")])
                        .spawn();
                },
                // Videos - open in MPV
                "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "3gp" | "ogv" | "mpeg" | "mpg" => {
                    let _ = Command::new("mpv")
                        .arg(path.to_str().unwrap_or(""))
                        .spawn();
                },
                // Images - open in Firefox
                "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "tiff" | "ico" => {
                    let _ = Command::new("firefox")
                        .arg(path.to_str().unwrap_or(""))
                        .spawn();
                },
                // PDFs - open in Firefox
                "pdf" => {
                    let _ = Command::new("firefox")
                        .arg(path.to_str().unwrap_or(""))
                        .spawn();
                },
                // Audio files - open in MPV
                "mp3" | "wav" | "flac" | "ogg" | "aac" | "m4a" | "wma" => {
                    let _ = Command::new("mpv")
                        .arg(path.to_str().unwrap_or(""))
                        .spawn();
                },
                // Default - try to open with system default
                _ => {
                    let _ = Command::new("xdg-open")
                        .arg(path.to_str().unwrap_or(""))
                        .spawn();
                }
            }
        } else {
            // No extension - try system default
            let _ = Command::new("xdg-open")
                .arg(path.to_str().unwrap_or(""))
                .spawn();
        }
    }

    fn truncate_text(&self, text: &str, max_width: f32, font_size: f32) -> String {
        // Simple character-based truncation with proper UTF-8 handling
        let char_width = font_size * 0.6; // Approximate character width
        let max_chars = (max_width / char_width) as usize;
        
        // Convert to chars for proper UTF-8 handling
        let chars: Vec<char> = text.chars().collect();
        
        if chars.len() <= max_chars {
            text.to_string()
        } else {
            let truncate_at = max_chars.saturating_sub(3);
            let truncated: String = chars[..truncate_at].iter().collect();
            format!("{}...", truncated)
        }
    }

    fn get_scrolling_text(&self, text: &str, max_width: f32, font_size: f32, _item_index: usize, is_hovered: bool, time: f32) -> (String, f32) {
        let char_width = font_size * 0.6;
        let max_chars = (max_width / char_width) as usize;
        
        // Convert to chars for proper UTF-8 handling
        let chars: Vec<char> = text.chars().collect();
        
        if chars.len() <= max_chars {
            return (text.to_string(), 0.0);
        }
        
        if is_hovered {
            // Use a simple time-based approach that starts immediately
            let total_text_width = chars.len() as f32 * char_width;
            let visible_width = max_chars as f32 * char_width;
            let scroll_range = total_text_width - visible_width;
            
            // Calculate scroll position based on time with immediate start
            let cycle_time = 4.0; // seconds for full cycle (2s right, 2s left)
            let normalized_time = (time % cycle_time) / cycle_time;
            
            let scroll_offset = if normalized_time < 0.5 {
                // First half: scroll right (0 to max)
                normalized_time * 2.0 * scroll_range
            } else {
                // Second half: scroll left (max to 0)
                (1.0 - (normalized_time - 0.5) * 2.0) * scroll_range
            };
            
            let start_char = (scroll_offset / char_width) as usize;
            let visible_chars = max_chars;
            
            if start_char + visible_chars <= chars.len() {
                let result: String = chars[start_char..start_char + visible_chars].iter().collect();
                (result, 0.0)
            } else {
                // Wrap around
                let end_part: String = chars[start_char..].iter().collect();
                let remaining = visible_chars - (chars.len() - start_char);
                let start_part: String = chars[..remaining].iter().collect();
                (format!("{}{}", end_part, start_part), 0.0)
            }
        } else {
            // Show truncated text with ellipsis
            (self.truncate_text(text, max_width, font_size), 0.0)
        }
    }

    fn save_settings(&self) {
        let settings = format!(
            "ui_scale={}\nmax_items_per_row={}\nshow_scanlines={}\nshow_hidden={}\nhorizontal_spacing={}\nvertical_spacing={}\n",
            self.ui_scale, self.max_items_per_row, self.show_scanlines, self.show_hidden, self.horizontal_spacing, self.vertical_spacing
        );
        let _ = std::fs::write("datara_settings.txt", settings);
    }

    fn load_settings(&mut self) {
        if let Ok(contents) = std::fs::read_to_string("datara_settings.txt") {
            for line in contents.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    match key {
                        "ui_scale" => if let Ok(val) = value.parse::<f32>() { self.ui_scale = val; },
                        "max_items_per_row" => if let Ok(val) = value.parse::<i32>() { self.max_items_per_row = val.clamp(2, 5); },
                        "show_scanlines" => if let Ok(val) = value.parse::<bool>() { self.show_scanlines = val; },
                        "show_hidden" => if let Ok(val) = value.parse::<bool>() { self.show_hidden = val; },
                        "horizontal_spacing" => if let Ok(val) = value.parse::<f32>() { self.horizontal_spacing = val; },
                        "vertical_spacing" => if let Ok(val) = value.parse::<f32>() { self.vertical_spacing = val; },
                        _ => {}
                    }
                }
            }
        }
    }

}

impl eframe::App for DataraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load icons if not already loaded
        self.load_icons(ctx);
        
        let bg = egui::Color32::from_rgba_unmultiplied(0, 12, 0, 210);

        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::default().fill(bg))
            .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let back_enabled = !self.history.is_empty();
                let fwd_enabled = !self.future.is_empty();
                if ui.add_enabled(back_enabled, egui::Button::new("←")).clicked() {
                    self.navigate_back();
                }
                if ui.add_enabled(fwd_enabled, egui::Button::new("→")).clicked() {
                    self.navigate_forward();
                }
                if ui.button("↑").clicked() {
                    self.navigate_up();
                }
                ui.separator();
                ui.label(egui::RichText::new(self.current_dir.to_string_lossy()).monospace());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Settings button
                    if ui.button("⚙️").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                    ui.separator();
                    let label = if self.grid_view { "Grid" } else { "List" };
                    if ui.button(format!("View: {}", label)).clicked() {
                        self.grid_view = !self.grid_view;
                    }
                });
            });
        });

        // Settings window
        if self.show_settings {
            egui::Window::new("Settings")
                .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 50.0))
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Display Settings");
                        ui.separator();
                        
                        ui.add(egui::Slider::new(&mut self.max_items_per_row, 2..=5).text("Max Items Per Row"));
                        // Calculate max horizontal spacing based on available space
                        let max_h_spacing = if self.grid_view {
                            let available_width = ctx.screen_rect().width() - (32.0 * self.ui_scale); // Account for margins
                            let columns = self.max_items_per_row as f32;
                            let min_item_width = 200.0 * self.ui_scale; // Minimum reasonable item width
                            let max_spacing = (available_width - (columns * min_item_width)) / (columns - 1.0);
                            max_spacing.max(0.0).min(100.0)
                        } else {
                            50.0 // For list view, keep reasonable max
                        };
                        ui.add(egui::Slider::new(&mut self.horizontal_spacing, 0.0..=max_h_spacing).text("Horizontal Spacing"));
                        ui.add(egui::Slider::new(&mut self.vertical_spacing, 0.0..=50.0).text("Vertical Spacing"));
                        
                        ui.separator();
                        ui.heading("Visual Effects");
                        ui.separator();
                        
                        ui.checkbox(&mut self.show_scanlines, "CRT Scanlines");
                        
                        ui.separator();
                        ui.heading("File Options");
                        ui.separator();
                        
                        let hidden_label = if self.show_hidden { "Show Hidden Files" } else { "Hide Hidden Files" };
                        if ui.checkbox(&mut self.show_hidden, hidden_label).changed() {
                            self.read_dir();
                        }
                        
                        ui.separator();
                        if ui.button("Reset to Defaults").clicked() {
                            self.ui_scale = 1.0;
                            self.max_items_per_row = 3;
                            self.horizontal_spacing = 16.0;
                            self.vertical_spacing = 12.0;
                            self.show_scanlines = false;
                            self.show_hidden = false;
                            self.read_dir();
                        }
                    });
                });
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(bg))
            .show(ctx, |ui| {
            // Apply dynamic font size based on ui_scale
            let mut style = (*ui.ctx().style()).clone();
            style.override_font_id = Some(egui::FontId::monospace(16.0 * self.ui_scale));
            ui.ctx().set_style(style);

            if let Some(err) = &self.error {
                ui.colored_label(egui::Color32::RED, err);
            }

            let mut navigate_to_path: Option<PathBuf> = None;
            let base_green = egui::Color32::from_rgb(0, 255, 0);
            let hover_green = egui::Color32::from_rgb(120, 255, 120);
            let hover_stroke = egui::Stroke { width: 1.0 * self.ui_scale, color: hover_green };

            // Add margin around the entire content area
            let margin = 16.0 * self.ui_scale;
            ui.add_space(margin);

            if self.grid_view {
                // Grid view with neon bordered cards and metadata
                let card_height = 80.0 * self.ui_scale;
                let horizontal_spacing = self.horizontal_spacing * self.ui_scale;
                let vertical_spacing = self.vertical_spacing * self.ui_scale;
                let available_width = ui.available_width() - (margin * 2.0);
                let columns = self.max_items_per_row as usize;
                // Calculate item width based on available space and max items per row
                let desired_width = (available_width - (horizontal_spacing * (columns - 1) as f32)) / columns as f32;
                
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        egui::Grid::new("files_grid").num_columns(columns).spacing(egui::vec2(horizontal_spacing, vertical_spacing)).show(ui, |ui| {
                    for (i, entry) in self.entries.iter().enumerate() {
                        let name_plain = Self::entry_name(entry);
                        let (is_dir, size_opt, date_opt) = Self::entry_info(entry);

                        let (alloc_id, alloc_rect) = ui.allocate_space(egui::vec2(desired_width, card_height - margin));
                        // Adjust rect to add left margin and ensure right margin
                        let adjusted_rect = alloc_rect.translate(egui::vec2(margin, 0.0));
                        let response = ui
                            .interact(adjusted_rect, alloc_id, egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        // Sharp neon border and hover inner border
                        let round = 0.0;
                        ui.painter().rect_stroke(adjusted_rect, round, hover_stroke);
                        if response.hovered() {
                            ui.painter().rect_stroke(adjusted_rect.shrink(2.0), round, hover_stroke);
                            // Play hover sound only once per item
                            if self.last_hovered_item != Some(i) {
                                self.play_hover_sound();
                                self.last_hovered_item = Some(i);
                            }
                        } else if self.last_hovered_item == Some(i) {
                            self.last_hovered_item = None;
                        }

                        // Content positions (vertically centered name + vector icon)
                        let left = adjusted_rect.left() + 12.0 * self.ui_scale;
                        let center_y = adjusted_rect.center().y;

                        // Icon (custom PNG)
                        let icon_size = 28.0 * self.ui_scale;
                        if let Some(icon_texture) = if is_dir { &self.folder_icon } else { &self.file_icon } {
                            let icon_rect = egui::Rect::from_center_size(
                                egui::pos2(left + icon_size * 0.5, center_y),
                                egui::vec2(icon_size, icon_size)
                            );
                            ui.painter().image(icon_texture.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), base_green);
                        } else {
                            // Fallback to emoji if custom icon not loaded
                            let icon = if is_dir { "📁" } else { "📄" };
                            ui.painter().text(
                                egui::pos2(left.floor(), center_y.floor()),
                                egui::Align2::LEFT_CENTER,
                                icon,
                                egui::FontId::monospace(icon_size),
                                base_green,
                            );
                        }

                        // Name (no icon in text) with scrolling
                        let name_x = left + icon_size + 12.0 * self.ui_scale;
                        let name_width = adjusted_rect.right() - name_x - 8.0 * self.ui_scale;
                        let font_size = 15.0 * self.ui_scale;
                        let time = ui.ctx().input(|i| i.time) as f32;
                        let (display_name, _) = self.get_scrolling_text(&name_plain, name_width, font_size, i, response.hovered(), time);
                        let name_text = if is_dir { egui::RichText::new(display_name).strong().monospace() } else { egui::RichText::new(display_name).monospace() };
                        ui.painter().text(
                            egui::pos2(name_x.floor(), center_y.floor()),
                            egui::Align2::LEFT_CENTER,
                            name_text.text(),
                            egui::FontId::monospace(font_size),
                            base_green,
                        );

                        // Metadata line (date · size) with scrolling
                        let meta_y = (adjusted_rect.bottom() - 10.0 * self.ui_scale).floor();
                        let date_str = date_opt.map(Self::format_date).unwrap_or_default();
                        let size_str = size_opt.map(Self::format_size).unwrap_or_default();
                        let meta = if is_dir {
                            date_str
                        } else if !date_str.is_empty() && !size_str.is_empty() {
                            format!("{}  ·  {}", date_str, size_str)
                        } else {
                            format!("{}{}", date_str, size_str)
                        };
                        let meta_width = adjusted_rect.right() - name_x - 8.0 * self.ui_scale;
                        let meta_font_size = 11.0 * self.ui_scale;
                        let (display_meta, _) = self.get_scrolling_text(&meta, meta_width, meta_font_size, i, response.hovered(), time);
                        ui.painter().text(
                            egui::pos2(name_x.floor(), meta_y),
                            egui::Align2::LEFT_BOTTOM,
                            display_meta,
                            egui::FontId::monospace(meta_font_size),
                            base_green,
                        );

                        if response.clicked() {
                            self.play_click_sound();
                            if is_dir {
                                navigate_to_path = Some(entry.path());
                            } else {
                                self.open_file(&entry.path());
                            }
                        }

                        let last_col = (i + 1) % columns == 0;
                        if last_col { ui.end_row(); }
                    }
                    });
                });
                // Add bottom margin after grid scroll area
                ui.add_space(margin);
            } else {
                // List view with sharp bordered rows and vector icons
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                    // Add top margin for list view
                    ui.add_space(margin);
                    
                    let row_h = 56.0 * self.ui_scale;
                    let vertical_spacing = self.vertical_spacing * self.ui_scale;
                    let available_width = ui.available_width() - (margin * 2.0);
                    for (i, entry) in self.entries.iter().enumerate() {
                        let (is_dir, _size_opt, _date_opt) = Self::entry_info(entry);
                        let name_plain = Self::entry_name(entry);

                        // Add vertical margin between rows
                        if i > 0 { ui.add_space(vertical_spacing); }

                        let (row_id, row_rect) = ui.allocate_space(egui::vec2(available_width, row_h - margin));
                        // Adjust rect to add left margin
                        let adjusted_rect = row_rect.translate(egui::vec2(margin, 0.0));
                        let response = ui
                            .interact(adjusted_rect, row_id, egui::Sense::click())
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        // Sharp border
                        ui.painter().rect_stroke(adjusted_rect, 0.0, hover_stroke);
                        if response.hovered() {
                            ui.painter().rect_stroke(adjusted_rect.shrink(2.0), 0.0, hover_stroke);
                            // Play hover sound only once per item
                            if self.last_hovered_item != Some(i) {
                                self.play_hover_sound();
                                self.last_hovered_item = Some(i);
                            }
                        } else if self.last_hovered_item == Some(i) {
                            self.last_hovered_item = None;
                        }

                        // Icon and name centered vertically
                        let left = adjusted_rect.left() + 12.0 * self.ui_scale;
                        let cy = adjusted_rect.center().y;
                        let icon_size = 26.0 * self.ui_scale;
                        if let Some(icon_texture) = if is_dir { &self.folder_icon } else { &self.file_icon } {
                            let icon_rect = egui::Rect::from_center_size(
                                egui::pos2(left + icon_size * 0.5, cy),
                                egui::vec2(icon_size, icon_size)
                            );
                            ui.painter().image(icon_texture.id(), icon_rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), base_green);
                        } else {
                            // Fallback to emoji if custom icon not loaded
                            let icon = if is_dir { "📁" } else { "📄" };
                            ui.painter().text(
                                egui::pos2(left.floor(), cy.floor()),
                                egui::Align2::LEFT_CENTER,
                                icon,
                                egui::FontId::monospace(icon_size),
                                base_green,
                            );
                        }

                        let name_x = left + icon_size + 12.0 * self.ui_scale;
                        let name_width = adjusted_rect.right() - name_x - 8.0 * self.ui_scale;
                        let font_size = 15.0 * self.ui_scale;
                        let time = ui.ctx().input(|i| i.time) as f32;
                        let (display_name, _) = self.get_scrolling_text(&name_plain, name_width, font_size, i, response.hovered(), time);
                        let name_text = if is_dir { egui::RichText::new(display_name).strong().monospace() } else { egui::RichText::new(display_name).monospace() };
                        ui.painter().text(
                            egui::pos2(name_x.floor(), cy.floor()),
                            egui::Align2::LEFT_CENTER,
                            name_text.text(),
                            egui::FontId::monospace(font_size),
                            base_green,
                        );

                        // Add metadata (date · size) to list view with scrolling
                        let (is_dir, size_opt, date_opt) = Self::entry_info(entry);
                        let date_str = date_opt.map(Self::format_date).unwrap_or_default();
                        let size_str = size_opt.map(Self::format_size).unwrap_or_default();
                        let meta = if is_dir {
                            date_str
                        } else if !date_str.is_empty() && !size_str.is_empty() {
                            format!("{}  ·  {}", date_str, size_str)
                        } else {
                            format!("{}{}", date_str, size_str)
                        };
                        let meta_width = adjusted_rect.right() - name_x - 8.0 * self.ui_scale;
                        let meta_font_size = 11.0 * self.ui_scale;
                        let meta_y = cy + 12.0 * self.ui_scale;
                        let (display_meta, _) = self.get_scrolling_text(&meta, meta_width, meta_font_size, i, response.hovered(), time);
                        ui.painter().text(
                            egui::pos2(name_x.floor(), meta_y),
                            egui::Align2::LEFT_TOP,
                            display_meta,
                            egui::FontId::monospace(meta_font_size),
                            base_green,
                        );

                        if response.clicked() {
                            self.play_click_sound();
                            if is_dir {
                                navigate_to_path = Some(entry.path());
                            } else {
                                self.open_file(&entry.path());
                            }
                        }
                    }
                });
            }

            if let Some(path) = navigate_to_path { self.navigate_to(path, true); }

            // Optional CRT scanlines overlay (more transparent, thicker, animated downward, faux glow)
            if self.show_scanlines {
                ui.ctx().request_repaint();
                let rect = ui.max_rect();
                // Extremely transparent core line
                let line_color = egui::Color32::from_rgba_unmultiplied(0, 255, 0, 4);
                let spacing = (70.0 * self.ui_scale).max(10.0); // 500% more spacing
                let thickness = (2.5 * self.ui_scale).max(1.2);
                let time = ui.ctx().input(|i| i.time);
                let t = time as f32;
                let speed = 40.0 * self.ui_scale; // pixels per second
                let offset = (t * speed) % spacing;

                let mut y = rect.top() + offset;
                while y < rect.bottom() + spacing {
                    let y2 = (y + thickness).min(rect.bottom());

                    // Core bright band
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), y),
                            egui::pos2(rect.right(), y2),
                        ),
                        0.0,
                        line_color,
                    );

                    // Faux glow: draw softly expanded bands with decreasing alpha
                    let glow_layers = 4; // larger-radius glow
                    for i in 1..=glow_layers {
                        let spread = (i as f32) * (3.0 * self.ui_scale); // bigger radius
                        // Subtle but visible: 6,4,2,1
                        let alpha: u8 = match i { 1 => 6, 2 => 4, 3 => 2, _ => 1 };
                        let glow_color = egui::Color32::from_rgba_unmultiplied(0, 255, 0, alpha);
                        // Top halo
                        let gy1 = (y - spread).max(rect.top());
                        let gy2 = y.min(rect.bottom());
                        if gy1 < gy2 {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left(), gy1),
                                    egui::pos2(rect.right(), gy2),
                                ),
                                0.0,
                                glow_color,
                            );
                        }
                        // Bottom halo
                        let gy3 = y2.min(rect.bottom());
                        let gy4 = (y2 + spread).min(rect.bottom());
                        if gy3 < gy4 {
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(rect.left(), gy3),
                                    egui::pos2(rect.right(), gy4),
                                ),
                                0.0,
                                glow_color,
                            );
                        }
                    }
                    y += spacing;
                }
                // Add bottom margin for list view
                ui.add_space(margin);
                // Add bottom margin after list view scroll area
                ui.add_space(margin);
            }
        });
        
        // Auto-save settings when they change
        self.save_settings();
    }
}

fn apply_retro_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(0, 255, 0));
    style.override_font_id = Some(egui::FontId::monospace(16.0));
    // Slightly thicker visuals for a retro look
    style.spacing.item_spacing = egui::vec2(6.0, 6.0);
    style.spacing.button_padding = egui::vec2(8.0, 6.0);
    
    // Custom scrollbar styling - thin, half-transparent green
    // Note: Scrollbar styling is handled by the ScrollArea configuration
    
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let start_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Datara",
        native_options,
        Box::new(|cc| {
            apply_retro_style(&cc.egui_ctx);
            Ok(Box::new(DataraApp::new(std::env::current_dir().unwrap_or(start_dir))))
        }),
    )
}
