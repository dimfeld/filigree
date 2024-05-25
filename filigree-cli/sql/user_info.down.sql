{% if auth.builtin %} DROP TABLE user_invites;

DROP TABLE oauth_authorization_sessions;

DROP TABLE oauth_logins;

DROP TABLE email_logins;

DROP TABLE api_keys;

DROP TABLE organization_members;

DROP TABLE user_sessions;

{% endif %} DROP TABLE user_roles;
