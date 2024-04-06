import { logout } from "filigree-web";

export async function load(event) {
	await logout(event);
	return {};
}
