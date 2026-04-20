pub(super) mod addon;
pub(super) mod addon_index;
pub(super) mod addon_lock;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod external_package;
pub(super) mod installation;

pub(super) fn map_domain_vec<TDomain, TResult, FConvert>(
    values: Vec<TDomain>,
    convert: FConvert,
) -> Vec<TResult>
where
    FConvert: FnMut(TDomain) -> TResult,
{
    values.into_iter().map(convert).collect()
}
