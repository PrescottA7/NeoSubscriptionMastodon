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
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  MenuItem,
  Snackbar,
  Stack,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Tooltip,
  Typography,
} from "@mui/material";
import { Add, Delete, Edit, Refresh } from "@mui/icons-material";
import { Link as RouterLink } from "react-router-dom";

import {
  deleteNode,
  getApiErrorMessage,
  renderServerConfig,
  setNodeEnabled,
} from "../api";
import { CopyButton, MonoBlock } from "../components/CopyButton";
import PageHeader from "../components/PageHeader";
import { SERVER_CORES, serverCore } from "../components/cores";
import { MONO_FONT } from "../theme";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { loadCatalog, nodeUpdated, nodesReplaced } from "../store/catalogSlice";
import type { Node, ServerCore } from "../types";

export default function NodesPage() {
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
    setPreview({ node, core: serverCore(node.params.protocol) });
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
          setPreviewError(getApiErrorMessage(err, "渲染失败"));
          setPreviewBusy(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [preview]);

  async function handleToggle(node: Node, enabled: boolean) {
    setToggling((current) => new Set(current).add(node.id));
    try {
      const updated = await setNodeEnabled(node.id, enabled);
      dispatch(nodeUpdated(updated));
      setToast(`已${enabled ? "启用" : "停用"}节点 ${node.name}`);
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, "切换状态失败") }),
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
      setToast(`已删除节点 ${node.name}`);
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "删除失败") }));
    }
  }

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: "节点" }]}
        title="节点"
        description="每个节点保存服务端与客户端两套配置，并按用户下发独立凭据。"
        actions={
          <>
            <Button
              startIcon={<Refresh sx={{ fontSize: 16 }} />}
              variant="outlined"
              onClick={() => void dispatch(loadCatalog())}
            >
              刷新
            </Button>
            <Button
              startIcon={<Add sx={{ fontSize: 16 }} />}
              variant="contained"
              component={RouterLink}
              to="/nodes/new"
            >
              新建节点
            </Button>
          </>
        }
      />

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <Card>
        <TableContainer>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>名称</TableCell>
                <TableCell>协议</TableCell>
                <TableCell>地址</TableCell>
                <TableCell>状态</TableCell>
                <TableCell align="right">操作</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading && (
                <TableRow>
                  <TableCell colSpan={5} align="center">
                    <CircularProgress size={22} sx={{ my: 2 }} />
                  </TableCell>
                </TableRow>
              )}
              {!loading && nodes.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: "text.secondary" }}>
                    还没有节点，点击「新建节点」开始配置。
                  </TableCell>
                </TableRow>
              )}
              {nodes.map((node) => (
                <TableRow key={node.id} hover sx={{ "& td": { whiteSpace: "nowrap" } }}>
                  <TableCell>
                    <Typography sx={{ fontWeight: 500, fontSize: "0.8125rem" }}>
                      {node.name}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip size="small" label={node.protocol} variant="outlined" />
                  </TableCell>
                  <TableCell sx={{ fontFamily: MONO_FONT, fontSize: "0.75rem" }}>
                    {node.server}:{node.port}
                  </TableCell>
                  <TableCell>
                    <FormControlLabel
                      sx={{ m: 0, gap: 0.5, "& .MuiFormControlLabel-label": { fontSize: "0.75rem" } }}
                      control={
                        <Switch
                          size="small"
                          checked={node.enabled}
                          disabled={toggling.has(node.id)}
                          onChange={(event) => void handleToggle(node, event.target.checked)}
                          inputProps={{ "aria-label": `${node.name} 启用开关` }}
                        />
                      }
                      label={node.enabled ? "启用" : "停用"}
                    />
                  </TableCell>
                  <TableCell align="right" sx={{ whiteSpace: "nowrap" }}>
                    <Stack direction="row" spacing={0.5} sx={{ justifyContent: "flex-end" }}>
                      <Button
                        size="small"
                        variant="text"
                        sx={{
                          minWidth: 0,
                          px: 1,
                          whiteSpace: "nowrap",
                          fontSize: "0.75rem",
                          color: "text.secondary",
                          "&:hover": { color: "text.primary" },
                        }}
                        onClick={() => openPreview(node)}
                      >
                        服务端配置
                      </Button>
                      <Tooltip title="编辑">
                        <IconButton
                          size="small"
                          component={RouterLink}
                          to={`/nodes/${node.id}`}
                          aria-label="编辑节点"
                        >
                          <Edit sx={{ fontSize: 16 }} />
                        </IconButton>
                      </Tooltip>
                      <Tooltip title="删除">
                        <IconButton
                          size="small"
                          onClick={() => setConfirmDelete(node)}
                          aria-label="删除节点"
                        >
                          <Delete sx={{ fontSize: 16 }} />
                        </IconButton>
                      </Tooltip>
                    </Stack>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Card>

      <Dialog open={Boolean(preview)} onClose={closePreview} fullWidth maxWidth="md">
        <DialogTitle>
          <Stack direction="row" spacing={1.5} sx={{ alignItems: "center" }}>
            <Box sx={{ flexGrow: 1 }}>服务端配置 · {preview?.node.name}</Box>
            <TextField
              select
              size="small"
              label="核心"
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
              {SERVER_CORES.map((core) => (
                <MenuItem key={core.value} value={core.value}>
                  {core.label}
                </MenuItem>
              ))}
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
          <CopyButton value={previewText} label="复制配置" />
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
            下载
          </Button>
          <Button onClick={closePreview}>关闭</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={Boolean(confirmDelete)} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>删除节点</DialogTitle>
        <DialogContent>
          <Typography>确定删除「{confirmDelete?.name}」吗？此操作不可撤销。</Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setConfirmDelete(null)}>取消</Button>
          <Button
            color="error"
            variant="contained"
            onClick={() => confirmDelete && void handleDelete(confirmDelete)}
          >
            删除
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
