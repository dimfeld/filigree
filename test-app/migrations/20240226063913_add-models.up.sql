CREATE TABLE posts (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  subject text NOT NULL,
  body text NOT NULL
);

CREATE INDEX posts_organization_id ON posts (organization_id);

CREATE TABLE comments (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  body text NOT NULL,
  post_id uuid NOT NULL REFERENCES posts (id) ON DELETE CASCADE
);

CREATE INDEX comments_organization_id ON comments (organization_id);

CREATE INDEX comments_post_id ON comments (post_id);

CREATE TABLE polls (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  question text NOT NULL,
  answers jsonb NOT NULL,
  post_id uuid UNIQUE NOT NULL REFERENCES posts (id) ON DELETE CASCADE
);

CREATE INDEX polls_organization_id ON polls (organization_id);

CREATE INDEX polls_post_id ON polls (post_id);

CREATE TABLE reactions (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  type TEXT NOT NULL,
  post_id uuid NOT NULL REFERENCES posts (id) ON DELETE CASCADE
);

CREATE INDEX reactions_organization_id ON reactions (organization_id);

CREATE INDEX reactions_post_id ON reactions (post_id);

CREATE TABLE report_sections (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  name text NOT NULL,
  viz text NOT NULL,
  options jsonb NOT NULL,
  report_id uuid NOT NULL REFERENCES reports (id) ON DELETE CASCADE
);

CREATE INDEX report_sections_organization_id ON report_sections (organization_id);

CREATE INDEX report_sections_report_id ON report_sections (report_id);
