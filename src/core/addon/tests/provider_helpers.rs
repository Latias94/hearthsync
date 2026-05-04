use tempfile::tempdir;

use super::{create_addon_archive, create_fixture_installation};
use crate::core::addon::package_prep::prepare_package_from_source_input_with_provider;
use crate::core::addon::provider::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource,
};
use crate::core::addon::{SearchAddonRequest, search_addons_with_provider};
use crate::core::error::AppResult;
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::NeverCancel;

#[test]
fn search_addons_can_use_fake_provider() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
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
            request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            assert_eq!(request.query, "WeakAuras");
            assert_eq!(request.flavor, WowFlavor::Retail);
            assert_eq!(request.limit, 5);
            assert_eq!(request.provider_id, None);
            Ok(vec![AddonSearchResult {
                provider: "fake",
                name: "WeakAuras".to_string(),
                summary: Some("fixture result".to_string()),
                source: AddonSourceRef::HttpArchive {
                    url: "https://example.invalid/weakauras.zip".to_string(),
                },
                install_hint: "https://example.invalid/weakauras.zip".to_string(),
                website_url: Some("https://example.invalid/weakauras".to_string()),
                provider_project_id: Some(42),
                provider_file_id: Some(84),
                download_count: 7,
            }])
        }
    }

    let installation = create_fixture_installation(tempdir().expect("temp dir").path());
    let catalog = search_addons_with_provider(
        &FakeProvider,
        SearchAddonRequest {
            installation,
            query: "WeakAuras".to_string(),
            limit: 5,
            provider_id: None,
        },
    )
    .expect("search through fake provider");

    assert_eq!(catalog.query, "WeakAuras");
    assert_eq!(catalog.results.len(), 1);
    assert_eq!(catalog.results[0].provider, "fake");
    assert!(catalog.failures.is_empty());
}

#[test]
fn search_addons_forwards_provider_scope_to_provider() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
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
            request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            assert_eq!(request.provider_id, Some("fake"));
            Ok(Vec::new())
        }
    }

    let installation = create_fixture_installation(tempdir().expect("temp dir").path());
    let catalog = search_addons_with_provider(
        &FakeProvider,
        SearchAddonRequest {
            installation,
            query: "WeakAuras".to_string(),
            limit: 5,
            provider_id: Some("fake".to_string()),
        },
    )
    .expect("search through fake provider");

    assert_eq!(catalog.provider_id.as_deref(), Some("fake"));
}

#[test]
fn prepare_package_from_source_input_can_use_fake_provider() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            assert_eq!(request.source, "fake:bundle");
            assert_eq!(request.context.target_flavor, Some(WowFlavor::Retail));
            assert!(request.context.cancellation.is_some());
            let archive_path = request.stage_root.join("fake-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::HttpArchive {
                    url: "https://example.invalid/fake-addon.zip".to_string(),
                },
                archive_path,
            })
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
    }

    let cancellation = NeverCancel;
    let prepared = prepare_package_from_source_input_with_provider(
        &FakeProvider,
        "fake:bundle",
        Some(WowFlavor::Retail),
        HostPlatform::Windows,
        &cancellation,
    )
    .expect("prepare package");

    assert_eq!(prepared.package_id, "fake-addon");
    assert_eq!(prepared.addons.len(), 1);
    assert_eq!(prepared.addons[0].addon.directory_name, "WeakAuras");
    assert_eq!(
        prepared.source,
        AddonSourceRef::HttpArchive {
            url: "https://example.invalid/fake-addon.zip".to_string(),
        }
    );
}
