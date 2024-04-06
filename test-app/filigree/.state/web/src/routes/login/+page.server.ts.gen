import {
	getOauthEnabledFlag,
	handleLoginWithPasswordForm,
	handlePasswordlessLoginToken,
	requestPasswordlessLoginForm,
} from "filigree-web/auth/login.server";

export const actions = {
	login: handleLoginWithPasswordForm,
	passwordless: requestPasswordlessLoginForm,
};

const oauthEnabled = {
	github: getOauthEnabledFlag("OAUTH_GITHUB_CLIENT_ID"),
	twitter: getOauthEnabledFlag("OAUTH_TWITTER_CLIENT_ID"),
	google: getOauthEnabledFlag("OAUTH_GOOGLE_CLIENT_ID"),
};

export async function load(event) {
	// Handle passwordless login, if the token is present.
	const pwResult = await handlePasswordlessLoginToken(event);

	return {
		oauthEnabled,
		...pwResult,
	};
}
