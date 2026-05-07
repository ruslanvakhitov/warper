CREATE TABLE ambient_agent_panes (
    id INTEGER PRIMARY KEY REFERENCES pane_nodes(id) ON DELETE CASCADE,
    kind TEXT NOT NULL DEFAULT 'ambient_agent',
    uuid BLOB NOT NULL,
    task_id TEXT
);
