CREATE TABLE user_roles (
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  role_id uuid NOT NULL REFERENCES roles (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (organization_id, user_id, role_id)
);

CREATE TABLE user_sessions (
  session_id uuid PRIMARY KEY,
  hash bytea NOT NULL,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now(),
  expires_at timestamptz
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
  prefix text NOT NULL,
  hash bytea NOT NULL,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  user_id uuid REFERENCES users (id) ON DELETE SET NULL,
  inherits_user_permissions bool NOT NULL DEFAULT FALSE,
  description text,
  active boolean NOT NULL DEFAULT TRUE,
  expires_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Methods for a user to log in.
CREATE TABLE email_logins (
  email text PRIMARY KEY,
  password text,
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  reset_token text,
  reset_expires_at timestamptz,
  verify_token text,
  verify_expires_at timestamptz
);

CREATE INDEX ON email_logins (user_id);

CREATE TABLE oauth_logins (
  user_id uuid NOT NULL REFERENCES users (id) ON DELETE CASCADE DEFERRABLE INITIALLY IMMEDIATE,
  oauth_provider text NOT NULL,
  oauth_account_id text NOT NULL,
  PRIMARY KEY (user_id, oauth_provider, oauth_account_id)
);

CREATE INDEX ON oauth_logins (user_id);