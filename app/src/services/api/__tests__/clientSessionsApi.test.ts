import { beforeEach, describe, expect, it, vi } from 'vitest';

const mockCallCoreRpc = vi.fn();

vi.mock('../../coreRpcClient', () => ({
  callCoreRpc: (...args: unknown[]) => mockCallCoreRpc(...args),
}));

const { clientSessionsApi } = await import('../clientSessionsApi');

describe('clientSessionsApi', () => {
  beforeEach(() => {
    mockCallCoreRpc.mockReset();
  });

  it('loads status from a CLI envelope', async () => {
    mockCallCoreRpc.mockResolvedValueOnce({
      result: {
        session_model: 'static_bearer_plus_device_sessions',
        device_scoped_tokens: true,
        revocation_supported: true,
        static_bearer_enabled: true,
        provider_tokens_server_side: true,
        mobile_public_ready: false,
        recommended_next_step: 'Use sessions.',
        client_token_storage: 'hashed_device_tokens',
        provider_token_storage: 'server_auth_service',
        sessions: { initialized: true, activeCount: 1, revokedCount: 0, totalCount: 1 },
      },
      logs: ['security_client_sessions_status computed'],
    });

    await expect(clientSessionsApi.status()).resolves.toMatchObject({
      device_scoped_tokens: true,
      sessions: { activeCount: 1 },
    });
  });

  it('creates a session and returns the one-time token', async () => {
    mockCallCoreRpc.mockResolvedValueOnce({
      result: {
        session: {
          id: 's1',
          label: 'phone',
          tokenPrefix: 'ohs_s1_ab',
          createdAt: '2026-05-25T00:00:00Z',
          lastSeenAt: null,
          revokedAt: null,
        },
        token: 'ohs_s1_secret',
      },
      logs: ['security_client_sessions_create issued'],
    });

    await expect(clientSessionsApi.create('phone')).resolves.toMatchObject({
      session: { id: 's1', label: 'phone' },
      token: 'ohs_s1_secret',
    });
    expect(mockCallCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.security_client_sessions_create',
      params: { label: 'phone' },
    });
  });

  it('lists sessions and rejects invalid shape', async () => {
    mockCallCoreRpc.mockResolvedValueOnce({
      result: { sessions: [{ id: 's1', label: 'phone' }] },
      logs: ['security_client_sessions_list returned'],
    });
    await expect(clientSessionsApi.list()).resolves.toEqual([{ id: 's1', label: 'phone' }]);

    mockCallCoreRpc.mockResolvedValueOnce({ result: { sessions: null }, logs: [] });
    await expect(clientSessionsApi.list()).rejects.toThrow('Client sessions list');
  });

  it('revokes a session and surfaces server errors', async () => {
    mockCallCoreRpc.mockResolvedValueOnce({
      result: { revoked: true, session: { id: 's1', label: 'phone' } },
      logs: ['security_client_sessions_revoke revoked'],
    });
    await expect(clientSessionsApi.revoke('s1')).resolves.toMatchObject({ revoked: true });

    mockCallCoreRpc.mockResolvedValueOnce({
      result: { revoked: false, error: 'session not found' },
      logs: ['security_client_sessions_revoke not_found'],
    });
    await expect(clientSessionsApi.revoke('missing')).rejects.toThrow('session not found');
  });
});
