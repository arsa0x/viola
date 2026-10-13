use crate::Mode;

/// Returns whether a command may execute in the current chat.
///
/// Global mode restrictions and per-command restrictions are both enforced.
/// Missing sender identity is never treated as an owner.
pub fn can_execute_command(
    mode: Mode,
    owners: &[String],
    sender_id: Option<&str>,
    is_group: bool,
    owner_only: bool,
    group_only: bool,
) -> bool {
    let is_owner = sender_id.is_some_and(|sender| owners.iter().any(|owner| owner == sender));

    if matches!(mode, Mode::Group) && !is_group {
        return false;
    }

    if matches!(mode, Mode::Owner) && !is_owner {
        return false;
    }

    if group_only && !is_group {
        return false;
    }

    if owner_only && !is_owner {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::can_execute_command;
    use crate::Mode;

    fn owners() -> Vec<String> {
        vec!["628123456789".to_owned()]
    }

    #[test]
    fn public_mode_allows_public_command_in_private_chat() {
        assert!(can_execute_command(
            Mode::Public,
            &owners(),
            Some("628999999999"),
            false,
            false,
            false,
        ));
    }

    #[test]
    fn group_mode_rejects_private_chat() {
        assert!(!can_execute_command(
            Mode::Group,
            &owners(),
            Some("628123456789"),
            false,
            false,
            false,
        ));
    }

    #[test]
    fn group_only_command_rejects_private_chat() {
        assert!(!can_execute_command(
            Mode::Public,
            &owners(),
            Some("628123456789"),
            false,
            false,
            true,
        ));
    }

    #[test]
    fn group_only_command_allows_group_chat() {
        assert!(can_execute_command(
            Mode::Public,
            &owners(),
            Some("628999999999"),
            true,
            false,
            true,
        ));
    }

    #[test]
    fn owner_only_command_rejects_non_owner() {
        assert!(!can_execute_command(
            Mode::Public,
            &owners(),
            Some("628999999999"),
            true,
            true,
            false,
        ));
    }

    #[test]
    fn owner_only_command_rejects_missing_identity() {
        assert!(!can_execute_command(
            Mode::Public,
            &owners(),
            None,
            true,
            true,
            false,
        ));
    }

    #[test]
    fn owner_only_command_allows_configured_owner() {
        assert!(can_execute_command(
            Mode::Public,
            &owners(),
            Some("628123456789"),
            true,
            true,
            false,
        ));
    }

    #[test]
    fn owner_mode_rejects_missing_identity() {
        // Regression test: a missing sender must not bypass owner mode.
        assert!(!can_execute_command(
            Mode::Owner,
            &owners(),
            None,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn owner_mode_rejects_unknown_sender() {
        assert!(!can_execute_command(
            Mode::Owner,
            &owners(),
            Some("628999999999"),
            false,
            false,
            false,
        ));
    }

    #[test]
    fn owner_mode_allows_configured_owner() {
        assert!(can_execute_command(
            Mode::Owner,
            &owners(),
            Some("628123456789"),
            false,
            false,
            false,
        ));
    }

    #[test]
    fn owner_mode_with_empty_owner_list_denies_everyone() {
        assert!(!can_execute_command(
            Mode::Owner,
            &[],
            Some("628123456789"),
            false,
            false,
            false,
        ));
    }
}
