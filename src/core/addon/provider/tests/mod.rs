use std::path::Path;

use super::http::HttpClient;
use super::{AddonProvider, AddonProviderContext, AddonSourceRef, DefaultAddonProvider};

mod curseforge_cache;
mod github_cache;
mod http_cache;

fn materialize_source_twice<H>(
    provider: &DefaultAddonProvider<H>,
    source: &AddonSourceRef,
    stage_root: &Path,
) -> (
    super::MaterializedAddonSource,
    super::MaterializedAddonSource,
)
where
    H: HttpClient,
{
    (
        materialize_source(provider, source, stage_root, "stage-a"),
        materialize_source(provider, source, stage_root, "stage-b"),
    )
}

fn materialize_source<H>(
    provider: &DefaultAddonProvider<H>,
    source: &AddonSourceRef,
    stage_root: &Path,
    stage_name: &str,
) -> super::MaterializedAddonSource
where
    H: HttpClient,
{
    let stage_root = stage_root.join(stage_name);
    provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source,
            stage_root: &stage_root,
            context: AddonProviderContext::default(),
        })
        .unwrap_or_else(|error| panic!("materialize {stage_name}: {error}"))
}
