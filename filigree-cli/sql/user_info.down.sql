{% if auth.builtin %} DROP TABLE {{auth_schema}}.user_invites;

DROP TABLE {{auth_schema}}.oauth_authorization_sessions;

DROP TABLE {{auth_schema}}.oauth_logins;

DROP TABLE {{auth_schema}}.email_logins;

DROP TABLE {{auth_schema}}.api_keys;

DROP TABLE {{auth_schema}}.organization_members;

{% endif %} DROP TABLE {{auth_schema}}.user_sessions;

DROP TABLE {{auth_schema}}.user_roles;
