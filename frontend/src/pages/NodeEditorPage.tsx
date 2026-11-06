/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  FormControlLabel,
  IconButton,
  InputLabel,
  MenuItem,
  Select,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { ArrowBack, Add, Delete, Visibility } from "@mui/icons-material";
import { useNavigate, useParams } from "react-router-dom";

import {
  createNode,
  getApiErrorMessage,
  getApiFieldErrors,
  getNode,
  listProtocols,
  renderServerConfig,
  updateNode,
} from "../api";
import { QuicSection, TlsSection } from "../components/TlsEditor";
import { TransportSection } from "../components/TransportEditor";
import { defaultParams, PROTOCOL_LABELS } from "../components/protocolDefaults";
import { ServerSection } from "../components/ServerEditor";
import { CopyButton, MonoBlock } from "../components/CopyButton";
import PageHeader from "../components/PageHeader";
import { SERVER_CORES, serverCore } from "../components/cores";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { loadCatalog } from "../store/catalogSlice";
import {
  checkDuration,
  checkHost,
  checkInteger,
  checkPort,
  checkPortRanges,
  checkText,
  isEmpty,
  validateCredentials,
  type FieldErrors,
} from "../validation";
import type {
  Credential,
  CredentialInput,
  NodeParams,
  ProtocolDescriptor,
  ServerConfig,
  ServerCore,
} from "../types";

/// Editor state for one credential row.
///
/// `uuid` and `password` stay as strings so a cleared field keeps its meaning:
/// empty means "generate a fresh value on save".
interface CredentialRow {
  user_id: number;
  uuid: string;
  password: string;
  email: string;
  enabled: boolean;
}

function toRow(credential: Credential): CredentialRow {
  return {
    user_id: credential.user_id,
    uuid: credential.uuid,
    password: credential.password,
    email: credential.email,
    enabled: credential.enabled,
  };
}

function toInput(row: CredentialRow): CredentialInput {
  return {
    user_id: row.user_id,
    uuid: row.uuid.trim() || null,
    password: row.password.trim() || null,
    email: row.email.trim() || null,
    enabled: row.enabled,
  };
}

/// Mirrors `validate_node` on the server so obvious mistakes never round-trip.
function validateForm(form: {
  name: string;
  server: string;
  port: string;
  sortOrder: string;
  params: NodeParams;
}): FieldErrors {
  const errors: FieldErrors = {};

  checkText(errors, "name", "节点名称", form.name, 1, 64);
  checkHost(errors, "server", form.server);
  checkPort(errors, "port", form.port);
  checkInteger(errors, "sort_order", "排序", form.sortOrder);

  switch (form.params.protocol) {
    case "vless": {
      const { flow } = form.params;
      if (flow && flow !== "xtls-rprx-vision") {
        errors["params.flow"] = `不支持 flow “${flow}”`;
      }
      const tls = form.params.tls;
      const needsTls = Boolean(flow);
      if (needsTls && !tls.enabled && !tls.reality?.enabled) {
        errors["params.tls.enabled"] = "使用 xtls-rprx-vision 时必须启用 TLS 或 REALITY";
      }
      if (tls.reality?.enabled && !tls.reality.public_key.trim()) {
        errors["params.tls.reality.public_key"] = "启用 REALITY 时必须填写 Public Key";
      }
      break;
    }
    case "trojan": {
      if (!form.params.tls.enabled) {
        errors["params.tls.enabled"] = "Trojan 必须启用 TLS";
      }
      break;
    }
    case "hysteria2": {
      checkPortRanges(errors, "params.server_ports", form.params.server_ports);
      checkDuration(errors, "params.hop_interval", "跳跃间隔", form.params.hop_interval);
      checkDuration(
        errors,
        "params.hop_interval_max",
        "最大跳跃间隔",
        form.params.hop_interval_max,
      );
      const obfs = form.params.obfs;
      if (obfs) {
        if (!obfs.password.trim()) {
          errors["params.obfs.password"] = "启用混淆时必须填写密码";
        }
        if (
          obfs.min_packet_size != null &&
          obfs.max_packet_size != null &&
          obfs.min_packet_size > obfs.max_packet_size
        ) {
          errors["params.obfs.max_packet_size"] = "最大包大小不能小于最小包大小";
        }
      }
      break;
    }
    case "tuic": {
      checkDuration(errors, "params.heartbeat", "心跳间隔", form.params.heartbeat);
      if (form.params.udp_over_stream && form.params.udp_relay_mode === "quic") {
        errors["params.udp_relay_mode"] = "UDP over Stream 与 quic 中继模式不能同时启用";
      }
      break;
    }
    case "anytls": {
      checkDuration(
        errors,
        "params.idle_session_check_interval",
        "空闲检查间隔",
        form.params.idle_session_check_interval,
      );
      checkDuration(
        errors,
        "params.idle_session_timeout",
        "空闲会话超时",
        form.params.idle_session_timeout,
      );
      break;
    }
  }

  return errors;
}

export default function NodeEditorPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const dispatch = useAppDispatch();
  const users = useAppSelector((state) => state.catalog.users);

  const [protocols, setProtocols] = useState<ProtocolDescriptor[]>([]);
  const [name, setName] = useState("");
  const [server, setServer] = useState("");
  // Kept as a string so a cleared input stays empty instead of becoming NaN,
  // which would serialise as `null` and be rejected by the API.
  const [port, setPort] = useState("443");
  const [enabled, setEnabled] = useState(true);
  const [sortOrder, setSortOrder] = useState("0");
  const [params, setParams] = useState<NodeParams>(defaultParams("vless"));
  // The server half: how this node runs, as opposed to how clients reach it.
  const [serverConfig, setServerConfig] = useState<ServerConfig | null>(null);
  // One row per user; a row means "this user has an account on this node".
  const [credentials, setCredentials] = useState<CredentialRow[]>([]);
  const [loading, setLoading] = useState(Boolean(id));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});

  // Which server core the preview dialog is showing.
  const [previewCore, setPreviewCore] = useState<ServerCore | null>(null);
  const [previewText, setPreviewText] = useState("");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);

  useEffect(() => {
    void listProtocols()
      .then(setProtocols)
      .catch(() => setProtocols([]));
  }, []);

  // Load the existing node once the editor is opened for a specific id.
  const nodeId = id ? Number(id) : null;
  useEffect(() => {
    if (nodeId === null) {
      return;
    }

    let cancelled = false;
    getNode(nodeId)
      .then((node) => {
        if (cancelled) {
          return;
        }
        setName(node.name);
        setServer(node.server);
        setPort(String(node.port));
        setEnabled(node.enabled);
        setSortOrder(String(node.sort_order));
        setParams(node.params);
        setServerConfig(node.server_params ?? null);
        setCredentials(node.credentials.map(toRow));
      })
      .catch((err) => {
        if (!cancelled) {
          setError(getApiErrorMessage(err, "加载节点失败"));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [nodeId]);

  function changeProtocol(protocol: NodeParams["protocol"]) {
    setParams(defaultParams(protocol));
    const descriptor = protocols.find((item) => item.name === protocol);
    if (descriptor) {
      setPort(String(descriptor.default_port));
    }
  }

  function patchRow(index: number, patch: Partial<CredentialRow>) {
    setCredentials((current) =>
      current.map((row, position) => (position === index ? { ...row, ...patch } : row)),
    );
  }

  // Render the node's inbound config for the selected core. Server-side output is
  // per-node, never per-user, so no credential selection is involved.
  useEffect(() => {
    if (!previewCore || nodeId === null) {
      return;
    }
    let cancelled = false;
    renderServerConfig(nodeId, previewCore)
      .then((document) => {
        if (!cancelled) {
          setPreviewText(document.content);
          setPreviewError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setPreviewError(getApiErrorMessage(err, "渲染服务端配置失败"));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setPreviewBusy(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [previewCore, nodeId]);

  /** Opens the preview dialog and marks it as loading in the same event. */
  function openPreview(core: ServerCore) {
    setPreviewText("");
    setPreviewError(null);
    setPreviewBusy(true);
    setPreviewCore(core);
  }

  const assignedUserIds = new Set(credentials.map((row) => row.user_id));
  const availableUsers = users.filter((user) => !assignedUserIds.has(user.id));

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    // Run the same rules as the server so mistakes surface on the right input
    // immediately; the server still validates independently.
    const errors = validateForm({
      name,
      server,
      port,
      sortOrder,
      params,
    });
    validateCredentials(errors, credentials);
    if (!isEmpty(errors)) {
      setFieldErrors(errors);
      setError(Object.values(errors)[0] ?? "请检查表单");
      return;
    }

    setFieldErrors({});
    setSaving(true);
    setError(null);

    const payload = {
      name: name.trim(),
      server: server.trim(),
      port: Number(port),
      params,
      server_params: serverConfig,
      enabled,
      sort_order: Number(sortOrder),
      credentials: credentials.map(toInput),
    };

    try {
      if (id) {
        await updateNode(Number(id), payload);
        dispatch(noticeAdded({ severity: "success", message: "节点已更新" }));
      } else {
        await createNode(payload);
        dispatch(noticeAdded({ severity: "success", message: "节点已创建" }));
      }
      await dispatch(loadCatalog());
      navigate("/", { replace: true });
    } catch (err) {
      // Map server-side field errors back onto the inputs.
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setFieldErrors(serverErrors);
      }
      setError(getApiErrorMessage(err, "保存失败"));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <Box sx={{ display: "grid", placeItems: "center", py: 8 }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Box component="form" onSubmit={handleSubmit}>
      <PageHeader
        crumbs={[{ label: "节点", to: "/" }, { label: id ? name || "未命名" : "新建节点" }]}
        title={id ? "编辑节点" : "新建节点"}
        description="客户端参数决定别人怎么连它，服务端参数决定这台机器怎么跑。"
        actions={
          <>
            <Button startIcon={<ArrowBack sx={{ fontSize: 16 }} />} onClick={() => navigate("/")}>
              返回
            </Button>
            <Button type="submit" variant="contained" disabled={saving}>
              {saving ? "保存中…" : "保存"}
            </Button>
          </>
        }
      />

      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      <Stack spacing={2}>
        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              基础信息
            </Typography>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                label="节点名称"
                value={name}
                onChange={(event) => setName(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.name)}
                helperText={fieldErrors.name}
              />
              <TextField
                label="服务器地址"
                value={server}
                onChange={(event) => setServer(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.server)}
                helperText={fieldErrors.server ?? "域名或 IP，IPv6 不需要方括号"}
              />
              <TextField
                label="端口"
                type="number"
                value={port}
                onChange={(event) => setPort(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.port)}
                helperText={fieldErrors.port}
                slotProps={{ htmlInput: { min: 1, max: 65535 } }}
              />
              <TextField
                label="排序"
                type="number"
                value={sortOrder}
                onChange={(event) => setSortOrder(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.sort_order)}
                helperText={fieldErrors.sort_order}
              />
              <FormControlLabel
                control={
                  <Switch checked={enabled} onChange={(event) => setEnabled(event.target.checked)} />
                }
                label="启用"
              />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Stack
              direction={{ xs: "column", sm: "row" }}
              spacing={2}
              sx={{ mb: 2, alignItems: { sm: "center" }, justifyContent: "space-between" }}
            >
              <Box>
                <Typography variant="subtitle2" color="text.secondary">
                  协议
                </Typography>
                <Typography variant="caption" color="text.disabled">
                  客户端侧参数：别人怎么连这个节点
                </Typography>
              </Box>
              <Button
                size="small"
                variant="outlined"
                startIcon={<Visibility sx={{ fontSize: 16 }} />}
                onClick={() => openPreview(serverCore(params.protocol))}
              >
                服务端配置
              </Button>
            </Stack>
            <FormControl size="small" fullWidth>
              <InputLabel>协议</InputLabel>
              <Select
                label="协议"
                value={params.protocol}
                onChange={(event) => changeProtocol(event.target.value as NodeParams["protocol"])}
              >
                {protocols.length === 0 &&
                  (Object.keys(PROTOCOL_LABELS) as NodeParams["protocol"][]).map((key) => (
                    <MenuItem key={key} value={key}>
                      {PROTOCOL_LABELS[key]}
                    </MenuItem>
                  ))}
                {protocols.map((descriptor) => (
                  <MenuItem key={descriptor.name} value={descriptor.name}>
                    {PROTOCOL_LABELS[descriptor.name as NodeParams["protocol"]] ?? descriptor.name}
                  </MenuItem>
                ))}
              </Select>
            </FormControl>

            <Box sx={{ mt: 2 }}>
              <ProtocolFields params={params} onChange={setParams} errors={fieldErrors} />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary">
              服务端配置
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              这台机器上代理服务端怎么运行：监听、证书、REALITY 私钥、回落。
              与上面的客户端参数分开保存，只有这里的内容会进入服务端配置文件。
            </Typography>
            <ServerSection
              config={serverConfig}
              onChange={setServerConfig}
              errors={fieldErrors}
            />
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary">
              用户凭据
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              每个节点有自己的账号表。只有在这里配置了凭据的用户才会收到该节点；
              留空的 UUID / 密码会在保存时自动生成。
            </Typography>

            <Stack spacing={1.5}>
              {credentials.length === 0 && (
                <Alert severity="info">
                  还没有用户能获取这个节点。添加一行，或点击「为所有用户生成」。
                </Alert>
              )}

              {credentials.map((row, index) => {
                const user = users.find((item) => item.id === row.user_id);
                return (
                  <Box
                    key={`${row.user_id}-${index}`}
                    sx={{
                      display: "grid",
                      gridTemplateColumns: { xs: "1fr", md: "auto 1fr 1fr auto auto" },
                      gap: 1.5,
                      alignItems: "start",
                      p: 1.5,
                      border: 1,
                      borderColor: "divider",
                      borderRadius: 1,
                    }}
                  >
                    <Typography variant="body2" sx={{ alignSelf: "center", minWidth: 90 }}>
                      {user ? user.remark || `#${user.id}` : `#${row.user_id}`}
                    </Typography>
                    <TextField
                      size="small"
                      label="UUID"
                      value={row.uuid}
                      onChange={(event) => patchRow(index, { uuid: event.target.value })}
                      error={Boolean(fieldErrors[`credentials.${index}.uuid`])}
                      helperText={
                        fieldErrors[`credentials.${index}.uuid`] ?? "留空自动生成"
                      }
                      slotProps={{ htmlInput: { style: { fontFamily: "monospace" } } }}
                    />
                    <TextField
                      size="small"
                      label="密码"
                      value={row.password}
                      onChange={(event) => patchRow(index, { password: event.target.value })}
                      error={Boolean(fieldErrors[`credentials.${index}.password`])}
                      helperText={
                        fieldErrors[`credentials.${index}.password`] ?? "留空自动生成"
                      }
                      slotProps={{ htmlInput: { style: { fontFamily: "monospace" } } }}
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          size="small"
                          checked={row.enabled}
                          onChange={(event) => patchRow(index, { enabled: event.target.checked })}
                        />
                      }
                      label="启用"
                    />
                    <IconButton
                      size="small"
                      color="error"
                      aria-label="移除凭据"
                      onClick={() =>
                        setCredentials((current) =>
                          current.filter((_, position) => position !== index),
                        )
                      }
                    >
                      <Delete fontSize="small" />
                    </IconButton>
                  </Box>
                );
              })}
            </Stack>

            <Stack direction="row" spacing={1} sx={{ mt: 2, flexWrap: "wrap" }}>
              <Button
                size="small"
                startIcon={<Add />}
                disabled={availableUsers.length === 0}
                onClick={() => {
                  const next = availableUsers[0];
                  if (next) {
                    setCredentials((current) => [
                      ...current,
                      { user_id: next.id, uuid: "", password: "", email: "", enabled: true },
                    ]);
                  }
                }}
              >
                添加用户
              </Button>
              <Button
                size="small"
                variant="outlined"
                disabled={users.length === 0}
                onClick={() =>
                  setCredentials(
                    users.map((user) => ({
                      user_id: user.id,
                      uuid: "",
                      password: "",
                      email: "",
                      enabled: true,
                    })),
                  )
                }
              >
                为所有用户生成
              </Button>
              {availableUsers.length > 0 && (
                <FormControl size="small" sx={{ minWidth: 160 }}>
                  <InputLabel>快速添加</InputLabel>
                  <Select
                    label="快速添加"
                    value=""
                    onChange={(event) => {
                      const userId = Number(event.target.value);
                      setCredentials((current) => [
                        ...current,
                        { user_id: userId, uuid: "", password: "", email: "", enabled: true },
                      ]);
                    }}
                  >
                    {availableUsers.map((user) => (
                      <MenuItem key={user.id} value={user.id}>
                        {user.remark || `#${user.id}`}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
              )}
            </Stack>
          </CardContent>
        </Card>
      </Stack>

      <Dialog
        open={previewCore !== null}
        onClose={() => setPreviewCore(null)}
        fullWidth
        maxWidth="md"
      >
        <DialogTitle sx={{ pb: 1 }}>
          <Stack direction="row" spacing={2} sx={{ alignItems: "center", flexWrap: "wrap" }}>
            <span>服务端配置</span>
            <Select
              size="small"
              value={previewCore ?? "xray"}
              onChange={(event) => setPreviewCore(event.target.value as ServerCore)}
              sx={{ minWidth: 150 }}
            >
              {SERVER_CORES.map((core) => (
                <MenuItem key={core.value} value={core.value}>
                  {core.label}
                </MenuItem>
              ))}
            </Select>
          </Stack>
        </DialogTitle>
        <DialogContent dividers>
          {nodeId === null ? (
            <Alert severity="info">先保存节点，才能按真实凭据渲染配置。</Alert>
          ) : previewError ? (
            <Alert severity="error">{previewError}</Alert>
          ) : previewBusy ? (
            <Box sx={{ display: "grid", placeItems: "center", py: 6 }}>
              <CircularProgress size={22} />
            </Box>
          ) : (
            <MonoBlock value={previewText} maxHeight={460} />
          )}
        </DialogContent>
        <DialogActions>
          <CopyButton value={previewText} label="复制配置" />
          <Button onClick={() => setPreviewCore(null)}>关闭</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

function ProtocolFields({
  params,
  onChange,
  errors = {},
}: {
  params: NodeParams;
  onChange: (next: NodeParams) => void;
  errors?: FieldErrors;
}) {
  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });
  // Errors that belong to a section rather than a single input are surfaced
  // above the fields so they are never silently swallowed.
  const sectionErrors = [
    errors["params.tls.enabled"],
    errors["params.udp_relay_mode"],
  ].filter((message): message is string => Boolean(message));
  switch (params.protocol) {
    case "vless":
      return (
        <Stack spacing={2}>
          {sectionErrors.map((message) => (
            <Alert key={message} severity="error">
              {message}
            </Alert>
          ))}
          <Box
            sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
          >
            <TextField
              select
              size="small"
              label="Flow"
              value={params.flow ?? ""}
              onChange={(event) => onChange({ ...params, flow: event.target.value })}
              {...field("params.flow")}
              helperText={field("params.flow").helperText ?? "xtls-rprx-vision 仅支持 TCP + TLS/REALITY"}
            >
              <MenuItem value="">不使用</MenuItem>
              <MenuItem value="xtls-rprx-vision">xtls-rprx-vision</MenuItem>
            </TextField>
            <TextField
              select
              size="small"
              label="UDP 包编码"
              value={params.packet_encoding ?? ""}
              onChange={(event) => onChange({ ...params, packet_encoding: event.target.value })}
            >
              <MenuItem value="">默认</MenuItem>
              <MenuItem value="xudp">xudp</MenuItem>
              <MenuItem value="packetaddr">packetaddr</MenuItem>
            </TextField>
            <TextField
              size="small"
              label="VLESS Encryption"
              value={params.encryption ?? ""}
              onChange={(event) => onChange({ ...params, encryption: event.target.value })}
              helperText="留空为 none"
            />
          </Box>
          <TlsSection
            tls={params.tls}
            onChange={(tls) => onChange({ ...params, tls })}
            errors={errors}
          />
          <TransportSection
            transport={params.transport}
            onChange={(transport) => onChange({ ...params, transport })}
            mux={params.mux}
            onMuxChange={(mux) => onChange({ ...params, mux })}
          />
        </Stack>
      );

    case "trojan":
      return (
        <Stack spacing={2}>
          <FormControlLabel
            control={
              <Switch
                checked={params.udp}
                onChange={(event) => onChange({ ...params, udp: event.target.checked })}
              />
            }
            label="启用 UDP"
          />
          <TlsSection
            tls={params.tls}
            onChange={(tls) => onChange({ ...params, tls })}
            requireTls
            allowReality={false}
            errors={errors}
          />
          <TransportSection
            transport={params.transport}
            onChange={(transport) => onChange({ ...params, transport })}
            mux={params.mux}
            onMuxChange={(mux) => onChange({ ...params, mux })}
          />
        </Stack>
      );

    case "hysteria2":
      return (
        <Stack spacing={2}>
          <Box
            sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
          >
            <TextField
              size="small"
              label="端口跳跃范围"
              value={params.server_ports.join(",")}
              onChange={(event) =>
                onChange({
                  ...params,
                  server_ports: event.target.value
                    .split(",")
                    .map((item) => item.trim())
                    .filter((item) => item.length > 0),
                })
              }
              {...field("params.server_ports")}
              helperText={field("params.server_ports").helperText ?? "例如 2080:3000,4000"}
            />
            <TextField
              size="small"
              label="跳跃间隔"
              placeholder="30s"
              value={params.hop_interval ?? ""}
              onChange={(event) => onChange({ ...params, hop_interval: event.target.value })}
              {...field("params.hop_interval")}
              helperText={field("params.hop_interval").helperText ?? "例如 30s"}
            />
            <TextField
              size="small"
              label="最大跳跃间隔"
              placeholder="45s"
              value={params.hop_interval_max ?? ""}
              onChange={(event) => onChange({ ...params, hop_interval_max: event.target.value })}
              {...field("params.hop_interval_max")}
              helperText={field("params.hop_interval_max").helperText ?? "填写后改为随机间隔"}
            />
            <TextField
              size="small"
              label="BBR 配置文件"
              placeholder="standard"
              value={params.bbr_profile ?? ""}
              onChange={(event) => onChange({ ...params, bbr_profile: event.target.value })}
            />            <TextField
              size="small"
              type="number"
              label="上行 Mbps"
              value={params.up_mbps ?? ""}
              onChange={(event) =>
                onChange({
                  ...params,
                  up_mbps: event.target.value === "" ? null : Number(event.target.value),
                })
              }
            />
            <TextField
              size="small"
              type="number"
              label="下行 Mbps"
              value={params.down_mbps ?? ""}
              onChange={(event) =>
                onChange({
                  ...params,
                  down_mbps: event.target.value === "" ? null : Number(event.target.value),
                })
              }
            />
          </Box>

          <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
            <FormControlLabel
              control={
                <Switch
                  checked={Boolean(params.obfs)}
                  onChange={(event) =>
                    onChange({
                      ...params,
                      obfs: event.target.checked
                        ? { obfs_type: "salamander", password: "", min_packet_size: null, max_packet_size: null }
                        : null,
                    })
                  }
                />
              }
              label="启用混淆"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={params.brutal_debug}
                  onChange={(event) => onChange({ ...params, brutal_debug: event.target.checked })}
                />
              }
              label="Brutal 调试日志"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={params.disable_chrome_parrot}
                  onChange={(event) =>
                    onChange({ ...params, disable_chrome_parrot: event.target.checked })
                  }
                />
              }
              label="禁用 Chrome 指纹模仿"
            />
          </Stack>

          {params.obfs && (
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <FormControl size="small">
                <InputLabel>混淆类型</InputLabel>
                <Select
                  label="混淆类型"
                  value={params.obfs.obfs_type}
                  onChange={(event) =>
                    onChange({
                      ...params,
                      obfs: { ...params.obfs!, obfs_type: event.target.value as "salamander" | "gecko" },
                    })
                  }
                >
                  <MenuItem value="salamander">salamander</MenuItem>
                  <MenuItem value="gecko">gecko</MenuItem>
                </Select>
              </FormControl>
              <TextField
                size="small"
                label="混淆密码"
                value={params.obfs.password}
                onChange={(event) =>
                  onChange({ ...params, obfs: { ...params.obfs!, password: event.target.value } })
                }
                {...field("params.obfs.password")}
              />
              <TextField
                size="small"
                type="number"
                label="最小包大小"
                value={params.obfs.min_packet_size ?? ""}
                onChange={(event) =>
                  onChange({
                    ...params,
                    obfs: {
                      ...params.obfs!,
                      min_packet_size: event.target.value === "" ? null : Number(event.target.value),
                    },
                  })
                }
                helperText="仅 gecko"
              />
              <TextField
                size="small"
                type="number"
                label="最大包大小"
                value={params.obfs.max_packet_size ?? ""}
                onChange={(event) =>
                  onChange({
                    ...params,
                    obfs: {
                      ...params.obfs!,
                      max_packet_size: event.target.value === "" ? null : Number(event.target.value),
                    },
                  })
                }
                {...field("params.obfs.max_packet_size")}
                helperText={field("params.obfs.max_packet_size").helperText ?? "仅 gecko"}
              />
            </Box>
          )}

          <TlsSection
            tls={params.tls}
            onChange={(tls) => onChange({ ...params, tls })}
            requireTls
            allowReality={false}
            errors={errors}
          />
          <QuicSection quic={params.quic} onChange={(quic) => onChange({ ...params, quic })} />
        </Stack>
      );

    case "tuic":
      return (
        <Stack spacing={2}>
          <Box
            sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
          >
            <FormControl size="small">
              <InputLabel>拥塞控制</InputLabel>
              <Select
                label="拥塞控制"
                value={params.congestion_control}
                onChange={(event) =>
                  onChange({
                    ...params,
                    congestion_control: event.target.value as typeof params.congestion_control,
                  })
                }
              >
                <MenuItem value="cubic">cubic</MenuItem>
                <MenuItem value="new_reno">new_reno</MenuItem>
                <MenuItem value="bbr">bbr</MenuItem>
              </Select>
            </FormControl>
            <FormControl size="small">
              <InputLabel>UDP 中继模式</InputLabel>
              <Select
                label="UDP 中继模式"
                value={params.udp_relay_mode}
                onChange={(event) =>
                  onChange({
                    ...params,
                    udp_relay_mode: event.target.value as typeof params.udp_relay_mode,
                  })
                }
              >
                <MenuItem value="native">native</MenuItem>
                <MenuItem value="quic">quic</MenuItem>
              </Select>
            </FormControl>
            <TextField
              size="small"
              label="心跳间隔"
              placeholder="10s"
              value={params.heartbeat ?? ""}
              onChange={(event) => onChange({ ...params, heartbeat: event.target.value })}
              {...field("params.heartbeat")}
              helperText={field("params.heartbeat").helperText ?? "例如 10s"}
            />
          </Box>
          <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
            <FormControlLabel
              control={
                <Switch
                  checked={params.udp_over_stream}
                  onChange={(event) =>
                    onChange({ ...params, udp_over_stream: event.target.checked })
                  }
                />
              }
              label="UDP over Stream"
            />
            <FormControlLabel
              control={
                <Switch
                  checked={params.zero_rtt_handshake}
                  onChange={(event) =>
                    onChange({ ...params, zero_rtt_handshake: event.target.checked })
                  }
                />
              }
              label="0-RTT 握手"
            />
          </Stack>
          <TlsSection
            tls={params.tls}
            onChange={(tls) => onChange({ ...params, tls })}
            requireTls
            allowReality={false}
            errors={errors}
          />
          <QuicSection quic={params.quic} onChange={(quic) => onChange({ ...params, quic })} />
        </Stack>
      );

    case "anytls":
      return (
        <Stack spacing={2}>
          <Box
            sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
          >
            <TextField
              size="small"
              label="空闲检查间隔"
              placeholder="30s"
              value={params.idle_session_check_interval ?? ""}
              onChange={(event) =>
                onChange({ ...params, idle_session_check_interval: event.target.value })
              }
              {...field("params.idle_session_check_interval")}
              helperText={field("params.idle_session_check_interval").helperText ?? "例如 30s"}
            />
            <TextField
              size="small"
              label="空闲会话超时"
              placeholder="30s"
              value={params.idle_session_timeout ?? ""}
              onChange={(event) =>
                onChange({ ...params, idle_session_timeout: event.target.value })
              }
              {...field("params.idle_session_timeout")}
              helperText={field("params.idle_session_timeout").helperText ?? "例如 30s"}
            />
            <TextField
              size="small"
              type="number"
              label="最少空闲会话"
              value={params.min_idle_session ?? ""}
              onChange={(event) =>
                onChange({
                  ...params,
                  min_idle_session: event.target.value === "" ? null : Number(event.target.value),
                })
              }
            />
            <TextField
              size="small"
              label="客户端元数据"
              value={params.client_metadata ?? ""}
              onChange={(event) => onChange({ ...params, client_metadata: event.target.value })}
            />
          </Box>
          <TlsSection
            tls={params.tls}
            onChange={(tls) => onChange({ ...params, tls })}
            requireTls
            allowReality={false}
            errors={errors}
          />
        </Stack>
      );

    default:
      return null;
  }
}
