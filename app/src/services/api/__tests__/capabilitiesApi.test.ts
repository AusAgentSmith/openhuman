import { beforeEach, describe, expect, it, vi } from 'vitest';

import type {
  CapabilityInventory,
  CapabilityStatus,
  ControllerCapability,
} from '../capabilitiesApi';

const mockCallCoreRpc = vi.fn();

vi.mock('../../coreRpcClient', () => ({
  callCoreRpc: (...args: unknown[]) => mockCallCoreRpc(...args),
}));

const {
  getControllerCapability,
  isCapabilityAvailable,
  loadCapabilitiesInventory,
  loadCapabilitiesStatus,
} = await import('../capabilitiesApi');

describe('capabilitiesApi', () => {
  beforeEach(() => {
    mockCallCoreRpc.mockReset();
  });

  it('loads and unwraps capability status from the CLI envelope', async () => {
    const status: CapabilityStatus = {
      counts: { 'server-safe': 4, 'blocked-by-tauri-bridge': 1 },
      runtimeDependencies: [
        {
          id: 'tauri:webview_apis',
          label: 'Desktop webview APIs bridge',
          available: false,
          details: 'OPENHUMAN_WEBVIEW_APIS_PORT is not set',
        },
      ],
      blockedByTauriBridge: ['openhuman.webview_apis_gmail_search'],
    };
    mockCallCoreRpc.mockResolvedValueOnce({
      result: status,
      logs: ['capability status generated'],
    });

    await expect(loadCapabilitiesStatus()).resolves.toEqual(status);
    expect(mockCallCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.capabilities_status',
      params: {},
    });
  });

  it('loads and unwraps capability inventory from the CLI envelope', async () => {
    const inventory: CapabilityInventory = {
      controllers: [
        {
          method: 'openhuman.health_snapshot',
          namespace: 'health',
          function: 'snapshot',
          visibility: 'public',
          capability: {
            label: 'server-safe',
            mobileSafe: true,
            standaloneServerSafe: true,
            requires: [],
            reason: 'server owned',
          },
        },
      ],
      status: { counts: { 'server-safe': 1 }, runtimeDependencies: [], blockedByTauriBridge: [] },
    };
    mockCallCoreRpc.mockResolvedValueOnce({
      result: inventory,
      logs: ['capability inventory generated'],
    });

    await expect(loadCapabilitiesInventory()).resolves.toEqual(inventory);
    expect(getControllerCapability(inventory, 'OPENHUMAN.HEALTH_SNAPSHOT')?.namespace).toBe(
      'health'
    );
  });

  it('rejects invalid capability status shape', async () => {
    mockCallCoreRpc.mockResolvedValueOnce(null);

    await expect(loadCapabilitiesStatus()).rejects.toThrow(
      'Capabilities status returned an invalid response shape'
    );
  });

  it('treats mobile-safe server capabilities as available only when dependencies are available', () => {
    const capability: ControllerCapability = {
      label: 'server-safe',
      mobileSafe: true,
      standaloneServerSafe: true,
      requires: ['provider:gmail'],
      reason: 'server owned',
    };
    expect(
      isCapabilityAvailable(capability, {
        counts: {},
        runtimeDependencies: [
          { id: 'provider:gmail', label: 'Gmail', available: true, details: 'connected' },
        ],
        blockedByTauriBridge: [],
      })
    ).toBe(true);
    expect(
      isCapabilityAvailable(capability, {
        counts: {},
        runtimeDependencies: [
          { id: 'provider:gmail', label: 'Gmail', available: false, details: 'not connected' },
        ],
        blockedByTauriBridge: [],
      })
    ).toBe(false);
  });

  it('treats client-only and bridge-backed capabilities as unavailable to mobile', () => {
    for (const label of ['client-only', 'blocked-by-tauri-bridge'] as const) {
      expect(
        isCapabilityAvailable({
          label,
          mobileSafe: false,
          standaloneServerSafe: false,
          requires: [],
          reason: 'not mobile safe',
        })
      ).toBe(false);
    }
  });
});
