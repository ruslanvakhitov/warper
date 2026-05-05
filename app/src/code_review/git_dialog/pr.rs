//! Create-PR mode for [`GitDialog`].
//!
//! Renders the branch's PR diff (what would be included in the pull request)
//! with expandable per-file stats. On confirm, spawns `create_pr` and shows
//! a toast with a clickable "Open PR" link.

use warp_core::ui::appearance::Appearance;
use warpui::{
    elements::{
        ClippedScrollStateHandle, Container, Element, Flex, MouseStateHandle, ParentElement, Text,
    },
    SingletonEntity, ViewContext,
};

use crate::{
    code_review::git_dialog::{
        interactive_path_future, render_branch_section, render_file_changes_box, show_toast,
        user_facing_git_error, GitDialog, GitDialogAction, GitDialogEvent, GitDialogMode,
    },
    ui_components::icons::Icon,
    util::git::{
        create_pr, get_branch_commit_messages, get_branch_diff_entries, get_diff_for_pr,
        FileChangeEntry, PrInfo,
    },
    view_components::{DismissibleToast, ToastLink},
    workspace::ToastStack,
};

/// PR-mode sub-actions, dispatched wrapped in `GitDialogAction::Pr`.
#[derive(Clone, Debug, PartialEq)]
pub enum PrSubAction {
    ToggleChangesExpanded,
}

pub struct PrState {
    file_changes: Vec<FileChangeEntry>,
    changes_expanded: bool,
    summary_mouse_state: MouseStateHandle,
    changes_scroll_state: ClippedScrollStateHandle,
}

pub(super) fn confirm_label_for() -> &'static str {
    "Create PR"
}

pub(super) fn confirm_icon_for() -> Icon {
    Icon::Github
}

fn loading_label_for() -> &'static str {
    "Creating\u{2026}"
}

/// PR mode has no prerequisites beyond a branch with commits; confirm is
/// always enabled when not loading.
pub(super) fn is_ready_to_confirm(_state: &PrState) -> bool {
    true
}

pub(super) fn new_state(
    repo_path: &std::path::Path,
    parent_branch: Option<&str>,
    ctx: &mut ViewContext<GitDialog>,
) -> PrState {
    let diff_repo_path = repo_path.to_path_buf();
    let parent_branch = parent_branch.map(|s| s.to_string());
    ctx.spawn(
        async move { get_branch_diff_entries(&diff_repo_path, parent_branch.as_deref()).await },
        |me, result, ctx| {
            if let GitDialogMode::CreatePr(state) = &mut me.mode {
                match result {
                    Ok(entries) => {
                        state.file_changes = entries;
                        ctx.notify();
                    }
                    Err(err) => {
                        log::error!("Failed to load branch diff entries: {err}");
                    }
                }
            }
        },
    );

    PrState {
        file_changes: Vec::new(),
        changes_expanded: false,
        summary_mouse_state: MouseStateHandle::default(),
        changes_scroll_state: ClippedScrollStateHandle::default(),
    }
}

pub(super) fn handle_sub_action(
    me: &mut GitDialog,
    action: &PrSubAction,
    ctx: &mut ViewContext<GitDialog>,
) {
    match action {
        PrSubAction::ToggleChangesExpanded => {
            if let GitDialogMode::CreatePr(state) = me.mode_mut() {
                state.changes_expanded = !state.changes_expanded;
            }
            ctx.notify();
        }
    }
}

pub(super) fn start_confirm(me: &mut GitDialog, ctx: &mut ViewContext<GitDialog>) {
    let GitDialogMode::CreatePr(_) = me.mode() else {
        return;
    };
    let repo_path = me.repo_path().clone();
    let branch_name = me.branch_name().to_string();
    let parent_branch = me.parent_branch_name.clone();

    me.set_loading(loading_label_for(), ctx);

    let path_future = interactive_path_future(ctx);

    ctx.spawn(
        async move {
            let path_env = path_future.await;
            let _ = branch_name;
            create_pr(
                &repo_path,
                None,
                None,
                parent_branch.as_deref(),
                path_env.as_deref(),
            )
            .await
        },
        move |_me, result, ctx| {
            match result {
                Ok(pr_info) => {
                    show_pr_created_toast(&pr_info, ctx);
                }
                Err(err) => {
                    log::error!("Failed to create PR: {err}");
                    show_toast(user_facing_git_error(&err.to_string()), ctx);
                }
            }
            ctx.emit(GitDialogEvent::Completed);
        },
    );
}

/// Shows a toast announcing PR creation with a clickable "Open PR" link.
pub(super) fn show_pr_created_toast(pr_info: &PrInfo, ctx: &mut ViewContext<GitDialog>) {
    let window_id = ctx.window_id();
    let url = pr_info.url.clone();
    ToastStack::handle(ctx).update(ctx, |toast_stack, ctx| {
        let link = ToastLink::new("Open PR".to_string()).with_href(url);
        let toast =
            DismissibleToast::default("PR successfully created.".to_string()).with_link(link);
        toast_stack.add_ephemeral_toast(toast, window_id, ctx);
    });
}

pub(super) fn render_body(
    state: &PrState,
    branch_name: &str,
    appearance: &Appearance,
) -> Box<dyn Element> {
    Flex::column()
        .with_child(
            Container::new(render_branch_section(branch_name, appearance))
                .with_margin_bottom(16.)
                .finish(),
        )
        .with_child(render_changes_section(state, appearance))
        .finish()
}

fn render_changes_section(state: &PrState, appearance: &Appearance) -> Box<dyn Element> {
    let theme = appearance.theme();
    let main_color = theme.main_text_color(theme.surface_1()).into_solid();

    let label = Text::new(
        "Changes",
        appearance.ui_font_family(),
        appearance.ui_font_size(),
    )
    .with_color(main_color)
    .finish();

    let changes_box = render_file_changes_box(
        &state.file_changes,
        state.changes_expanded,
        &state.summary_mouse_state,
        &state.changes_scroll_state,
        GitDialogAction::Pr(PrSubAction::ToggleChangesExpanded),
        appearance,
    );

    Flex::column()
        .with_child(Container::new(label).with_margin_bottom(8.).finish())
        .with_child(changes_box)
        .finish()
}
