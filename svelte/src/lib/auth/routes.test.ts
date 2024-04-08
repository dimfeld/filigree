import { test, expect } from 'vitest';
import { hasPermissions, protectRoutes } from './routes.js';
import type { RequestEvent } from '@sveltejs/kit';

function mockRequest(path: string, perms?: string[] | undefined) {
  return {
    route: {
      id: path,
    },
    locals: {
      user: perms
        ? {
            permissions: perms,
          }
        : undefined,
    },
  } as unknown as RequestEvent;
}

test('allowUnauthed', () => {
  const protect = protectRoutes({
    allowUnauthed: ['/login', '/images', '/others/blah'],
  });

  expect(protect(mockRequest('/login'))).eq('ok');
  expect(protect(mockRequest('/images'))).eq('ok');
  expect(protect(mockRequest('/images/1'))).eq('ok');
  expect(protect(mockRequest('/others/blah'))).eq('ok');
  expect(protect(mockRequest('/others'))).eq('unknown-user');
  expect(protect(mockRequest('/different-url'))).eq('unknown-user');

  expect(protect(mockRequest('/login', ['read']))).eq('ok');
  expect(protect(mockRequest('/images', ['read']))).eq('ok');
  expect(protect(mockRequest('/images/1', ['read']))).eq('ok');
  expect(protect(mockRequest('/others/blah', ['read']))).eq('ok');
  expect(protect(mockRequest('/others', ['read']))).eq('ok');
  expect(protect(mockRequest('/different-url', ['read']))).eq('ok');
});

test('requireAuth', () => {
  const protect = protectRoutes({
    requireAuth: ['/posts', '/images', '/others/blah'],
  });

  expect(protect(mockRequest('/login'))).eq('ok');
  expect(protect(mockRequest('/images'))).eq('unknown-user');
  expect(protect(mockRequest('/images/1'))).eq('unknown-user');
  expect(protect(mockRequest('/others/blah'))).eq('unknown-user');
  expect(protect(mockRequest('/others'))).eq('ok');
  expect(protect(mockRequest('/different-url'))).eq('ok');

  expect(protect(mockRequest('/login', ['read']))).eq('ok');
  expect(protect(mockRequest('/images', ['read']))).eq('ok');
  expect(protect(mockRequest('/images/1', ['read']))).eq('ok');
  expect(protect(mockRequest('/others/blah', ['read']))).eq('ok');
  expect(protect(mockRequest('/others', ['read']))).eq('ok');
  expect(protect(mockRequest('/different-url', ['read']))).eq('ok');
});

test('checkers', () => {
  const protect = protectRoutes({
    check: {
      '/posts': hasPermissions(['read']),
      '/images': hasPermissions(['read']),
      '/images/[imageId]/edit': hasPermissions(['read', 'write']),
    },
  });

  expect(protect(mockRequest('/login'))).eq('ok');
  expect(protect(mockRequest('/images'))).eq('unknown-user');
  expect(protect(mockRequest('/images/[imageId]/edit'))).eq('unknown-user');

  expect(protect(mockRequest('/login', ['read']))).eq('ok');
  expect(protect(mockRequest('/images', ['read']))).eq('ok');
  expect(protect(mockRequest('/images/[imageId]/edit', ['read']))).eq('forbidden');
  expect(protect(mockRequest('/images/[imageId]/edit', ['read', 'write']))).eq('ok');
  expect(protect(mockRequest('/images/[imageId]/edit/resize', ['read']))).eq('forbidden');
  expect(protect(mockRequest('/images/[imageId]/edit/resize', ['read', 'write']))).eq('ok');
});

test('full with unauthed root', () => {
  const protect = protectRoutes({
    allowUnauthed: ['/'],
    requireAuth: ['/topics'],
    check: {
      '/posts': hasPermissions(['read']),
      '/images': hasPermissions(['read']),
      '/images/[imageId]/edit': hasPermissions(['read', 'write']),
    },
  });

  expect(protect(mockRequest('/'))).eq('ok');
  expect(protect(mockRequest('/login'))).eq('ok');
  expect(protect(mockRequest('/topics'))).eq('unknown-user');
  expect(protect(mockRequest('/images'))).eq('unknown-user');
  expect(protect(mockRequest('/images/[imageId]/edit'))).eq('unknown-user');

  expect(protect(mockRequest('/', ['read']))).eq('ok');
  expect(protect(mockRequest('/login', ['read']))).eq('ok');
  expect(protect(mockRequest('/topics', ['read']))).eq('ok');
  expect(protect(mockRequest('/images', ['read']))).eq('ok');
  expect(protect(mockRequest('/images/[imageId]/edit', ['read']))).eq('forbidden');
  expect(protect(mockRequest('/images/[imageId]/edit', ['read', 'write']))).eq('ok');
  expect(protect(mockRequest('/images/[imageId]/edit/resize', ['read']))).eq('forbidden');
  expect(protect(mockRequest('/images/[imageId]/edit/resize', ['read', 'write']))).eq('ok');
});
