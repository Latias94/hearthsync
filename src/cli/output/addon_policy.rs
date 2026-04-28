use crate::core::app::{AddonPolicyInspectionResult, AddonPolicyMutationResult};

use super::shared::format_string_list_or_none;

pub(in crate::cli) fn render_addon_policy_inspection(item: &AddonPolicyInspectionResult) -> String {
    if item.packages.is_empty() {
        return format!(
            "Policy file: {}\nPackages: 0\nEntries: none",
            item.policy_path.display()
        );
    }

    let packages = item
        .packages
        .iter()
        .map(|package| {
            let package_name = package.package_name.as_deref().unwrap_or("none");
            let addon_directories = format_string_list_or_none(&package.addon_directories);
            let pin = match &package.pin {
                Some(crate::core::app::AddonPolicyPinValue::Version { value }) => {
                    format!("version:{value}")
                }
                Some(crate::core::app::AddonPolicyPinValue::FileId { value }) => {
                    format!("file-id:{value}")
                }
                None => "none".to_string(),
            };
            let release_channel = package
                .release_channel
                .map(format_release_channel)
                .unwrap_or("none");
            format!(
                "- {} | name: {} | tracked: {} | addons: {} | ignored: {} | pin: {} | channel: {} | prerelease: {} | dependencies: {}",
                package.package_id,
                package_name,
                package.tracked,
                addon_directories,
                format_optional_bool(package.ignored),
                pin,
                release_channel,
                format_optional_bool(package.allow_prerelease),
                format_optional_bool(package.install_dependencies),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Policy file: {}\nPackages: {}\n{}",
        item.policy_path.display(),
        item.package_count,
        packages
    )
}

pub(in crate::cli) fn render_addon_policy_mutation(item: &AddonPolicyMutationResult) -> String {
    if item.entry_removed {
        format!(
            "Removed addon policy entry: {}\nPolicy file: {}\nRemaining entries: {}",
            item.package_id,
            item.policy_path.display(),
            item.package_count
        )
    } else {
        let package = item.package.as_ref().expect("addon policy package");
        let pin = match &package.pin {
            Some(crate::core::app::AddonPolicyPinValue::Version { value }) => {
                format!("version:{value}")
            }
            Some(crate::core::app::AddonPolicyPinValue::FileId { value }) => {
                format!("file-id:{value}")
            }
            None => "none".to_string(),
        };
        format!(
            "Updated addon policy entry: {}\nTracked: {}\nIgnored: {}\nPin: {}\nChannel: {}\nPrerelease: {}\nDependencies: {}\nPolicy file: {}\nTotal entries: {}",
            package.package_id,
            package.tracked,
            format_optional_bool(package.ignored),
            pin,
            package
                .release_channel
                .map(format_release_channel)
                .unwrap_or("none"),
            format_optional_bool(package.allow_prerelease),
            format_optional_bool(package.install_dependencies),
            item.policy_path.display(),
            item.package_count
        )
    }
}

fn format_optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "none",
    }
}

fn format_release_channel(value: crate::core::app::AddonReleaseChannelValue) -> &'static str {
    match value {
        crate::core::app::AddonReleaseChannelValue::Stable => "stable",
        crate::core::app::AddonReleaseChannelValue::Beta => "beta",
        crate::core::app::AddonReleaseChannelValue::Alpha => "alpha",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{render_addon_policy_inspection, render_addon_policy_mutation};
    use crate::core::app::{
        AddonPolicyInspectionResult, AddonPolicyMutationResult, AddonPolicyPackageResult,
        AddonPolicyPinValue, AddonReleaseChannelValue,
    };

    #[test]
    fn render_addon_policy_inspection_lists_entries() {
        let rendered = render_addon_policy_inspection(&AddonPolicyInspectionResult {
            policy_path: PathBuf::from("addon-policy.toml"),
            package_count: 1,
            packages: vec![AddonPolicyPackageResult {
                package_id: "weakauras".to_string(),
                package_name: Some("WeakAuras".to_string()),
                addon_directories: vec!["WeakAuras".to_string()],
                tracked: true,
                ignored: Some(true),
                pin: Some(AddonPolicyPinValue::Version {
                    value: "1.2.3".to_string(),
                }),
                release_channel: Some(AddonReleaseChannelValue::Beta),
                allow_prerelease: Some(true),
                install_dependencies: Some(false),
            }],
        });

        assert!(rendered.contains("Policy file: addon-policy.toml"));
        assert!(rendered.contains("weakauras | name: WeakAuras"));
        assert!(rendered.contains("pin: version:1.2.3"));
        assert!(rendered.contains("channel: beta"));
    }

    #[test]
    fn render_addon_policy_mutation_reports_removed_entry() {
        let rendered = render_addon_policy_mutation(&AddonPolicyMutationResult {
            policy_path: PathBuf::from("addon-policy.toml"),
            package_count: 0,
            package_id: "details".to_string(),
            entry_removed: true,
            package: None,
        });

        assert!(rendered.contains("Removed addon policy entry: details"));
        assert!(rendered.contains("Remaining entries: 0"));
    }
}
