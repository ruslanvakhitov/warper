use crate::{
    cloud_object::{
        model::persistence::CloudModel, CloudObjectEventEntrypoint, CloudObjectLocation, Space,
    },
    network::{NetworkStatus, NetworkStatusKind},
    server::{
        cloud_objects::{listener::Listener, update_manager::UpdateManager},
        ids::ClientId,
    },
    util::bindings::keybinding_name_to_display_string,
    workflows::workflow::Workflow,
    workspaces::{team::Team, user_workspaces::UserWorkspaces, workspace::Workspace},
};
use warpui::{async_assert, async_assert_eq, integration::TestStep, SingletonEntity};

fn set_and_assert_network_status(status: NetworkStatusKind) -> TestStep {
    TestStep::new("Set and assert network status")
        .with_action(move |app, _, _| {
            NetworkStatus::handle(app).update(app, |network_status, ctx| {
                if matches!(status, NetworkStatusKind::Online) {
                    network_status.reachability_changed(true, ctx);
                } else {
                    network_status.reachability_changed(false, ctx);
                }
            });
        })
        .add_assertion(move |app, _| {
            NetworkStatus::handle(app).read(app, |network_status, _| {
                async_assert!(
                    network_status.status() == status,
                    "network status is correct"
                )
            })
        })
}

pub fn go_offline() -> TestStep {
    set_and_assert_network_status(NetworkStatusKind::Offline)
}

pub fn go_online() -> TestStep {
    set_and_assert_network_status(NetworkStatusKind::Online)
}

pub fn join_a_workspace() -> TestStep {
    TestStep::new("Join a Warp Drive workspace")
        .with_action(move |app, _, _| {
            UserWorkspaces::handle(app).update(app, |user_workspaces, ctx| {
                let workspace_uid = "workspace_uid123456789".to_string().into();
                let teams: Vec<Team> = vec![Team {
                    uid: "team_uid12345678912345".try_into().expect("ID is valid"),
                    name: "My Team".to_string(),
                    organization_settings: Default::default(),
                }];
                let workspaces: Vec<Workspace> = vec![Workspace {
                    uid: workspace_uid,
                    name: "My Workspace".to_string(),
                    teams: teams.clone(),
                    settings: Default::default(),
                }];

                user_workspaces.update_workspaces(workspaces, ctx);
                user_workspaces.set_current_workspace_uid(workspace_uid, ctx)
            });
        })
        .add_assertion(move |app, _| {
            UserWorkspaces::handle(app).read(app, |user_workspaces, _| {
                async_assert!(user_workspaces.has_teams(), "user is on a team")
            })
        })
        .add_assertion(move |app, _| {
            UserWorkspaces::handle(app).read(app, |user_workspaces, _| {
                async_assert!(user_workspaces.has_workspaces(), "user is on a workspace")
            })
        })
}

pub fn create_a_personal_workflow() -> TestStep {
    TestStep::new("Create a personal workflow")
        .with_action(move |app, _, _| {
            UpdateManager::handle(app).update(app, |update_manager, ctx| {
                update_manager.create_workflow(
                    Workflow::new("My first workflow", "ls"),
                    UserWorkspaces::as_ref(ctx)
                        .personal_drive(ctx)
                        .expect("User UID must be set in tests"),
                    None,
                    ClientId::default(),
                    CloudObjectEventEntrypoint::ManagementUI,
                    true,
                    ctx,
                )
            })
        })
        .add_assertion(move |app, _| {
            CloudModel::handle(app).read(app, |cloud_model, ctx| {
                async_assert!(
                    cloud_model
                        .active_cloud_objects_in_location_without_descendents(
                            CloudObjectLocation::Space(Space::Personal),
                            ctx,
                        )
                        .count()
                        > 0,
                    "cloud objects exist"
                )
            })
        })
}

pub fn assert_binding_display_string(
    binding: &'static str,
    display_string: Option<&'static str>,
) -> TestStep {
    TestStep::new("Assert a binding's display string").add_named_assertion(
        format!("Binding {binding} should have display string {display_string:?}"),
        move |app, _| {
            app.update(|ctx| {
                async_assert_eq!(
                    keybinding_name_to_display_string(binding, ctx).as_deref(),
                    display_string
                )
            })
        },
    )
}

pub fn assert_websocket_has_started() -> TestStep {
    TestStep::new("Assert a websocket has started").add_named_assertion(
        "subscription abort handle should exist",
        move |app, _| {
            Listener::handle(app).read(app, |listener, _| {
                async_assert!(
                    listener.has_current_subscription_abort_handle(),
                    "subscription has started"
                )
            })
        },
    )
}

pub fn assert_websocket_has_not_started() -> TestStep {
    TestStep::new("Assert a websocket has not started").add_named_assertion(
        "subscription abort handle should not exist",
        move |app, _| {
            Listener::handle(app).read(app, |listener, _| {
                async_assert!(
                    !listener.has_current_subscription_abort_handle(),
                    "subscription has not started"
                )
            })
        },
    )
}
