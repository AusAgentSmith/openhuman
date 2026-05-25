/**
 * Unit tests for the boot-check service-backed transport.
 *
 * Validates that bootCheckTransport delegates correctly to callCoreRpc and
 * @tauri-apps/api/core invoke, since these are the production wiring used by
 * BootCheckGate.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

const callCoreRpcMock = vi.fn();
vi.mock('./coreRpcClient', () => ({ callCoreRpc: (req: unknown) => callCoreRpcMock(req) }));

const invokeMock = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => invokeMock(cmd, args),
}));

const isTauriMock = vi.fn(() => true);
vi.mock('../utils/tauriCommands/common', () => ({ isTauri: () => isTauriMock() }));

describe('bootCheckTransport', () => {
  beforeEach(() => {
    callCoreRpcMock.mockReset();
    invokeMock.mockReset();
    isTauriMock.mockReset();
    isTauriMock.mockReturnValue(true);
  });

  it('callRpc forwards method+params to callCoreRpc', async () => {
    callCoreRpcMock.mockResolvedValueOnce({ ok: true });

    const { bootCheckTransport } = await import('./bootCheckService');
    const result = await bootCheckTransport.callRpc<{ ok: boolean }>('openhuman.ping', { x: 1 });

    expect(result).toEqual({ ok: true });
    expect(callCoreRpcMock).toHaveBeenCalledWith({ method: 'openhuman.ping', params: { x: 1 } });
  });

  it('invokeCmd forwards cmd+args to Tauri invoke', async () => {
    invokeMock.mockResolvedValueOnce(42);

    const { bootCheckTransport } = await import('./bootCheckService');
    const result = await bootCheckTransport.invokeCmd<number>('start_core_process', {});

    expect(result).toBe(42);
    expect(invokeMock).toHaveBeenCalledWith('start_core_process', {});
  });

  it('configureCoreConnection forwards cloud URL and token to Tauri', async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    const { bootCheckTransport } = await import('./bootCheckService');
    await bootCheckTransport.configureCoreConnection?.({
      kind: 'cloud',
      url: 'https://core.example.com/rpc',
      token: 'tok-123',
    });

    expect(invokeMock).toHaveBeenCalledWith('configure_core_rpc_connection', {
      url: 'https://core.example.com/rpc',
      token: 'tok-123',
    });
  });

  it('configureCoreConnection clears Tauri back to local mode', async () => {
    invokeMock.mockResolvedValueOnce(undefined);

    const { bootCheckTransport } = await import('./bootCheckService');
    await bootCheckTransport.configureCoreConnection?.({ kind: 'local' });

    expect(invokeMock).toHaveBeenCalledWith('configure_core_rpc_connection', {
      url: null,
      token: null,
    });
  });

  it('configureCoreConnection is a no-op outside Tauri', async () => {
    isTauriMock.mockReturnValueOnce(false);

    const { bootCheckTransport } = await import('./bootCheckService');
    await bootCheckTransport.configureCoreConnection?.({
      kind: 'cloud',
      url: 'https://core.example.com/rpc',
      token: 'tok-123',
    });

    expect(invokeMock).not.toHaveBeenCalledWith('configure_core_rpc_connection', expect.anything());
  });
});
