CREATE TABLE {{auth_schema}}.user_roles (
  organization_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  user_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  role_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.roles (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  PRIMARY KEY (organization_id, user_id, role_id)
);

CREATE TABLE {{auth_schema}}.user_sessions (
  id {{auth.id_sql_type}} PRIMARY KEY,
  user_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE,
  hash uuid NOT NULL{% endif %},
  expires_at timestamptz NOT NULL
);

{% if auth.builtin %}
-- A list of users and what organizations they belong to. Users can potentially be in more than one organization.
CREATE TABLE {{auth_schema}}.organization_members (
  organization_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  user_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  active boolean NOT NULL DEFAULT TRUE,
  PRIMARY KEY (organization_id, user_id)
);

CREATE INDEX user_sessions_user_id ON {{auth_schema}}.user_sessions (user_id);

CREATE TABLE {{auth_schema}}.api_keys (
  api_key_id uuid PRIMARY KEY,
  hash bytea NOT NULL,
  organization_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.organizations (id) ON DELETE CASCADE{% endif %},
  user_id {{auth.id_sql_type}} {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE{% endif %},
  inherits_user_permissions bool NOT NULL DEFAULT FALSE,
  description text NOT NULL DEFAULT '',
  active boolean NOT NULL DEFAULT TRUE,
  expires_at timestamptz NOT NULL
);

-- Methods for a user to log in.
CREATE TABLE {{auth_schema}}.email_logins (
  email text PRIMARY KEY,
  user_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  verified bool NOT NULL,
  reset_token uuid,
  reset_expires_at timestamptz,
  passwordless_login_token uuid,
  passwordless_login_expires_at timestamptz
);

CREATE INDEX email_logins_user_id ON {{auth_schema}}.email_logins (user_id);

CREATE TABLE {{auth_schema}}.oauth_logins (
  oauth_provider text NOT NULL,
  oauth_account_id text NOT NULL,
  user_id {{auth.id_sql_type}} NOT NULL {% if auth.builtin %}REFERENCES
    {{auth_schema}}.users (id) ON DELETE CASCADE DEFERRABLE INITIALLY
    IMMEDIATE{% endif %},
  PRIMARY KEY (oauth_provider, oauth_account_id)
);

CREATE INDEX oauth_logins_user_id ON {{auth_schema}}.oauth_logins (user_id);

CREATE TABLE {{auth_schema}}.oauth_authorization_sessions (
  key text PRIMARY KEY,
  provider text NOT NULL,
  pkce_verifier text,
  add_to_user_id {{auth.id_sql_type}},
  redirect_to text,
  expires_at timestamptz NOT NULL
);

CREATE TABLE {{auth_schema}}.user_invites (
  email text NOT NULL,
  token uuid NOT NULL,
  token_expires_at timestamptz NOT NULL,
  -- The person's name, if known.
  name text,
  -- The user that sent the invite
  invited_by {{auth.id_sql_type}},
  -- The organization that the user will be added to. NULL indicates a new organization.
  organization_id {{auth.id_sql_type}},
  -- The roles that the user will be added with, if inviting to an existing organization.
  -- If omitted, the organization's default role will be used.
  role_ids {{auth.id_sql_type}}[],
  invite_sent_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX user_invites_email_org ON {{auth_schema}}.user_invites (email,
  organization_id) NULLS NOT DISTINCT;

{% endif %}
