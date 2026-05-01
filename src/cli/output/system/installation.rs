use crate::core::app::{
    InstallationHealthResult, InstallationInspectionResult, InstallationScanResult,
};

pub(in crate::cli) fn render_installation_scan(item: &InstallationScanResult) -> String {
    if item.installations.is_empty() {
        "No World of Warcraft installations detected.".to_string()
    } else {
        let mut lines = vec![format!(
            "Detected {} installation(s):",
            item.installation_count
        )];
        for installation in &item.installations {
            lines.push(format!(
                "- {} => {}",
                installation.flavor.as_str(),
                installation.flavor_root.display()
            ));
        }
        lines.join("\n")
    }
}

pub(in crate::cli) fn render_installation_inspection(
    item: &InstallationInspectionResult,
) -> String {
    format!(
        "Flavor: {}\nProduct root: {}\nFlavor root: {}\nAddOns: {}\nWTF: {}\nFonts: {}\nHealth: {}",
        item.installation.flavor.as_str(),
        item.product_root.display(),
        item.installation.flavor_root.display(),
        item.installation.addon_dir.display(),
        item.installation.wtf_dir.display(),
        item.installation.fonts_dir.display(),
        item.health.status_label
    )
}

pub(in crate::cli) fn render_installation_health_report(
    health: &InstallationHealthResult,
) -> String {
    let mut lines = vec![format!("Status: {}", health.status_label)];

    if health.missing_paths.is_empty() {
        lines.push("Missing required paths: none".to_string());
    } else {
        lines.push("Missing required paths:".to_string());
        for path in &health.missing_paths {
            lines.push(format!("- {}", path.display()));
        }
    }

    if health.warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push("Warnings:".to_string());
        for warning in &health.warnings {
            lines.push(format!("- {warning}"));
        }
    }

    lines.join("\n")
}
