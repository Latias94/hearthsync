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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::sample_installation;
    use super::*;
    use crate::core::app::{
        HealthStatusValue, InstallationHealthResult, InstallationInspectionResult,
        InstallationScanResult, WowFlavorValue,
    };

    #[test]
    fn render_installation_scan_lists_detected_installations() {
        let rendered = render_installation_scan(&InstallationScanResult {
            installation_count: 1,
            installations: vec![sample_installation()],
        });

        assert!(rendered.contains("Detected 1 installation(s):"));
        assert!(rendered.contains("- retail => C:\\Games\\World of Warcraft\\_retail_"));
    }

    #[test]
    fn render_installation_health_report_lists_missing_paths_and_warnings() {
        let rendered = render_installation_health_report(&InstallationHealthResult {
            status: HealthStatusValue::Warning,
            status_label: "warning".to_string(),
            missing_paths: vec![PathBuf::from("Fonts")],
            warnings: vec!["WTF folder is empty".to_string()],
        });

        assert!(rendered.contains("Status: warning"));
        assert!(rendered.contains("Missing required paths:"));
        assert!(rendered.contains("- Fonts"));
        assert!(rendered.contains("Warnings:"));
        assert!(rendered.contains("- WTF folder is empty"));
    }

    #[test]
    fn render_installation_inspection_reports_selected_flavor() {
        let rendered = render_installation_inspection(&InstallationInspectionResult {
            requested_path: PathBuf::from("C:\\Games\\World of Warcraft"),
            product_root: PathBuf::from("C:\\Games\\World of Warcraft"),
            available_flavors: vec![WowFlavorValue::Retail],
            installation: sample_installation(),
            health: InstallationHealthResult {
                status: HealthStatusValue::Healthy,
                status_label: "healthy".to_string(),
                missing_paths: Vec::new(),
                warnings: Vec::new(),
            },
        });

        assert!(rendered.contains("Flavor: retail"));
        assert!(rendered.contains("Product root: C:\\Games\\World of Warcraft"));
        assert!(rendered.contains("Health: healthy"));
    }
}
