CREATE INDEX comments_updated_at ON comments (organization_id, updated_at DESC);

CREATE INDEX comments_created_at ON comments (organization_id, created_at DESC);

CREATE INDEX polls_updated_at ON polls (organization_id, updated_at DESC);

CREATE INDEX polls_created_at ON polls (organization_id, created_at DESC);

CREATE INDEX post_images_updated_at ON post_images (organization_id, updated_at DESC);

CREATE INDEX post_images_created_at ON post_images (organization_id, created_at DESC);

CREATE INDEX reactions_updated_at ON reactions (organization_id, updated_at DESC);

CREATE INDEX reactions_created_at ON reactions (organization_id, created_at DESC);

CREATE INDEX posts_updated_at ON posts (organization_id, updated_at DESC);

CREATE INDEX posts_created_at ON posts (organization_id, created_at DESC);

CREATE INDEX report_sections_updated_at ON report_sections (organization_id, updated_at DESC);

CREATE INDEX report_sections_created_at ON report_sections (organization_id, created_at DESC);

CREATE INDEX reports_updated_at ON reports (organization_id, updated_at DESC);

CREATE INDEX reports_created_at ON reports (organization_id, created_at DESC);
