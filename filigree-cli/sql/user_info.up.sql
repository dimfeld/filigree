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

CREATE INDEX user_sessions_user_id ON user_sessions (user_id);

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
  passwordless_login_token uuid,
  passwordless_login_expires_at timestamptz
);

CREATE INDEX email_logins_user_id ON email_logins (user_id);

CREATE TABLE oauth_logins (
  oauth_provider text NOT NULL,
  oauth_account_id text NOT NULL,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  PRIMARY KEY (oauth_provider, oauth_account_id)
);

CREATE INDEX oauth_logins_user_id ON oauth_logins (user_id);

CREATE TABLE oauth_authorization_sessions (
  key text PRIMARY KEY,
  provider text NOT NULL,
  pkce_verifier text,
  add_to_user_id uuid,
  redirect_to text,
  expires_at timestamptz NOT NULL
);

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
  -- The roles that the user will be added with, if inviting to an existing organization.
  -- If omitted, the organization's default role will be used.
  role_ids uuid[],
  invite_sent_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX user_invites_email_org ON user_invites (email, organization_id) NULLS NOT DISTINCT;
