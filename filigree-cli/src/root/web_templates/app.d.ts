import type { SelfUser } from '$lib/api_types.js';

// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
  namespace App {
    interface Error {
      status: number;
      message: string;
      error: unknown;
    }
    interface Locals {
      user: SelfUser;
    }
    // interface PageData {}
    // interface Platform {}
  }
}

export {};
