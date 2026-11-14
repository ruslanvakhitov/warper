//! Persists terminal command blocks.

use std::collections::HashMap;

use diesel::{prelude::*, result::Error, sqlite::SqliteConnection};

use super::SerializedBlockListItem;
use crate::app_state::PaneUuid;
use crate::terminal::model::block::SerializedBlock;

use super::model::Block;
use super::{model, schema};

const MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION: i64 = 100;

type PersistedBlocks = HashMap<PaneUuid, Vec<SerializedBlockListItem>>;

/// Returns the most recent [`MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION`] block list items for each session. The
/// items are in chronological order.
pub(super) fn get_all_restored_blocks(
    conn: &mut SqliteConnection,
) -> Result<PersistedBlocks, diesel::result::Error> {
    let terminal_sessions = schema::terminal_panes::table
        .select(model::TerminalSession::as_select())
        .load::<model::TerminalSession>(conn)?;

    let block_lists = Block::belonging_to(&terminal_sessions)
        .select(Block::as_select())
        .order_by(schema::blocks::columns::id.asc())
        .load::<Block>(conn)?
        .grouped_by(&terminal_sessions);

    let mut all_block_items_by_pane = block_lists
        .into_iter()
        .zip(terminal_sessions)
        .map(|(blocks, terminal_pane)| {
            (
                PaneUuid(terminal_pane.uuid),
                blocks.into_iter().map(Into::into).collect(),
            )
        })
        .collect::<HashMap<_, Vec<SerializedBlockListItem>>>();

    for (_, blocks) in all_block_items_by_pane.iter_mut() {
        blocks.sort_by_key(|item| item.start_ts());
        // Only keep most recent command blocks
        blocks.drain(
            0..blocks
                .len()
                .saturating_sub(MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION as usize),
        );
    }

    Ok(all_block_items_by_pane)
}

pub(super) fn save_block(
    conn: &mut SqliteConnection,
    pane_id: Vec<u8>,
    block: &SerializedBlock,
    is_local_block: bool,
) -> Result<(), Error> {
    use schema::blocks::dsl::*;
    conn.transaction::<_, Error, _>(|conn| {
        let saved_blocks_count: i64 = schema::blocks::dsl::blocks
            .filter(pane_leaf_uuid.eq(pane_id.clone()))
            .filter(id.is_not_null())
            .filter(is_background.ne(true))
            .count()
            .first(conn)?;

        // add 1 because we are about to save a new block
        let diff = saved_blocks_count - MAX_TERMINAL_BLOCKS_TO_PERSIST_PER_SESSION + 1;
        if diff > 0 {
            // Find the oldest block to keep.
            let last_kept_id: Option<i32> = schema::blocks::dsl::blocks
                .filter(pane_leaf_uuid.eq(pane_id.clone()))
                .filter(id.is_not_null())
                .filter(is_background.ne(true))
                .select(id)
                .order(id.asc())
                .offset(diff)
                .limit(1)
                .first(conn)?;

            if let Some(last_kept_id) = last_kept_id {
                diesel::delete(
                    schema::blocks::dsl::blocks
                        .filter(id.lt(last_kept_id))
                        .filter(pane_leaf_uuid.eq(pane_id.clone())),
                )
                .execute(conn)?;
            }
        }

        let block = create_block(pane_id, block, is_local_block);
        diesel::insert_into(schema::blocks::dsl::blocks)
            .values(block)
            .execute(conn)?;
        Ok(())
    })
}

// TODO(vorporeal): can move this to a `to_persisted_block()` function on `SerializedBlock`
// to get it out of the persistence layer.
fn create_block<'a>(
    pane_leaf_uuid: Vec<u8>,
    block: &'a SerializedBlock,
    is_local: bool,
) -> model::NewBlock<'a> {
    model::NewBlock {
        block_id: block.id.as_str(),
        pane_leaf_uuid,
        stylized_command: &block.stylized_command,
        stylized_output: &block.stylized_output,
        pwd: block.pwd.as_ref(),
        // This sqlite column still uses the legacy `git_branch` name, but it now stores the
        // block's git head for backwards compatibility with existing persisted data.
        git_branch: block.git_head.as_ref(),
        git_branch_name: block.git_branch_name.as_ref(),
        virtual_env: block.virtual_env.as_ref(),
        conda_env: block.conda_env.as_ref(),
        exit_code: block.exit_code.value(),
        did_execute: block.did_execute,
        completed_ts: block.completed_ts.map(|ts| ts.naive_utc()),
        start_ts: block.start_ts.map(|ts| ts.naive_utc()),
        ps1: block.ps1.as_ref(),
        rprompt: block.rprompt.as_ref(),
        honor_ps1: block.honor_ps1,
        is_background: block.is_background,
        shell: block.shell_host.as_ref().map(|host| host.shell_type.name()),
        user: block.shell_host.as_ref().map(|host| host.user.as_str()),
        host: block.shell_host.as_ref().map(|host| host.hostname.as_str()),
        prompt_snapshot: block.prompt_snapshot.as_ref(),
        is_local: Some(is_local),
    }
}

pub(super) fn delete_blocks(conn: &mut SqliteConnection, pane_id: Vec<u8>) -> Result<(), Error> {
    use schema::blocks::dsl::*;
    conn.transaction::<_, Error, _>(|conn| {
        diesel::delete(schema::blocks::dsl::blocks.filter(pane_leaf_uuid.eq(pane_id.clone())))
            .execute(conn)?;
        Ok(())
    })
}
