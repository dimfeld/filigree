CREATE OR REPLACE FUNCTION objectid_to_uuid (text)
  RETURNS uuid
  LANGUAGE sql
  IMMUTABLE
  RETURNS NULL ON NULL INPUT PARALLEL SAFE
  AS $$
  SELECT
    encode(decode(replace(replace(
	  RIGHT ($1, 22), '-', '+'), '_', '/') ||
	    '==', 'base64'), 'hex')::uuid
$$;

CREATE OR REPLACE FUNCTION uuid_to_objectid (uuid)
  RETURNS text
  LANGUAGE sql
  IMMUTABLE
  RETURNS NULL ON NULL INPUT PARALLEL SAFE
  AS $$
  SELECT
    replace(replace(
      LEFT (encode(decode(replace($1::text, '-', ''), 'hex'),
	'base64'), 22), '+', '-'), '/', '_')
$$;
