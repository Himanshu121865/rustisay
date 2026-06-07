use indicatif::{ProgressBar, ProgressStyle};

pub fn default_progress_bar(label: &str, n_items: usize) -> ProgressBar {
    let progress = ProgressBar::new(n_items as u64);
    let template = format!("[{{wide_bar}}] {}: {{pos}}/{{len}} ({{elapsed}}/{{duration}})", label);
    progress.set_style(ProgressStyle::default_bar().template(&template).unwrap());
    progress
}
