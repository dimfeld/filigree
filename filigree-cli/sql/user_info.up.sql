CREATE TABLE user_roles (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  role_id uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  PRIMARY KEY (organization_id, user_id, role_id)
);

CREATE TABLE user_sessions (
  id uuid PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  hash uuid NOT NULL,
  expires_at timestamptz NOT NULL
);

-- A list of users and what organizations they belong to. Users can potentially be in more than one organization.
CREATE TABLE organization_members (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  active boolean NOT NULL DEFAULT TRUE,
  PRIMARY KEY (organization_id, user_id)
);

CREATE INDEX ON user_sessions (user_id);

CREATE TABLE api_keys (
  api_key_id uuid PRIMARY KEY,
  hash bytea NOT NULL,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  user_id uuid REFERENCES users (id) ON DELETE CASCADE,
  inherits_user_permissions bool NOT NULL DEFAULT FALSE,
  description text NOT NULL DEFAULT '',
  active boolean NOT NULL DEFAULT TRUE,
  expires_at timestamptz NOT NULL
);

-- Methods for a user to log in.
CREATE TABLE email_logins (
  email text PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  verified bool NOT NULL,
  reset_token uuid,
  reset_expires_at timestamptz,
  verify_token uuid,
  verify_expires_at timestamptz,
  passwordless_login_token uuid,
  passwordless_login_expires_at timestamptz
);

CREATE INDEX ON email_logins (user_id);

CREATE TABLE oauth_logins (
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  oauth_provider text NOT NULL,
  oauth_account_id text NOT NULL,
  PRIMARY KEY (user_id, oauth_provider, oauth_account_id)
);

CREATE INDEX ON oauth_logins (user_id);

CREATE TABLE user_invites (
  email text NOT NULL,
  token uuid NOT NULL,
  token_expires_at timestamptz NOT NULL,
  -- The person's name, if known.
  name text,
  -- The user that sent the invite
  invited_by uuid,
  -- The organization that the user will be added to. NULL indicates a new organization.
  organization_id uuid,
  invite_sent_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ON user_invites (email, organization_id) NULLS NOT DISTINCT;
