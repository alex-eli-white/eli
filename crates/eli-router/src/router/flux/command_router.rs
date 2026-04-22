use crate::router::registries::reg_vanilla::ControlLease;

pub fn can_issue_control(current: Option<&ControlLease>, controller_id: &str) -> bool {
    match current {
        Some(lease) => lease.controller_id == controller_id,
        None => false,
    }
}
