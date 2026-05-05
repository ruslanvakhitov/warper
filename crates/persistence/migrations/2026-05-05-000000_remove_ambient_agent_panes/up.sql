DELETE FROM pane_leaves WHERE kind = 'ambient_agent';
DELETE FROM pane_nodes WHERE is_leaf = TRUE AND id NOT IN (SELECT pane_node_id FROM pane_leaves);
DROP TABLE IF EXISTS ambient_agent_panes;
