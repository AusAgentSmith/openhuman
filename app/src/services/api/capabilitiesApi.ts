import { callCoreRpc } from '../coreRpcClient';
import { CORE_RPC_METHODS } from '../rpcMethods';

export type CapabilityLabel =
  | 'server-safe'
  | 'client-only'
  | 'desktop-collector'
  | 'blocked-by-tauri-bridge';

export interface ControllerCapability {
  label: CapabilityLabel;
  mobileSafe: boolean;
  standaloneServerSafe: boolean;
  requires: string[];
  reason: string;
}

export type ControllerVisibility = 'public' | 'internal';

export interface ControllerCapabilityEntry {
  method: string;
  namespace: string;
  function: string;
  visibility: ControllerVisibility;
  capability: ControllerCapability;
}

export interface RuntimeDependencyStatus {
  id: string;
  label: string;
  available: boolean;
  details: string;
}

export interface CapabilityStatus {
  counts: Record<string, number>;
  runtimeDependencies: RuntimeDependencyStatus[];
  blockedByTauriBridge: string[];
}

export interface CapabilityInventory {
  controllers: ControllerCapabilityEntry[];
  status: CapabilityStatus;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function unwrapCliEnvelope<T>(payload: unknown): T {
  const record = asRecord(payload);
  if (record && 'result' in record && 'logs' in record && Array.isArray(record.logs)) {
    return record.result as T;
  }
  return payload as T;
}

function expectObject<T extends object>(payload: unknown, context: string): T {
  const unwrapped = unwrapCliEnvelope<unknown>(payload);
  const record = asRecord(unwrapped);
  if (!record) {
    throw new Error(`${context} returned an invalid response shape`);
  }
  return record as T;
}

export async function loadCapabilitiesStatus(): Promise<CapabilityStatus> {
  const result = await callCoreRpc<unknown>({
    method: CORE_RPC_METHODS.capabilitiesStatus,
    params: {},
  });
  return expectObject<CapabilityStatus>(result, 'Capabilities status');
}

export async function loadCapabilitiesInventory(): Promise<CapabilityInventory> {
  const result = await callCoreRpc<unknown>({
    method: CORE_RPC_METHODS.capabilitiesInventory,
    params: {},
  });
  return expectObject<CapabilityInventory>(result, 'Capabilities inventory');
}

export function getControllerCapability(
  inventory: CapabilityInventory,
  method: string
): ControllerCapabilityEntry | null {
  const normalized = method.trim().toLowerCase();
  return inventory.controllers.find(entry => entry.method.toLowerCase() === normalized) ?? null;
}

export function isCapabilityAvailable(
  capability: ControllerCapability,
  status?: CapabilityStatus
): boolean {
  if (!capability.mobileSafe || !capability.standaloneServerSafe) return false;
  if (!status || capability.requires.length === 0) return true;

  const dependencies = new Map(
    status.runtimeDependencies.map(dependency => [dependency.id, dependency])
  );
  return capability.requires.every(requirement => {
    const dependency = dependencies.get(requirement);
    return !dependency || dependency.available;
  });
}
