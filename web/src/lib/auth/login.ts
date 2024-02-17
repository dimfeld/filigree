import type { FormResponse } from '$lib/forms.svelte.js';
import { env } from '$env/dynamic/private';
import {
  applyResponseCookies,
  forwardToApi,
  handleFormResponse,
  isExtractedResponse,
} from '$lib/requests.js';
import { error, redirect, type RequestEvent } from '@sveltejs/kit';
import { client } from '$lib/client.js';

export interface LoginFormResponse {
  email: string;
  password?: string;
}

export async function handleLoginWithPasswordForm(event: RequestEvent) {
  let response = await forwardToApi('POST', 'auth/login', event, { tolerateFailure: true });

  applyResponseCookies(response, event.cookies);

  if (response.ok) {
    redirect(301, '/');
  }

  const result = await handleFormResponse<LoginFormResponse>(response, [400, 401]);
  if (isExtractedResponse(result)) {
    // We probably should never hit this since we already checked `response.ok` above.
    return result.body;
  }

  if (result.data.form) {
    delete result.data.form.password;
  }
  return result;
}

export async function requestPasswordlessLoginForm(event: RequestEvent) {
  const res = await forwardToApi('POST', 'auth/email_login', event);
  if (!res.ok) {
    const data = await res.json();
    error(500, data);
  }

  return {
    form: {} as LoginFormResponse,
    message: 'You should receive an email soon.',
  } satisfies FormResponse<{ email: string; password?: string }>;
}

export interface PasswordlessLoginResult {
  message: string;
  redirect_to?: string;
}

export async function handlePasswordlessLoginToken({ fetch, url, cookies }: RequestEvent) {
  let token = url.searchParams.get('token');
  if (!token) {
    return null;
  }

  let res = await client({
    url: '/api/auth/email_login',
    method: 'GET',
    query: url.searchParams,
    fetch,
    tolerateFailure: true,
  });

  applyResponseCookies(res, cookies);
  if (res.ok) {
    let successBody = (await res.json()) as PasswordlessLoginResult;
    return {
      logInSuccess: true,
      ...successBody,
    };
  } else {
    let response = await res.json();
    let message: string = response.error?.message ?? 'An error occurred';
    return { logInSuccess: false, message };
  }
}

export function getOauthEnabledFlag(varName: string) {
  return env[varName] ? true : undefined;
}
