import type { RequestHandler } from "@sveltejs/kit";
import { client } from "filigree-web";

export const GET: RequestHandler = async ({ url, params, fetch }) => {
	const provider = params.provider;
	// This returns a redirect to the provider's authorization URL
	return client({
		url: `/api/auth/oauth/login/${provider}`,
		fetch,
		query: url.searchParams,
	});
};
