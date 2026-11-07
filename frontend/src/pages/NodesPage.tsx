/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useCallback, useEffect, useState } from "react";
import {
  Alert,
  Box,
  Button,
  Card,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  MenuItem,
  Snackbar,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { Add, Refresh } from "@mui/icons-material";
import { Link as RouterLink } from "react-router-dom";
import { useTranslation } from "react-i18next";

import {
  deleteNode,
  getApiErrorMessage,
  renderServerConfig,
  setNodeEnabled,
} from "../api";
import { CopyButton, MonoBlock } from "../components/CopyButton";
import NodeCard from "../components/NodeCard";
import PageHeader from "../components/PageHeader";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { loadCatalog, nodeUpdated, nodesReplaced } from "../store/catalogSlice";
import type { Node, ServerCore } from "../types";

export default function NodesPage() {
  const { t } = useTranslation();
  const dispatch = useAppDispatch();
  const nodes = useAppSelector((state) => state.catalog.nodes);
  const loading = useAppSelector((state) => state.catalog.loading);
  const error = useAppSelector((state) => state.catalog.error);

  const [preview, setPreview] = useState<{ node: Node; core: ServerCore } | null>(null);
  const [previewText, setPreviewText] = useState("");
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<Node | null>(null);
  const [toast, setToast] = useState("");
  const [toggling, setToggling] = useState<ReadonlySet<number>>(new Set());

  const closePreview = useCallback(() => {
    setPreview(null);
    setPreviewBusy(true);
    setPreviewError(null);
    setPreviewText("");
  }, []);

  function openPreview(node: Node) {
    setPreviewBusy(true);
    setPreviewError(null);
    setPreviewText("");
    setPreview({ node, core: node.server_params?.core ?? "xray" });
  }

  useEffect(() => {
    void dispatch(loadCatalog());
  }, [dispatch]);

  useEffect(() => {
    if (!preview) {
      return;
    }
    let cancelled = false;
    renderServerConfig(preview.node.id, preview.core)
      .then((document) => {
        if (!cancelled) {
          setPreviewText(document.content);
          setPreviewError(null);
          setPreviewBusy(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setPreviewError(getApiErrorMessage(err, t("nodes.renderFailed")));
          setPreviewBusy(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [preview, t]);

  async function handleToggle(node: Node, enabled: boolean) {
    setToggling((current) => new Set(current).add(node.id));
    try {
      const updated = await setNodeEnabled(node.id, enabled);
      dispatch(nodeUpdated(updated));
      setToast(
        t("nodes.toggleToast", {
          name: node.name,
          status: t(enabled ? "nodes.statusEnabled" : "nodes.statusDisabled"),
        }),
      );
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, t("nodes.toggleFailed")) }),
      );
    } finally {
      setToggling((current) => {
        const next = new Set(current);
        next.delete(node.id);
        return next;
      });
    }
  }

  async function handleDelete(node: Node) {
    setConfirmDelete(null);
    try {
      await deleteNode(node.id);
      dispatch(nodesReplaced(nodes.filter((item) => item.id !== node.id)));
      setToast(t("nodes.deleteSuccess", { name: node.name }));
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, t("nodes.deleteFailed")) }),
      );
    }
  }

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: t("nodes.breadcrumb") }]}
        title={t("nodes.title")}
        description={t("nodes.description")}
        actions={
          <>
            <Button
              startIcon={<Refresh sx={{ fontSize: 16 }} />}
              variant="outlined"
              onClick={() => void dispatch(loadCatalog())}
            >
              {t("common.refresh")}
            </Button>
            <Button
              startIcon={<Add sx={{ fontSize: 16 }} />}
              variant="contained"
              component={RouterLink}
              to="/nodes/new"
            >
              {t("nodes.addNode")}
            </Button>
          </>
        }
      />

      {error && (
        <Alert severity="error" sx={{ mb: 2 }}>
          {error}
        </Alert>
      )}

      {loading && (
        <Box sx={{ display: "grid", placeItems: "center", py: 4 }}>
          <CircularProgress size={22} />
        </Box>
      )}
      {!loading && nodes.length === 0 && (
        <Card>
          <Typography align="center" sx={{ py: 4, color: "text.secondary" }}>
            {t("nodes.noNodes")}
          </Typography>
        </Card>
      )}
      <Box sx={{ display: "grid", gap: 2, gridTemplateColumns: { xs: "1fr", sm: "repeat(2, minmax(0, 1fr))", lg: "repeat(3, minmax(0, 1fr))" } }}>
        {nodes.map((node) => (
          <NodeCard
            key={node.id}
            node={node}
            toggling={toggling.has(node.id)}
            onToggle={(enabled) => void handleToggle(node, enabled)}
            onPreview={() => openPreview(node)}
            onDelete={() => setConfirmDelete(node)}
          />
        ))}
      </Box>

      <Dialog open={Boolean(preview)} onClose={closePreview} fullWidth maxWidth="md">
        <DialogTitle>
          <Stack direction="row" spacing={1.5} sx={{ alignItems: "center" }}>
            <Box sx={{ flexGrow: 1 }}>
              {t("nodes.serverConfigTitle", { name: preview?.node.name ?? "" })}
            </Box>
            <TextField
              select
              size="small"
              label={t("nodes.core")}
              value={preview?.core ?? "xray"}
              onChange={(event) => {
                const core = event.target.value as ServerCore;
                setPreview((current) => (current ? { ...current, core } : current));
                setPreviewBusy(true);
                setPreviewError(null);
                setPreviewText("");
              }}
              sx={{ width: 150 }}
            >
              <MenuItem value="xray">{t("core.xray")}</MenuItem>
              <MenuItem value="hysteria2">{t("core.hysteria2")}</MenuItem>
            </TextField>
          </Stack>
        </DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2}>
            {previewError && <Alert severity="error">{previewError}</Alert>}
            {previewBusy ? (
              <Box sx={{ display: "grid", placeItems: "center", py: 4 }}>
                <CircularProgress size={24} />
              </Box>
            ) : (
              previewText && <MonoBlock value={previewText} maxHeight={420} />
            )}
          </Stack>
        </DialogContent>
        <DialogActions>
          <CopyButton value={previewText} label={t("nodeEditor.copyConfig")} />
          <Button
            onClick={() => {
              if (!previewText || !preview) {
                return;
              }
              const blob = new Blob([previewText], { type: "text/plain;charset=utf-8" });
              const url = URL.createObjectURL(blob);
              const anchor = document.createElement("a");
              anchor.href = url;
              anchor.download = `${preview.node.name}-${preview.core}.txt`;
              anchor.click();
              URL.revokeObjectURL(url);
            }}
          >
            {t("common.download")}
          </Button>
          <Button onClick={closePreview}>{t("common.close")}</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={Boolean(confirmDelete)} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>{t("nodes.deleteConfirmTitle")}</DialogTitle>
        <DialogContent>
          <Typography>
            {t("nodes.deleteConfirmMessage", { name: confirmDelete?.name ?? "" })}
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)}>{t("common.cancel")}</Button>
          <Button
            color="error"
            variant="contained"
            onClick={() => confirmDelete && void handleDelete(confirmDelete)}
          >
            {t("common.delete")}
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar
        open={toast.length > 0}
        autoHideDuration={2500}
        onClose={() => setToast("")}
        message={toast}
      />
    </Box>
  );
}
