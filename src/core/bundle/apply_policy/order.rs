use super::super::types::apply::{ApplyAction, ApplyGroup};

pub(in crate::core::bundle) fn apply_action_order(action: ApplyAction) -> u8 {
    match action {
        ApplyAction::Remove => 0,
        ApplyAction::Add => 1,
        ApplyAction::Replace => 2,
        ApplyAction::Skip => 3,
        ApplyAction::Preserve => 4,
    }
}

pub(in crate::core::bundle) fn apply_group_order(group: ApplyGroup) -> u8 {
    match group {
        ApplyGroup::Addons => 0,
        ApplyGroup::InterfaceAssets => 1,
        ApplyGroup::Fonts => 2,
        ApplyGroup::WtfCommon => 3,
        ApplyGroup::WtfCharacters => 4,
        ApplyGroup::Metadata => 5,
    }
}
