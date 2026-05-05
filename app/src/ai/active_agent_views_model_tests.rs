use warpui::{App, EntityId, WindowId};

use super::*;

fn setup_model(app: &mut App) -> ModelHandle<ActiveAgentViewsModel> {
    app.add_singleton_model(|_| ActiveAgentViewsModel::new())
}

#[test]
fn clearing_one_window_does_not_affect_other() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();
        let conversation_a = AIConversationId::new();
        let conversation_b = AIConversationId::new();

        model.update(&mut app, |model, _| {
            model.focused_terminal_states.insert(
                window_a,
                FocusedTerminalState {
                    focused_terminal_id: terminal_a,
                    active_conversation_id: Some(conversation_a),
                },
            );
            model.focused_terminal_states.insert(
                window_b,
                FocusedTerminalState {
                    focused_terminal_id: terminal_b,
                    active_conversation_id: Some(conversation_b),
                },
            );
        });

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, None, ctx);
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window_a), None);
            assert_eq!(
                model.get_focused_conversation(window_b),
                Some(conversation_b)
            );
        });
    });
}

#[test]
fn last_focused_terminal_tracks_most_recent_globally() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window_a = WindowId::new();
        let window_b = WindowId::new();
        let terminal_a = EntityId::new();
        let terminal_b = EntityId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_a, Some(terminal_a), ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal_a));
        });

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window_b, Some(terminal_b), ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.get_last_focused_terminal_id(), Some(terminal_b));
        });
    });
}

#[test]
fn focus_change_without_agent_view_has_no_conversation() {
    App::test((), |mut app| async move {
        let model = setup_model(&mut app);
        let window = WindowId::new();
        let terminal = EntityId::new();

        model.update(&mut app, |model, ctx| {
            model.handle_pane_focus_change(window, Some(terminal), ctx);
        });

        model.read(&app, |model, _| {
            assert_eq!(model.get_focused_conversation(window), None);
        });
    });
}
