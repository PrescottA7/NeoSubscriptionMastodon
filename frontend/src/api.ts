import axios from "axios";

import {
  AdminSchema,
  CredentialSchema,
  LoginResponseSchema,
  NodeDetailSchema,
  NodeSchema,
  ProtocolDescriptorSchema,
  ProvisionedUserSchema,
  RenderedDocumentSchema,
  SettingsSchema,
  SetupStatusSchema,
  UserSchema,
  type Admin,
  type Credential,
  type CredentialInput,
  type Node,
  type NodeDetail,
  type NodeParams,
  type ProvisionedUser,
  type ProtocolDescriptor,
  type RenderedDocument,
  type ServerCore,
  type Settings,
  type SetupStatus,
  type User,
} from "./types";

/**
 * API root. Under an admin path prefix the UI is served from `/subsc-admin`, so
 * requests have to carry that prefix too.
 */
const base = (window as { __NSM_BASE__?: string }).__NSM_BASE__ ?? "";

const http = axios.create({
  baseURL: `${base}/`,
  timeout: 15000,
  withCredentials: true,
});

let unauthorizedHandler: (() => void) | null = null;

http.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error?.response?.status === 401) {
      unauthorizedHandler?.();
    }
    return Promise.reject(error);
  },
);

export function setUnauthorizedHandler(handler: (() => void) | null): void {
  unauthorizedHandler = handler;
}

export function isUnauthorizedError(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    "response" in error &&
    (error as { response?: { status?: number } }).response?.status === 401
  );
}

export type FieldError = {
  field: string;
  message: string;
};

export type ApiErrorPayload = {
  error: string;
  details?: FieldError[];
  request_id?: string;
};

/**
 * Extracts the human-readable message from an API failure.
 *
 * The backend always sends `{ error }`, optionally with `details` for
 * field-level validation problems. Falls back to the axios message, and only
 * then to the caller's default, so the user always sees *something* specific.
 */
export function getApiErrorMessage(error: unknown, fallback: string): string {
  const payload = getApiErrorPayload(error);
  if (payload?.error) {
    return payload.error;
  }

  if (typeof error === "object" && error !== null) {
    if ("response" in error) {
      const response = (error as { response?: { status?: number; data?: unknown } }).response;
      if (response?.data && typeof response.data === "string" && response.data.length > 0) {
        return response.data;
      }
      if (response?.status) {
        return `${fallback}（HTTP ${response.status}）`;
      }
    }
    if (error instanceof Error && error.message) {
      return error.message;
    }
  }
  return fallback;
}

/** Field-level problems from a validation failure, keyed by field path. */
export function getApiFieldErrors(error: unknown): Record<string, string> {
  const payload = getApiErrorPayload(error);
  const map: Record<string, string> = {};
  for (const detail of payload?.details ?? []) {
    // Keep the first message per field; later ones are usually duplicates.
    if (!map[detail.field]) {
      map[detail.field] = detail.message;
    }
  }
  return map;
}

export function getApiErrorPayload(error: unknown): ApiErrorPayload | null {
  if (typeof error !== "object" || error === null || !("response" in error)) {
    return null;
  }
  const data = (error as { response?: { data?: unknown } }).response?.data;
  if (typeof data !== "object" || data === null) {
    return null;
  }
  const payload = data as Partial<ApiErrorPayload>;
  if (typeof payload.error !== "string") {
    return null;
  }
  return {
    error: payload.error,
    details: Array.isArray(payload.details) ? payload.details : undefined,
    request_id: payload.request_id,
  };
}

/* ----------------------------- auth ----------------------------- */

export async function getSetupStatus(): Promise<SetupStatus> {
  const { data } = await http.get("/api/auth/setup-status");
  return SetupStatusSchema.parse(data);
}

export async function setupAdmin(username: string, password: string): Promise<Admin> {
  const { data } = await http.post("/api/auth/setup", { username, password });
  return LoginResponseSchema.parse(data).admin;
}

export async function login(username: string, password: string): Promise<Admin> {
  const { data } = await http.post("/api/auth/login", { username, password });
  return LoginResponseSchema.parse(data).admin;
}

export async function logout(): Promise<void> {
  await http.post("/api/auth/logout");
}

export async function me(): Promise<Admin> {
  const { data } = await http.get("/api/auth/me");
  return AdminSchema.parse(data);
}

export async function changePassword(
  currentPassword: string,
  newPassword: string,
): Promise<void> {
  await http.post("/api/auth/password", {
    current_password: currentPassword,
    new_password: newPassword,
  });
}

/* ----------------------------- users ---------------------------- */

export async function listUsers(): Promise<User[]> {
  const { data } = await http.get("/api/users");
  return UserSchema.array().parse(data);
}

export async function createUser(payload: {
  remark?: string;
  enabled?: boolean;
  token?: string;
}): Promise<ProvisionedUser> {
  const { data } = await http.post("/api/users", payload);
  return ProvisionedUserSchema.parse(data);
}

export async function updateUser(
  id: number,
  payload: { remark?: string; enabled?: boolean; token?: string },
): Promise<User> {
  const { data } = await http.patch(`/api/users/${id}`, payload);
  return UserSchema.parse(data);
}

export async function deleteUser(id: number): Promise<void> {
  await http.delete(`/api/users/${id}`);
}

export async function resetUserLink(id: number): Promise<ProvisionedUser> {
  const { data } = await http.post(`/api/users/${id}/reset-link`);
  return ProvisionedUserSchema.parse(data);
}

export async function listUserCredentials(id: number): Promise<Credential[]> {
  const { data } = await http.get(`/api/users/${id}/credentials`);
  return CredentialSchema.array().parse(data);
}

/* ----------------------------- nodes ---------------------------- */

export type NodePayload = {
  name: string;
  server: string;
  port: number;
  params: NodeParams;
  enabled: boolean;
  sort_order: number;
  credentials: CredentialInput[];
};

export async function listProtocols(): Promise<ProtocolDescriptor[]> {
  const { data } = await http.get("/api/protocols");
  return ProtocolDescriptorSchema.array().parse(data);
}

export async function listNodes(): Promise<Node[]> {
  const { data } = await http.get("/api/nodes");
  return NodeSchema.array().parse(data);
}

export async function getNode(id: number): Promise<NodeDetail> {
  const { data } = await http.get(`/api/nodes/${id}`);
  return NodeDetailSchema.parse(data);
}

export async function createNode(payload: NodePayload): Promise<Node> {
  const { data } = await http.post("/api/nodes", payload);
  return NodeSchema.parse(data);
}

export async function updateNode(id: number, payload: NodePayload): Promise<Node> {
  const { data } = await http.put(`/api/nodes/${id}`, payload);
  return NodeSchema.parse(data);
}

export async function deleteNode(id: number): Promise<void> {
  await http.delete(`/api/nodes/${id}`);
}

/** Toggles `enabled` only, leaving every protocol parameter untouched. */
export async function setNodeEnabled(id: number, enabled: boolean): Promise<Node> {
  const { data } = await http.patch(`/api/nodes/${id}/enabled`, { enabled });
  return NodeSchema.parse(data);
}

export async function listNodeCredentials(id: number): Promise<Credential[]> {
  const { data } = await http.get(`/api/nodes/${id}/credentials`);
  return CredentialSchema.array().parse(data);
}

export async function suggestNodeCredentials(id: number): Promise<Credential[]> {
  const { data } = await http.get(`/api/nodes/${id}/suggested-credentials`);
  return CredentialSchema.array().parse(data);
}

export async function rotateNodeCredential(
  nodeId: number,
  userId: number,
): Promise<Credential> {
  const { data } = await http.post(`/api/nodes/${nodeId}/credentials/${userId}/rotate`);
  return CredentialSchema.parse(data);
}

/* ---------------------------- rendering ------------------------- */

export type RenderTarget = "uri" | "sing-box" | "mihomo" | "xray" | "hysteria2";

export async function renderNode(
  id: number,
  core: RenderTarget,
  userId?: number,
): Promise<RenderedDocument> {
  const { data } = await http.get(`/api/nodes/${id}/render`, {
    params: { core, full: 1, user_id: userId },
  });
  return RenderedDocumentSchema.parse(data);
}

export async function renderAll(core: RenderTarget, userId?: number): Promise<RenderedDocument> {
  const { data } = await http.get("/api/render", { params: { core, user_id: userId } });
  return RenderedDocumentSchema.parse(data);
}

/**
 * Renders the node's inbound (server-side) config with the requested core.
 * Server output is per-node, so no user selection is involved.
 */
export async function renderServerConfig(
  id: number,
  core: ServerCore,
): Promise<RenderedDocument> {
  const { data } = await http.get(`/api/nodes/${id}/server-config`, { params: { core } });
  return RenderedDocumentSchema.parse(data);
}

export function downloadDocument(document: RenderedDocument): void {
  const blob = new Blob([document.content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = window.document.createElement("a");
  anchor.href = url;
  anchor.download = document.filename;
  anchor.click();
  URL.revokeObjectURL(url);
}

/* ---------------------------- settings -------------------------- */

export async function getSettings(): Promise<Settings> {
  const { data } = await http.get("/api/settings");
  return SettingsSchema.parse(data);
}

export async function updateSettings(payload: {
  subscription_prefix?: string;
  default_target?: string;
  failure_delay_ms?: number;
  site_name?: string;
  disguise_passthrough?: boolean;
  disguise_template_url?: string;
  disguise_template_ip?: string;
  disguise_template_port?: number;
}): Promise<Settings> {
  const { data } = await http.put("/api/settings", payload);
  return SettingsSchema.parse(data);
}

export async function updateDisguisePage(html: string): Promise<Settings> {
  const { data } = await http.put("/api/settings/disguise-page", { html });
  return SettingsSchema.parse(data);
}
