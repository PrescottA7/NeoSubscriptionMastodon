import { useEffect, useState } from "react";
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
  IconButton,
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
import { Add, Delete, Link as LinkIcon, Refresh } from "@mui/icons-material";

import {
  createUser,
  deleteUser,
  getApiErrorMessage,
  getApiFieldErrors,
  resetUserLink,
  updateUser,
} from "../api";
import { CopyButton, MonoBlock } from "../components/CopyButton";
import PageHeader from "../components/PageHeader";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { loadCatalog } from "../store/catalogSlice";
import { isEmpty, validateUserForm, type FieldErrors } from "../validation";
import type { ProvisionedUser, User } from "../types";

export default function UsersPage() {
  const dispatch = useAppDispatch();
  const users = useAppSelector((state) => state.catalog.users);
  const loading = useAppSelector((state) => state.catalog.loading);

  const [createOpen, setCreateOpen] = useState(false);
  const [remark, setRemark] = useState("");
  const [provisioned, setProvisioned] = useState<ProvisionedUser | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<User | null>(null);
  const [busy, setBusy] = useState(false);
  const [toast, setToast] = useState("");
  const [createErrors, setCreateErrors] = useState<FieldErrors>({});
  const [dialogError, setDialogError] = useState<string | null>(null);

  useEffect(() => {
    void dispatch(loadCatalog());
  }, [dispatch]);

  async function refreshUsers() {
    await dispatch(loadCatalog());
  }

  async function handleCreate() {
    const errors = validateUserForm({ remark });
    if (!isEmpty(errors)) {
      setCreateErrors(errors);
      setDialogError(Object.values(errors)[0] ?? "请检查表单");
      return;
    }

    setCreateErrors({});
    setDialogError(null);
    setBusy(true);
    try {
      const created = await createUser({
        remark: remark.trim(),
        enabled: true,
      });
      setProvisioned(created);
      setCreateOpen(false);
      setRemark("");
      await refreshUsers();
    } catch (err) {
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setCreateErrors(serverErrors);
      }
      setDialogError(getApiErrorMessage(err, "创建失败"));
    } finally {
      setBusy(false);
    }
  }

  function closeCreateDialog() {
    setCreateOpen(false);
    setCreateErrors({});
    setDialogError(null);
  }

  async function handleToggle(user: User) {
    try {
      await updateUser(user.id, { enabled: !user.enabled });
      await refreshUsers();
      setToast(user.enabled ? "已停用用户" : "已启用用户");
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "更新失败") }));
    }
  }

  async function handleReset(user: User) {
    try {
      const reset = await resetUserLink(user.id);
      setProvisioned(reset);
      await refreshUsers();
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "重置失败") }));
    }
  }

  async function handleDelete(user: User) {
    setConfirmDelete(null);
    try {
      await deleteUser(user.id);
      await refreshUsers();
      setToast(`已删除用户 ${user.remark}`);
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "删除失败") }));
    }
  }

  const subscriptionUrl = provisioned
    ? `${window.location.origin}${provisioned.subscription_path}`
    : "";

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: "用户" }]}
        title="用户"
        description="用户只是订阅消费者，不持有代理凭据；订阅链接仅在创建或重置时显示一次。"
        actions={
          <>
            <Button
              startIcon={<Refresh sx={{ fontSize: 16 }} />}
              variant="outlined"
              onClick={() => void refreshUsers()}
            >
              刷新
            </Button>
            <Button
              startIcon={<Add sx={{ fontSize: 16 }} />}
              variant="contained"
              onClick={() => {
                setCreateErrors({});
                setDialogError(null);
                setCreateOpen(true);
              }}
            >
              新建用户
            </Button>
          </>
        }
      />

      <Card>
        <TableContainer>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>用户名</TableCell>
                <TableCell>备注</TableCell>
                <TableCell>订阅链接</TableCell>
                <TableCell>访问次数</TableCell>
                <TableCell>状态</TableCell>
                <TableCell align="right">操作</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {loading && <UsersLoadingRow />}
              {!loading && users.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: "text.secondary" }}>
                    还没有用户，点击「新建用户」创建第一个订阅者。
                  </TableCell>
                </TableRow>
              )}
              {users.map((user) => (
                <TableRow key={user.id} hover>
                  <TableCell>{user.remark}</TableCell>
                  <TableCell>{user.remark || "—"}</TableCell>
                  <TableCell sx={{ fontFamily: "monospace" }}>…{user.token_hint}</TableCell>
                  <TableCell>{user.access_count}</TableCell>
                  <TableCell>
                    <Switch
                      size="small"
                      checked={user.enabled}
                      onChange={() => void handleToggle(user)}
                    />
                  </TableCell>
                  <TableCell align="right">
                    <Tooltip title="重置订阅链接">
                      <IconButton size="small" onClick={() => void handleReset(user)} aria-label="重置链接">
                        <LinkIcon fontSize="small" />
                      </IconButton>
                    </Tooltip>
                    <Tooltip title="删除">
                      <IconButton
                        size="small"
                        color="error"
                        onClick={() => setConfirmDelete(user)}
                        aria-label="删除用户"
                      >
                        <Delete fontSize="small" />
                      </IconButton>
                    </Tooltip>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
      </Card>

      <Dialog open={createOpen} onClose={closeCreateDialog} fullWidth maxWidth="sm">
        <DialogTitle>新建用户</DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2}>
            {dialogError && <Alert severity="error">{dialogError}</Alert>}
            <TextField
              label="备注"
              value={remark}
              onChange={(event) => setRemark(event.target.value)}
              error={Boolean(createErrors.remark)}
              helperText={
                createErrors.remark ?? "用于在客户端中区分订阅名称，也会用作节点账号标签"
              }
              fullWidth
            />
            <Alert severity="info">
              用户只是订阅者，不持有代理凭据。每个节点的账号在节点编辑页单独配置。
            </Alert>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={closeCreateDialog}>取消</Button>
          <Button variant="contained" disabled={busy} onClick={() => void handleCreate()}>
            创建
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={Boolean(provisioned)} onClose={() => setProvisioned(null)} fullWidth maxWidth="sm">
        <DialogTitle>订阅链接</DialogTitle>
        <DialogContent dividers>
          <Alert severity="warning" sx={{ mb: 2 }}>
            链接只会显示这一次，请立即复制并妥善保存。
          </Alert>
          <Stack spacing={2}>
            <Box>
              <Typography variant="caption" color="text.secondary">
                默认格式（自动识别客户端）
              </Typography>
              <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
                <MonoBlock value={subscriptionUrl} maxHeight={80} />
                <CopyButton value={subscriptionUrl} label="复制订阅链接" size="medium" />
              </Stack>
            </Box>
            <Box>
              <Typography variant="caption" color="text.secondary">
                sing-box
              </Typography>
              <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
                <MonoBlock value={`${subscriptionUrl}?target=sing-box`} maxHeight={80} />
                <CopyButton value={`${subscriptionUrl}?target=sing-box`} />
              </Stack>
            </Box>
            <Box>
              <Typography variant="caption" color="text.secondary">
                Mihomo / Clash
              </Typography>
              <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
                <MonoBlock value={`${subscriptionUrl}?target=mihomo`} maxHeight={80} />
                <CopyButton value={`${subscriptionUrl}?target=mihomo`} />
              </Stack>
            </Box>
            <Typography variant="body2" color="text.secondary">
              支持 <code>?target=uri|sing-box|mihomo|xray|hysteria2</code>，不指定时根据
              User-Agent 自动选择。
            </Typography>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setProvisioned(null)}>完成</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={Boolean(confirmDelete)} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>删除用户</DialogTitle>
        <DialogContent>
          <Typography>
            确定删除「{confirmDelete?.remark}」吗？其订阅链接将立即失效。
          </Typography>
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

function UsersLoadingRow() {
  return (
    <TableRow>
      <TableCell colSpan={6} align="center">
        <CircularProgress size={22} sx={{ my: 2 }} />
      </TableCell>
    </TableRow>
  );
}
