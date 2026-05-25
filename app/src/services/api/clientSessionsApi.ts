import { callCoreRpc } from '../coreRpcClient';
import { CORE_RPC_METHODS } from '../rpcMethods';

export interface ClientSession {
  id: string;
  label: string;
  tokenPrefix: string;
  createdAt: string;
  lastSeenAt?: string | null;
  revokedAt?: string | null;
}

export interface CreatedClientSession {
  session: ClientSession;
  token: string;
}

export interface ClientSessionsStatus {
  session_model: string;
  device_scoped_tokens: boolean;
  revocation_supported: boolean;
  static_bearer_enabled: boolean;
  provider_tokens_server_side: boolean;
  mobile_public_ready: boolean;
  recommended_next_step: string;
  client_token_storage: string;
  provider_token_storage: string;
  sessions: { initialized: boolean; activeCount: number; revokedCount: number; totalCount: number };
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
  if (!record) throw new Error(`${context} returned an invalid response shape`);
  if (typeof record.error === 'string' && record.error.trim()) {
    throw new Error(record.error);
  }
  return record as T;
}

export const clientSessionsApi = {
  status: async (): Promise<ClientSessionsStatus> => {
    const result = await callCoreRpc<unknown>({
      method: CORE_RPC_METHODS.securityClientSessionsStatus,
      params: {},
    });
    return expectObject<ClientSessionsStatus>(result, 'Client sessions status');
  },

  list: async (): Promise<ClientSession[]> => {
    const result = await callCoreRpc<unknown>({
      method: CORE_RPC_METHODS.securityClientSessionsList,
      params: {},
    });
    const record = expectObject<{ sessions?: unknown }>(result, 'Client sessions list');
    if (!Array.isArray(record.sessions)) {
      throw new Error('Client sessions list returned an invalid response shape');
    }
    return record.sessions as ClientSession[];
  },

  create: async (label?: string): Promise<CreatedClientSession> => {
    const result = await callCoreRpc<unknown>({
      method: CORE_RPC_METHODS.securityClientSessionsCreate,
      params: { label },
    });
    const record = expectObject<CreatedClientSession>(result, 'Client sessions create');
    if (!record.session || typeof record.token !== 'string' || !record.token) {
      throw new Error('Client sessions create returned an invalid response shape');
    }
    return record;
  },

  revoke: async (sessionId: string): Promise<{ revoked: boolean; session?: ClientSession }> => {
    const result = await callCoreRpc<unknown>({
      method: CORE_RPC_METHODS.securityClientSessionsRevoke,
      params: { session_id: sessionId },
    });
    const record = expectObject<{ revoked?: unknown; session?: ClientSession }>(
      result,
      'Client sessions revoke'
    );
    if (typeof record.revoked !== 'boolean') {
      throw new Error('Client sessions revoke returned an invalid response shape');
    }
    return { revoked: record.revoked, session: record.session };
  },
};
