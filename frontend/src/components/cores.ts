import type { NodeParams, ServerCore } from "../types";

/** Proxy servers NSM can emit an inbound config for. */
export const SERVER_CORES: { value: ServerCore; label: string }[] = [
  { value: "xray", label: "Xray" },
  { value: "hysteria2", label: "Hysteria2" },
];

/**
 * The core that can actually run a listener for `protocol`.
 *
 * Xray has no Hysteria2 inbound and the Hysteria2 exporter only understands
 * Hysteria2, so each protocol has exactly one sensible server core.
 */
export function serverCore(protocol: NodeParams["protocol"]): ServerCore {
  return protocol === "hysteria2" ? "hysteria2" : "xray";
}
