#[derive(Debug, Clone)]
pub(in crate::core::bundle::external_package) struct SourceEntry {
    pub(in crate::core::bundle::external_package) source_path: String,
    pub(in crate::core::bundle::external_package) segments: Vec<String>,
}
