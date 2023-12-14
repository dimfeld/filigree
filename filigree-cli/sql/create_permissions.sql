CREATE TABLE permissions (
  team_id uuid NOT NULL REFERENCES teams (id) ON DELETE CASCADE,
  actor_id uuid NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (team_id, actor_id, permission)
);
