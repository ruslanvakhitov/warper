#[cfg(test)]
use warpui::App;

#[cfg(test)]
pub fn initialize_settings_for_tests(app: &mut App) {
    use crate::{
        search::command_search::settings::CommandSearchSettings,
        settings::{
            app_icon::AppIconSettings, init_and_register_user_preferences,
            manager::SettingsManager, AccessibilitySettings, AliasExpansionSettings,
            AppEditorSettings, BlockVisibilitySettings, CLIAgentSettings, ChangelogSettings,
            CodeSettings, DebugSettings, EmacsBindingsSettings, FontSettings, GPUSettings,
            InputModeSettings, InputSettings, LocalPreferencesSettings, NativePreferenceSettings,
            PaneSettings, SameLinePromptBlockSettings, ScrollSettings, SelectionSettings,
            SshSettings, ThemeSettings, VimBannerSettings,
        },
        terminal::{
            general_settings::GeneralSettings, keys_settings::KeysSettings,
            ligature_settings::LigatureSettings, safe_mode_settings::SafeModeSettings,
            session_settings::SessionSettings, settings::TerminalSettings,
            warpify::settings::WarpifySettings, BlockListSettings,
        },
        undo_close::UndoCloseSettings,
        user_config::WarpConfig,
        window_settings::WindowSettings,
        workspace::tab_settings::TabSettings,
    };
    use warp_core::semantic_selection::SemanticSelection;

    app.update(init_and_register_user_preferences);
    app.add_singleton_model(|_ctx| SettingsManager::default());
    app.add_singleton_model(WarpConfig::mock);

    AccessibilitySettings::register(app);
    app.update(CLIAgentSettings::register_and_subscribe_to_events);
    AliasExpansionSettings::register(app);
    AppEditorSettings::register(app);
    BlockVisibilitySettings::register(app);
    BlockListSettings::register(app);
    ChangelogSettings::register(app);
    LocalPreferencesSettings::register(app);
    CommandSearchSettings::register(app);
    DebugSettings::register(app);
    AppIconSettings::register(app);
    EmacsBindingsSettings::register(app);

    #[cfg(feature = "local_fs")]
    {
        crate::util::file::external_editor::EditorSettings::register(app);
    }

    FontSettings::register(app);
    GeneralSettings::register(app);
    GPUSettings::register(app);
    InputModeSettings::register(app);
    InputSettings::register(app);
    KeysSettings::register(app);
    LigatureSettings::register(app);

    #[cfg(target_os = "linux")]
    {
        use crate::settings::LinuxAppConfiguration;
        LinuxAppConfiguration::register(app);
    }

    NativePreferenceSettings::register(app);
    SafeModeSettings::register(app);
    SameLinePromptBlockSettings::register(app);
    ScrollSettings::register(app);
    SelectionSettings::register(app);
    app.update(|ctx| {
        WarpifySettings::register(ctx);
    });
    SessionSettings::register(app);
    SshSettings::register(app);
    TabSettings::register(app);
    TerminalSettings::register(app);
    PaneSettings::register(app);
    ThemeSettings::register(app);
    UndoCloseSettings::register(app);
    VimBannerSettings::register(app);
    WindowSettings::register(app);
    CodeSettings::register(app);
    SemanticSelection::register(app);
}
