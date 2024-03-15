CREATE TABLE post_images (
  id uuid NOT NULL PRIMARY KEY,
  organization_id uuid NOT NULL REFERENCES organizations (id) ON DELETE CASCADE,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  file_storage_key text,
  file_storage_bucket text NOT NULL,
  file_original_name text,
  file_size integer,
  file_hash bytea,
  post_id uuid UNIQUE NOT NULL REFERENCES posts (id) ON DELETE CASCADE
);

CREATE INDEX post_images_organization_id ON post_images (organization_id);

CREATE INDEX post_images_post_id ON post_images (post_id);
