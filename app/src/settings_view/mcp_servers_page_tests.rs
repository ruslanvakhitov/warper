use super::MCPServersSettingsPageView;

#[test]
fn install_shows_modal_only_for_variables_or_instructions() {
    assert!(!MCPServersSettingsPageView::should_show_install_modal(
        false, false,
    ));
    assert!(MCPServersSettingsPageView::should_show_install_modal(
        true, false,
    ));
    assert!(MCPServersSettingsPageView::should_show_install_modal(
        false, true,
    ));
    assert!(MCPServersSettingsPageView::should_show_install_modal(
        true, true,
    ));
}
