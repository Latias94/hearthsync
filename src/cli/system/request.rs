use crate::cli::InstallTargetArgs;
use crate::core::app::InspectInstallationRequest;

pub(super) fn build_inspect_installation_request(
    install_target: InstallTargetArgs,
) -> InspectInstallationRequest {
    InspectInstallationRequest {
        path: install_target.install,
        flavor: install_target.flavor.map(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::{FlavorArg, InstallTargetArgs};
    use crate::core::app::WowFlavorValue;

    #[test]
    fn build_inspect_installation_request_maps_flavor() {
        let request = build_inspect_installation_request(InstallTargetArgs {
            install: PathBuf::from("E:\\Games\\World of Warcraft"),
            flavor: Some(FlavorArg::Retail),
        });

        assert_eq!(request.path, PathBuf::from("E:\\Games\\World of Warcraft"));
        assert_eq!(request.flavor, Some(WowFlavorValue::Retail));
    }
}
