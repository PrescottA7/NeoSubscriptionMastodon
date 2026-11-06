/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Alert,
  Box,
  Button,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { Add, Delete, ExpandMore } from "@mui/icons-material";

import type { ServerConfig } from "../types";
import { emptyServerConfig } from "./serverDefaults";
import type { FieldErrors } from "../validation";

const V2RAY_TRANSPORTS = ["tcp", "ws", "grpc", "httpupgrade", "xhttp"];

/**
 * Server-side (inbound) settings for a node.
 *
 * This is how the machine itself runs, not how a client reaches it: listener
 * address, REALITY private key, certificate paths, and fallbacks. Kept separate
 * from the protocol section so a subscription can never carry the private key.
 */
export function ServerSection({
  config,
  onChange,
  errors = {},
}: {
  config: ServerConfig | null;
  onChange: (next: ServerConfig | null) => void;
  errors?: FieldErrors;
}) {
  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  if (!config) {
    return (
      <Box>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          还没有服务端配置。填写后可以导出可直接被{" "}
          <code>xray -c</code> / <code>hysteria2 -c</code> 启动的完整配置。
        </Typography>
        <Button
          variant="outlined"
          onClick={() => onChange(emptyServerConfig())}
          startIcon={<Add />}
        >
          添加服务端配置
        </Button>
      </Box>
    );
  }

  function patch(next: Partial<ServerConfig>) {
    onChange({ ...config!, ...next });
  }

  function patchInbound(next: Partial<ServerConfig["inbound"]>) {
    patch({ inbound: { ...config!.inbound, ...next } });
  }

  function patchReality(next: Partial<ServerConfig["reality"]>) {
    patch({ reality: { ...config!.reality, ...next } });
  }

  function patchTls(next: Partial<ServerConfig["tls"]>) {
    patch({ tls: { ...config!.tls, ...next } });
  }

  const reality = config.reality;

  return (
    <Box>
      <Stack spacing={2}>
        <FormControl size="small" fullWidth>
          <InputLabel>目标核心</InputLabel>
          <Select
            label="目标核心"
            value={config.core}
            onChange={(event) => patch({ core: event.target.value as ServerConfig["core"] })}
          >
            <MenuItem value="xray">xray</MenuItem>
            <MenuItem value="hysteria2">hysteria2</MenuItem>
          </Select>
        </FormControl>

        <Accordion defaultExpanded disableGutters>
          <AccordionSummary expandIcon={<ExpandMore />}>
            <Typography sx={{ flexGrow: 1 }}>监听</Typography>
            <Typography variant="caption" color="text.secondary">
              {config.inbound.listen}:{config.inbound.port}
            </Typography>
          </AccordionSummary>
          <AccordionDetails>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                select
                label="入站协议"
                value={config.inbound.protocol}
                onChange={(event) =>
                  patchInbound({
                    protocol: event.target.value as ServerConfig["inbound"]["protocol"],
                  })
                }
              >
                <MenuItem value="vless">VLESS</MenuItem>
                <MenuItem value="trojan">Trojan</MenuItem>
                <MenuItem value="hysteria2">Hysteria2</MenuItem>
                <MenuItem value="tuic">TUIC</MenuItem>
                <MenuItem value="anytls">AnyTLS</MenuItem>
              </TextField>
              <TextField
                size="small"
                label="监听地址"
                value={config.inbound.listen}
                onChange={(event) => patchInbound({ listen: event.target.value })}
                {...field("server_params.inbound.listen")}
                helperText={field("server_params.inbound.listen").helperText ?? "0.0.0.0 表示所有网卡"}
              />
              <TextField
                size="small"
                type="number"
                label="监听端口"
                value={config.inbound.port}
                onChange={(event) => patchInbound({ port: Number(event.target.value) })}
                {...field("server_params.inbound.port")}
                helperText={field("server_params.inbound.port").helperText}
              />
              <TextField
                size="small"
                label="入站标签"
                value={config.inbound.tag}
                onChange={(event) => patchInbound({ tag: event.target.value })}
                {...field("server_params.inbound.tag")}
                helperText={field("server_params.inbound.tag").helperText ?? "路由规则里引用"}
              />
              <TextField
                size="small"
                select
                label="传输方式"
                value={config.inbound.transport}
                onChange={(event) => patchInbound({ transport: event.target.value })}
              >
                {V2RAY_TRANSPORTS.map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
              {["ws", "httpupgrade", "xhttp"].includes(config.inbound.transport) && (
                <TextField
                  size="small"
                  label="路径"
                  value={config.inbound.path}
                  onChange={(event) => patchInbound({ path: event.target.value })}
                />
              )}
              {config.inbound.transport === "grpc" && (
                <TextField
                  size="small"
                  label="gRPC 服务名"
                  value={config.inbound.service_name}
                  onChange={(event) => patchInbound({ service_name: event.target.value })}
                />
              )}
              <FormControlLabel
                control={
                  <Switch
                    checked={config.inbound.sniffing}
                    onChange={(event) => patchInbound({ sniffing: event.target.checked })}
                  />
                }
                label="流量嗅探"
              />
            </Box>
          </AccordionDetails>
        </Accordion>

        <Accordion disableGutters>
          <AccordionSummary expandIcon={<ExpandMore />}>
            <Typography sx={{ flexGrow: 1 }}>证书</Typography>
            <Typography variant="caption" color="text.secondary">
              {config.tls.certificate_file ? "已配置" : "未配置"}
            </Typography>
          </AccordionSummary>
          <AccordionDetails>
            <Stack spacing={2}>
              <Alert severity="info">
                REALITY 不需要证书。使用证书时填写服务器上的路径，导出的配置会直接引用。
              </Alert>
              <TextField
                size="small"
                label="证书文件"
                value={config.tls.certificate_file}
                onChange={(event) => patchTls({ certificate_file: event.target.value })}
                helperText="例如 /etc/ssl/fullchain.pem"
              />
              <TextField
                size="small"
                label="私钥文件"
                value={config.tls.key_file}
                onChange={(event) => patchTls({ key_file: event.target.value })}
                helperText="例如 /etc/ssl/key.pem"
              />
              <TextField
                size="small"
                label="ALPN"
                value={config.tls.alpn.join(",")}
                onChange={(event) =>
                  patchTls({
                    alpn: event.target.value
                      .split(",")
                      .map((item) => item.trim())
                      .filter((item) => item.length > 0),
                  })
                }
                helperText="逗号分隔，留空则使用核心默认值"
              />
            </Stack>
          </AccordionDetails>
        </Accordion>

        <Accordion disableGutters>
          <AccordionSummary expandIcon={<ExpandMore />}>
            <Typography sx={{ flexGrow: 1 }}>REALITY（服务端）</Typography>
            <Typography variant="caption" color="text.secondary">
              {reality.enabled ? "启用" : "关闭"}
            </Typography>
          </AccordionSummary>
          <AccordionDetails>
            <Stack spacing={2}>
              <Alert severity="warning">
                私钥只保存在服务端配置里，不会出现在订阅或客户端配置中。客户端用的公钥由它推导。
              </Alert>
              <FormControlLabel
                control={
                  <Switch
                    checked={reality.enabled}
                    onChange={(event) => patchReality({ enabled: event.target.checked })}
                  />
                }
                label="启用 REALITY"
              />
              {reality.enabled && (
                <Box
                  sx={{
                    display: "grid",
                    gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" },
                    gap: 2,
                  }}
                >
                  <TextField
                    size="small"
                    label="私钥 (privateKey)"
                    value={reality.private_key}
                    onChange={(event) => patchReality({ private_key: event.target.value })}
                    {...field("server_params.reality.private_key")}
                    helperText={
                      field("server_params.reality.private_key").helperText ??
                      "xray x25519 生成的 Private key"
                    }
                    slotProps={{ htmlInput: { style: { fontFamily: "monospace" } } }}
                  />
                  <TextField
                    size="small"
                    label="目标 (target)"
                    value={reality.target}
                    onChange={(event) => patchReality({ target: event.target.value })}
                    helperText="例如 www.microsoft.com:443"
                  />
                  <TextField
                    size="small"
                    label="serverNames"
                    value={reality.server_names.join(",")}
                    onChange={(event) =>
                      patchReality({
                        server_names: event.target.value
                          .split(",")
                          .map((item) => item.trim())
                          .filter((item) => item.length > 0),
                      })
                    }
                    helperText="逗号分隔"
                  />
                  <TextField
                    size="small"
                    label="shortIds"
                    value={reality.short_ids.join(",")}
                    onChange={(event) =>
                      patchReality({
                        short_ids: event.target.value
                          .split(",")
                          .map((item) => item.trim())
                          .filter((item) => item.length > 0),
                      })
                    }
                    helperText="逗号分隔，长度需为偶数"
                  />
                  <TextField
                    size="small"
                    label="maxTimeDiff"
                    type="number"
                    value={reality.max_time_diff}
                    onChange={(event) =>
                      patchReality({ max_time_diff: Number(event.target.value) })
                    }
                    helperText="允许的客户端时钟偏差（秒）"
                  />
                  <TextField
                    size="small"
                    label="ML-DSA-65 Seed"
                    value={reality.mldsa65_seed}
                    onChange={(event) => patchReality({ mldsa65_seed: event.target.value })}
                    helperText="可选，抗量子校验"
                  />
                </Box>
              )}
            </Stack>
          </AccordionDetails>
        </Accordion>

        <Accordion disableGutters>
          <AccordionSummary expandIcon={<ExpandMore />}>
            <Typography sx={{ flexGrow: 1 }}>回落（fallbacks）</Typography>
            <Typography variant="caption" color="text.secondary">
              {config.fallbacks.length} 条
            </Typography>
          </AccordionSummary>
          <AccordionDetails>
            <Stack spacing={1.5}>
              {config.fallbacks.map((fallback, index) => (
                <Box
                  key={index}
                  sx={{
                    display: "grid",
                    gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr auto" },
                    gap: 1.5,
                    alignItems: "start",
                  }}
                >
                  <TextField
                    size="small"
                    label="dest"
                    value={fallback.dest}
                    onChange={(event) =>
                      patch({
                        fallbacks: config.fallbacks.map((item, position) =>
                          position === index ? { ...item, dest: event.target.value } : item,
                        ),
                      })
                    }
                    helperText="例如 80 或 127.0.0.1:8080"
                  />
                  <TextField
                    size="small"
                    label="alpn / name / path"
                    value={fallback.alpn || fallback.name || fallback.path}
                    onChange={(event) =>
                      patch({
                        fallbacks: config.fallbacks.map((item, position) =>
                          position === index ? { ...item, alpn: event.target.value } : item,
                        ),
                      })
                    }
                    helperText="留空表示匹配所有请求"
                  />
                  <Button
                    size="small"
                    color="error"
                    aria-label="移除回落"
                    onClick={() =>
                      patch({
                        fallbacks: config.fallbacks.filter((_, position) => position !== index),
                      })
                    }
                  >
                    <Delete fontSize="small" />
                  </Button>
                </Box>
              ))}
              <Button
                size="small"
                startIcon={<Add />}
                onClick={() =>
                  patch({
                    fallbacks: [...config.fallbacks, { dest: "", alpn: "", name: "", path: "" }],
                  })
                }
              >
                添加回落
              </Button>
            </Stack>
          </AccordionDetails>
        </Accordion>

        <Box>
          <Button color="error" size="small" onClick={() => onChange(null)}>
            移除服务端配置
          </Button>
        </Box>
      </Stack>
    </Box>
  );
}
