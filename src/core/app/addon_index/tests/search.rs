use std::fs;

use tempfile::tempdir;

use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource,
};
use crate::core::app::{AddonIndexService, AppRuntime, SearchAddonIndexRequest};
use crate::core::error::AppResult;

#[test]
fn addon_index_service_search_projects_catalog_results() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("community-addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Community Catalog"

[[packages]]
id = "elvui"
name = "ElvUI"
version = "15.13"
source = { kind = "tukui_addon", slug = "elvui" }
source_url = "https://api.tukui.org/v1/addon/elvui"
website_url = "https://tukui.org/elvui"
addon_directories = ["ElvUI", "ElvUI_Libraries", "ElvUI_Options"]
supported_flavors = ["retail", "classic", "classic-era"]
"#,
    )
    .expect("index file");

    let service = AddonIndexService::with_runtime(AppRuntime::with_addon_provider(
        FakeIndexSearchAddonProvider,
    ));

    let result = service
        .search(SearchAddonIndexRequest {
            index_path: index_path.clone(),
            query: "Elv".to_string(),
            limit: 10,
        })
        .expect("search addon index");

    assert_eq!(result.index_path, index_path);
    assert_eq!(result.index_name, "Community Catalog");
    assert_eq!(result.query, "Elv");
    assert_eq!(result.package_count, 1);
    assert_eq!(result.matched_package_count, 1);
    assert_eq!(result.returned_package_count, 1);
    assert_eq!(result.packages.len(), 1);
    assert_eq!(result.packages[0].id, "elvui");
    assert_eq!(result.packages[0].source_label, "tukui:elvui");
}

#[derive(Clone)]
struct FakeIndexSearchAddonProvider;

impl AddonProvider for FakeIndexSearchAddonProvider {
    fn materialize_source_input(
        &self,
        _request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        panic!("materialize_source_input should not be called in this test")
    }

    fn materialize_source_ref(
        &self,
        _request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        panic!("materialize_source_ref should not be called in this test")
    }

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        panic!("search_addons should not be called in this test")
    }

    fn dependency_resolution_capability(
        &self,
        _source: &AddonSourceRef,
    ) -> crate::core::addon::AddonDependencyResolutionCapability {
        crate::core::addon::AddonDependencyResolutionCapability::Unsupported
    }
}
