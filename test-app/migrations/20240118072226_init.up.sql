CREATE TABLE delete_log (
  organization_id uuid NOT NULL,
  object_id uuid NOT NULL,
  object_type text NOT NULL,
  data jsonb NOT NULL,
  deleted_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  name text NOT NULL,
  password_hash text,
  email text UNIQUE NOT NULL
);

CREATE TABLE organizations (
  id uuid NOT NULL PRIMARY KEY,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  name text NOT NULL,
  owner UUID,
  active boolean NOT NULL DEFAULT TRUE
);

CREATE TABLE roles (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  name text NOT NULL,
  description text
);

CREATE TABLE reports (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  title text NOT NULL,
  description text,
  ui jsonb NOT NULL
);

CREATE TABLE user_roles (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  role_id uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  PRIMARY KEY (organization_id, user_id, role_id)
);

CREATE TABLE user_sessions (
  id uuid PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  hash UUID NOT NULL,
  expires_at timestamptz NOT NULL
);

CREATE TABLE organization_members (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  active boolean NOT NULL DEFAULT TRUE,
  PRIMARY KEY (organization_id, user_id)
);

CREATE INDEX user_sessions_user_id ON user_sessions (user_id);

CREATE TABLE api_keys (
  api_key_id uuid PRIMARY KEY,
  hash BYTEA NOT NULL,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  user_id uuid REFERENCES users (id) ON DELETE CASCADE,
  inherits_user_permissions bool NOT NULL DEFAULT FALSE,
  description text NOT NULL DEFAULT '',
  active boolean NOT NULL DEFAULT TRUE,
  expires_at timestamptz NOT NULL
);

CREATE TABLE email_logins (
  email text PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  verified bool NOT NULL,
  reset_token uuid,
  reset_expires_at timestamptz,
  passwordless_login_token uuid,
  passwordless_login_expires_at timestamptz
);

CREATE INDEX email_logins_user_id ON email_logins (user_id);

CREATE TABLE oauth_logins (
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  oauth_provider text NOT NULL,
  oauth_account_id text NOT NULL,
  PRIMARY KEY (user_id, oauth_provider, oauth_account_id)
);

CREATE INDEX oauth_logins_user_id ON oauth_logins (user_id);

CREATE TABLE user_invites (
  email text NOT NULL,
  token uuid NOT NULL,
  token_expires_at timestamptz NOT NULL,
  name text,
  invited_by uuid,
  organization_id uuid,
  invite_sent_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX user_invites_email_org ON user_invites (email, organization_id) NULLS NOT DISTINCT;

CREATE TABLE permissions (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  actor_id uuid NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (organization_id, actor_id, permission)
);

CREATE TABLE object_permissions (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  actor_id uuid NOT NULL,
  object_id uuid NOT NULL,
  permission text NOT NULL,
  PRIMARY KEY (organization_id, actor_id, object_id, permission)
);