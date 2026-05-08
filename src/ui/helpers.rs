use eframe::egui;

pub fn color_for_size(size: usize) -> egui::Color32 {
    match size {
        512 => egui::Color32::from_rgb(255, 87, 51), // Bright Red
        1024 => egui::Color32::from_rgb(255, 152, 0), // Bright Orange
        2048 => egui::Color32::from_rgb(255, 235, 59), // Bright Yellow
        4096 => egui::Color32::from_rgb(76, 175, 80), // Bright Green
        8192 => egui::Color32::from_rgb(33, 150, 243), // Bright Blue
        16384 => egui::Color32::from_rgb(156, 39, 176), // Bright Purple
        32768 => egui::Color32::from_rgb(255, 64, 129), // Bright Pink
        65536 => egui::Color32::from_rgb(0, 188, 212), // Bright Cyan
        131072 => egui::Color32::from_rgb(255, 87, 187), // Bright Magenta
        _ => egui::Color32::from_rgb(96, 96, 96),    // Dark Gray fallback
    }
}

pub fn get_size_label(size: usize) -> String {
    if size >= 1024 {
        format!("{} KB", size / 1024)
    } else {
        format!("{size} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_size_label_formats_bytes_under_1k() {
        assert_eq!(get_size_label(0), "0 B");
        assert_eq!(get_size_label(512), "512 B");
    }

    #[test]
    fn get_size_label_formats_kb_as_integer_division() {
        assert_eq!(get_size_label(1024), "1 KB");
        assert_eq!(get_size_label(1536), "1 KB");
        assert_eq!(get_size_label(2048), "2 KB");
    }
}
