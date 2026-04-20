use std::path::PathBuf;

use crate::cli::FlavorArg;
use crate::core::app::InspectInstallationRequest;

pub(super) fn build_inspect_installation_request(
    path: PathBuf,
    flavor: Option<FlavorArg>,
) -> InspectInstallationRequest {
    InspectInstallationRequest {
        path,
        flavor: flavor.map(Into::into),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::cli::FlavorArg;
    use crate::core::app::WowFlavorValue;

    #[test]
    fn build_inspect_installation_request_maps_flavor() {
        let request = build_inspect_installation_request(
            PathBuf::from("E:\\Games\\World of Warcraft"),
            Some(FlavorArg::Retail),
        );

        assert_eq!(request.path, PathBuf::from("E:\\Games\\World of Warcraft"));
        assert_eq!(request.flavor, Some(WowFlavorValue::Retail));
    }
}
