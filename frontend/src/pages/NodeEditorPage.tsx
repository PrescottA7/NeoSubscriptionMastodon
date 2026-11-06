/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useEffect, useRef, useState } from "react";
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
import { useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import {
  createNode,
  getApiErrorMessage,
  getApiFieldErrors,
  getNode,
  listProtocols,
  renderServerConfig,
  updateNode,
} from "../api";
import { duplicateName, parsePositiveId } from "../utils";
import { defaultParams } from "../components/protocolDefaults";
import { ServerSection } from "../components/server";
import { coreForProtocol } from "../components/server/defaults";
import { CopyButton, MonoBlock } from "../components/CopyButton";
import PageHeader from "../components/PageHeader";
import { CredentialFields, EMPTY_CREDENTIALS } from "../components/credentials";
import { ProtocolFields, toRow, toInput, type CredentialRow } from "../components/nodeEditor";
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
  NodeParams,
  ProtocolDescriptor,
  ProtocolName,
  ServerConfig,
  ServerCore,
} from "../types";

export default function NodeEditorPage() {
  const { id } = useParams();
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const dispatch = useAppDispatch();
  const { t } = useTranslation();
  const users = useAppSelector((state) => state.catalog.users);
  const knownNodes = useAppSelector((state) => state.catalog.nodes);

  // `?from=<id>` opens the editor for a new node prefilled from an existing one,
  // so the duplicate is only created when the admin saves it.
  const copyFromId = id ? null : parsePositiveId(searchParams.get("from"));
  const copying = copyFromId !== null;

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
  const [loading, setLoading] = useState(Boolean(id) || copyFromId !== null);
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

  // Load the existing node once the editor is opened for a specific id, or the
  // source node when duplicating one.
  const nodeId = id ? Number(id) : null;
  const sourceId = nodeId ?? copyFromId;

  // Held in a ref so a catalogue refresh cannot re-run the prefill below and
  // wipe edits the admin has already made.
  const existingNames = useRef<string[]>([]);
  useEffect(() => {
    existingNames.current = knownNodes.map((item) => item.name);
  }, [knownNodes]);

  useEffect(() => {
    if (sourceId === null) {
      return;
    }

    let cancelled = false;
    getNode(sourceId)
      .then((node) => {
        if (cancelled) {
          return;
        }
        // A duplicate keeps every parameter, credential and the enabled flag;
        // nothing is stored until the admin saves, so it stays adjustable.
        const duplicating = node.id !== nodeId;
        setName(duplicating ? duplicateName(node.name, existingNames.current) : node.name);
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
          setError(getApiErrorMessage(err, t("nodeEditor.loadFailed")));
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
  }, [sourceId, nodeId, t]);

  /**
   * The core this node's credentials are shaped for.
   *
   * Derived from the stored server config so the credential inputs match what
   * the exporter will actually read; without a server config there is no
   * listener yet, so the protocol's natural core is assumed.
   */
  const credentialCore: ServerCore =
    serverConfig?.core ?? coreForProtocol(params.protocol) ?? "xray";

  function changeProtocol(protocol: NodeParams["protocol"]) {
    setParams(defaultParams(protocol));
    const descriptor = protocols.find((item) => item.name === protocol);
    if (descriptor) {
      setPort(String(descriptor.default_port));
    }
    // The credential shape follows the protocol, so switching one clears the
    // other: a leftover UUID on a Trojan node would otherwise be submitted.
    setCredentials((current) =>
      current.map((row) => ({ ...row, uuid: "", password: "", username: "" })),
    );
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
          setPreviewError(getApiErrorMessage(err, t("nodeEditor.renderServerFailed")));
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
  }, [previewCore, nodeId, t]);

  /** Opens the preview dialog and marks it as loading in the same event. */
  function openPreview(core: ServerCore) {
    setPreviewText("");
    setPreviewError(null);
    setPreviewBusy(true);
    setPreviewCore(core);
  }

  const assignedUserIds = new Set(credentials.map((row) => row.user_id));
  const availableUsers = users.filter((user) => !assignedUserIds.has(user.id));

  /** Mirrors `validate_node` on the server so obvious mistakes never round-trip. */
  function validateForm(): FieldErrors {
    const errors: FieldErrors = {};

    checkText(errors, "name", t("nodes.nodeName"), name, 1, 64);
    checkHost(errors, "server", server);
    checkPort(errors, "port", port);
    checkInteger(errors, "sort_order", t("nodes.sortOrder"), sortOrder);

    switch (params.protocol) {
      case "vless": {
        const { flow } = params;
        if (flow && flow !== "xtls-rprx-vision") {
          errors["params.flow"] = t("validation.flowUnsupported", { flow });
        }
        const tls = params.tls;
        if (flow && !tls.enabled && !tls.reality?.enabled) {
          errors["params.tls.enabled"] = t("validation.tlsRequiredForXtls");
        }
        if (tls.reality?.enabled && !tls.reality.public_key.trim()) {
          errors["params.tls.reality.public_key"] = t("validation.realityPublicKeyRequired");
        }
        break;
      }
      case "trojan": {
        if (!params.tls.enabled) {
          errors["params.tls.enabled"] = t("validation.tlsRequiredForTrojan");
        }
        break;
      }
      case "hysteria2": {
        checkPortRanges(errors, "params.server_ports", params.server_ports);
        checkDuration(errors, "params.hop_interval", t("hysteria2.hopInterval"), params.hop_interval);
        checkDuration(
          errors,
          "params.hop_interval_max",
          t("hysteria2.maxHopInterval"),
          params.hop_interval_max,
        );
        const obfs = params.obfs;
        if (obfs) {
          if (!obfs.password.trim()) {
            errors["params.obfs.password"] = t("validation.obfsPasswordRequired");
          }
          if (
            obfs.min_packet_size != null &&
            obfs.max_packet_size != null &&
            obfs.min_packet_size > obfs.max_packet_size
          ) {
            errors["params.obfs.max_packet_size"] = t("validation.maxPacketSizeInvalid");
          }
        }
        break;
      }
      case "tuic": {
        checkDuration(errors, "params.heartbeat", t("tuic.heartbeat"), params.heartbeat);
        if (params.udp_over_stream && params.udp_relay_mode === "quic") {
          errors["params.udp_relay_mode"] = t("validation.udpRelayConflict");
        }
        break;
      }
      case "anytls": {
        checkDuration(
          errors,
          "params.idle_session_check_interval",
          t("anytls.idleCheckInterval"),
          params.idle_session_check_interval,
        );
        checkDuration(
          errors,
          "params.idle_session_timeout",
          t("anytls.idleSessionTimeout"),
          params.idle_session_timeout,
        );
        break;
      }
    }

    validateCredentials(errors, credentials);
    return errors;
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    // Run the same rules as the server so mistakes surface on the right input
    // immediately; the server still validates independently.
    const errors = validateForm();
    if (!isEmpty(errors)) {
      setFieldErrors(errors);
      setError(Object.values(errors)[0] ?? t("nodeEditor.checkForm"));
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
        dispatch(noticeAdded({ severity: "success", message: t("nodeEditor.nodeUpdated") }));
      } else {
        await createNode(payload);
        dispatch(
          noticeAdded({
            severity: "success",
            message: t(copying ? "nodeEditor.nodeDuplicated" : "nodeEditor.nodeCreated"),
          }),
        );
      }
      await dispatch(loadCatalog());
      navigate("/", { replace: true });
    } catch (err) {
      // Map server-side field errors back onto the inputs.
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setFieldErrors(serverErrors);
      }
      setError(getApiErrorMessage(err, t("nodeEditor.saveFailed")));
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
        crumbs={[
          { label: t("nodes.breadcrumb"), to: "/" },
          {
            label:
              id || copying
                ? name || t("nodeEditor.unnamed")
                : t("nodeEditor.newNode"),
          },
        ]}
        title={
          id
            ? t("nodeEditor.titleEdit")
            : copying
              ? t("nodeEditor.titleDuplicate")
              : t("nodeEditor.titleNew")
        }
        description={t(copying ? "nodeEditor.duplicateDescription" : "nodeEditor.description")}
        actions={
          <>
            <Button startIcon={<ArrowBack sx={{ fontSize: 16 }} />} onClick={() => navigate("/")}>
              {t("nodeEditor.back")}
            </Button>
            <Button type="submit" variant="contained" disabled={saving}>
              {saving ? t("nodes.saving") : t("nodes.save")}
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
              {t("nodeEditor.basicInfo")}
            </Typography>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                label={t("nodes.nodeName")}
                value={name}
                onChange={(event) => setName(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.name)}
                helperText={fieldErrors.name}
              />
              <TextField
                label={t("nodes.serverAddress")}
                value={server}
                onChange={(event) => setServer(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.server)}
                helperText={fieldErrors.server ?? t("nodes.serverAddressHelper")}
              />
              <TextField
                label={t("nodes.port")}
                type="number"
                value={port}
                onChange={(event) => setPort(event.target.value)}
                size="small"
                error={Boolean(fieldErrors.port)}
                helperText={fieldErrors.port}
                slotProps={{ htmlInput: { min: 1, max: 65535 } }}
              />
              <TextField
                label={t("nodes.sortOrder")}
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
                label={t("nodes.enabled")}
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
                  {t("nodeEditor.clientSection")}
                </Typography>
                <Typography variant="caption" color="text.disabled">
                  {t("nodeEditor.clientSectionHint")}
                </Typography>
              </Box>
              <Button
                size="small"
                variant="outlined"
                startIcon={<Visibility sx={{ fontSize: 16 }} />}
                onClick={() => openPreview(credentialCore)}
              >
                {t("nodeEditor.previewConfig")}
              </Button>
            </Stack>
            <FormControl size="small" fullWidth>
              <InputLabel>{t("nodes.protocol")}</InputLabel>
              <Select
                label={t("nodes.protocol")}
                value={params.protocol}
                onChange={(event) => changeProtocol(event.target.value as NodeParams["protocol"])}
              >
                {protocols.length === 0 &&
                  (["vless", "trojan", "hysteria2", "tuic", "anytls"] as ProtocolName[]).map(
                    (key) => (
                      <MenuItem key={key} value={key}>
                        {t(`protocol.${key}`)}
                      </MenuItem>
                    ),
                  )}
                {protocols.map((descriptor) => (
                  <MenuItem key={descriptor.name} value={descriptor.name}>
                    {t(`protocol.${descriptor.name}`, descriptor.name)}
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
              {t("nodeEditor.serverSection")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t("nodeEditor.serverSectionHint")}
            </Typography>
            <ServerSection
              config={serverConfig}
              onChange={setServerConfig}
              errors={fieldErrors}
              protocol={params.protocol}
            />
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary">
              {t("nodeEditor.credentials")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
              {t("nodeEditor.credentialsDescription")}
            </Typography>
            <Typography variant="caption" color="text.disabled" sx={{ display: "block", mb: 2 }}>
              {t("core.credentialsHint", { core: t(`core.${credentialCore}`) })}
            </Typography>

            <Stack spacing={1.5}>
              {credentials.length === 0 && (
                <Alert severity="info">{t("nodeEditor.credentialsEmpty")}</Alert>
              )}

              {credentials.map((row, index) => {
                const user = users.find((item) => item.id === row.user_id);
                return (
                  <Box
                    key={`${row.user_id}-${index}`}
                    sx={{
                      display: "grid",
                      gridTemplateColumns: { xs: "1fr", md: "auto 1fr auto auto" },
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
                    <CredentialFields
                      core={credentialCore}
                      protocol={params.protocol}
                      values={row}
                      index={index}
                      errors={fieldErrors}
                      onChange={(next) => patchRow(index, next)}
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          size="small"
                          checked={row.enabled}
                          onChange={(event) => patchRow(index, { enabled: event.target.checked })}
                        />
                      }
                      label={t("nodes.enabled")}
                    />
                    <IconButton
                      size="small"
                      color="error"
                      aria-label={t("nodeEditor.removeCredential")}
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
                      { ...EMPTY_CREDENTIALS, user_id: next.id, email: "", enabled: true },
                    ]);
                  }
                }}
              >
                {t("nodeEditor.addUser")}
              </Button>
              <Button
                size="small"
                variant="outlined"
                disabled={users.length === 0}
                onClick={() =>
                  setCredentials(
                    users.map((user) => ({
                      ...EMPTY_CREDENTIALS,
                      user_id: user.id,
                      email: "",
                      enabled: true,
                    })),
                  )
                }
              >
                {t("nodeEditor.generateForAll")}
              </Button>
              {availableUsers.length > 0 && (
                <FormControl size="small" sx={{ minWidth: 160 }}>
                  <InputLabel>{t("nodeEditor.quickAdd")}</InputLabel>
                  <Select
                    label={t("nodeEditor.quickAdd")}
                    value=""
                    onChange={(event) => {
                      const userId = Number(event.target.value);
                      setCredentials((current) => [
                        ...current,
                        { ...EMPTY_CREDENTIALS, user_id: userId, email: "", enabled: true },
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
            <span>{t("nodeEditor.serverSection")}</span>
            <Select
              size="small"
              value={previewCore ?? "xray"}
              onChange={(event) => setPreviewCore(event.target.value as ServerCore)}
              sx={{ minWidth: 150 }}
            >
              <MenuItem value="xray">{t("core.xray")}</MenuItem>
              <MenuItem value="hysteria2">{t("core.hysteria2")}</MenuItem>
            </Select>
          </Stack>
        </DialogTitle>
        <DialogContent dividers>
          {nodeId === null ? (
            <Alert severity="info">{t("nodeEditor.previewNeedSave")}</Alert>
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
          <CopyButton value={previewText} label={t("nodeEditor.copyConfig")} />
          <Button onClick={() => setPreviewCore(null)}>{t("nodeEditor.close")}</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}
