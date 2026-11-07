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
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Snackbar,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { Add, Refresh } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

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
import UserCard from "../components/UserCard";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { loadCatalog } from "../store/catalogSlice";
import { isEmpty, validateUserForm, type FieldErrors } from "../validation";
import type { ProvisionedUser, User } from "../types";

export default function UsersPage() {
  const { t } = useTranslation();
  const dispatch = useAppDispatch();
  const users = useAppSelector((state) => state.catalog.users);
  const loading = useAppSelector((state) => state.catalog.loading);

  const [createOpen, setCreateOpen] = useState(false);
  const [remark, setRemark] = useState("");
  const [slug, setSlug] = useState("");
  // Row being renamed inline: the user and the slug draft.
  const [editing, setEditing] = useState<{ user: User; draft: string } | null>(null);
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
    const errors = validateUserForm({ remark, token: slug });
    if (!isEmpty(errors)) {
      setCreateErrors(errors);
      setDialogError(Object.values(errors)[0] ?? t("users.checkForm"));
      return;
    }

    setCreateErrors({});
    setDialogError(null);
    setBusy(true);
    try {
      const created = await createUser({
        remark: remark.trim(),
        enabled: true,
        // An empty slug lets the server generate one.
        token: slug.trim() || undefined,
      });
      setProvisioned(created);
      setCreateOpen(false);
      setRemark("");
      setSlug("");
      await refreshUsers();
    } catch (err) {
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setCreateErrors(serverErrors);
      }
      setDialogError(getApiErrorMessage(err, t("users.createFailed")));
    } finally {
      setBusy(false);
    }
  }

  /** Saves an inline rename of the subscription slug. */
  async function handleRename() {
    if (!editing) {
      return;
    }
    const next = editing.draft.trim();
    if (next === editing.user.token) {
      setEditing(null);
      return;
    }

    const errors = validateUserForm({ remark: editing.user.remark, token: next });
    if (errors.token) {
      dispatch(noticeAdded({ severity: "error", message: errors.token }));
      return;
    }

    try {
      await updateUser(editing.user.id, { token: next });
      await refreshUsers();
      setEditing(null);
      setToast(t("users.pathUpdated"));
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, t("users.updateFailed")) }),
      );
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
      setToast(t(user.enabled ? "users.userDisabled" : "users.userEnabled"));
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, t("users.updateFailed")) }),
      );
    }
  }

  async function handleReset(user: User) {
    try {
      const reset = await resetUserLink(user.id);
      setProvisioned(reset);
      await refreshUsers();
      setToast(t("users.resetLinkSuccess"));
    } catch (err) {
      dispatch(
        noticeAdded({
          severity: "error",
          message: getApiErrorMessage(err, t("users.resetLinkFailed")),
        }),
      );
    }
  }

  async function handleDelete(user: User) {
    setConfirmDelete(null);
    try {
      await deleteUser(user.id);
      await refreshUsers();
      setToast(t("users.deleteSuccess", { name: userLabel(user) }));
    } catch (err) {
      dispatch(
        noticeAdded({ severity: "error", message: getApiErrorMessage(err, t("users.deleteFailed")) }),
      );
    }
  }

  const subscriptionUrl = provisioned
    ? `${window.location.origin}${provisioned.subscription_path}`
    : "";

  /** Full link for a user row, built from the path the server reports. */
  function userLink(user: User) {
    return `${window.location.origin}${user.subscription_path}`;
  }

  /** Name shown for a user row, or for the user a link was generated for. */
  function userLabel(user: { id: number; remark: string }) {
    return user.remark || `#${user.id}`;
  }

  /** `linkPrefix` is everything before the slug, for the inline editor. */
  function slugPrefix(user: User) {
    return user.subscription_path.slice(0, user.subscription_path.length - user.token.length);
  }

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: t("users.breadcrumb") }]}
        title={t("users.title")}
        description={t("users.subscriberOnlyNotice")}
        actions={
          <>
            <Button
              startIcon={<Refresh sx={{ fontSize: 16 }} />}
              variant="outlined"
              onClick={() => void refreshUsers()}
            >
              {t("common.refresh")}
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
              {t("users.addUser")}
            </Button>
          </>
        }
      />

      {loading && (
        <Box sx={{ display: "grid", placeItems: "center", py: 4 }}>
          <CircularProgress size={22} />
        </Box>
      )}
      {!loading && users.length === 0 && (
        <Card>
          <Typography align="center" sx={{ py: 4, color: "text.secondary" }}>
            {t("users.noUsers")}
          </Typography>
        </Card>
      )}
      <Box sx={{ display: "grid", gap: 2, gridTemplateColumns: { xs: "1fr", sm: "repeat(2, minmax(0, 1fr))", lg: "repeat(3, minmax(0, 1fr))" } }}>
        {users.map((user) => (
          <UserCard
            key={user.id}
            user={user}
            label={userLabel(user)}
            link={userLink(user)}
            prefix={slugPrefix(user)}
            draft={editing?.user.id === user.id ? editing.draft : null}
            onEdit={() => setEditing({ user, draft: user.token })}
            onDraftChange={(draft) => setEditing({ user, draft })}
            onSave={() => void handleRename()}
            onCancel={() => setEditing(null)}
            onToggle={() => void handleToggle(user)}
            onReset={() => void handleReset(user)}
            onDelete={() => setConfirmDelete(user)}
          />
        ))}
      </Box>

      <Dialog open={createOpen} onClose={closeCreateDialog} fullWidth maxWidth="sm">
        <DialogTitle>{t("users.createUserTitle")}</DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2}>
            {dialogError && <Alert severity="error">{dialogError}</Alert>}
            <TextField
              label={t("users.remark")}
              value={remark}
              onChange={(event) => setRemark(event.target.value)}
              error={Boolean(createErrors.remark)}
              helperText={createErrors.remark ?? t("users.remarkHelper")}
              fullWidth
            />
            <TextField
              label={t("users.subscriptionPath")}
              value={slug}
              onChange={(event) => setSlug(event.target.value)}
              error={Boolean(createErrors.token)}
              helperText={createErrors.token ?? t("users.subscriptionPathHelper")}
              fullWidth
            />
            <Alert severity="info">{t("users.subscriberOnlyNotice")}</Alert>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={closeCreateDialog}>{t("common.cancel")}</Button>
          <Button variant="contained" disabled={busy} onClick={() => void handleCreate()}>
            {t("users.createUser")}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog
        open={Boolean(provisioned)}
        onClose={() => setProvisioned(null)}
        fullWidth
        maxWidth="sm"
      >
        <DialogTitle>
          {provisioned
            ? t("users.provisionedTitle", { name: userLabel(provisioned.user) })
            : t("users.subscriptionLink")}
        </DialogTitle>
        <DialogContent dividers>
          <Stack spacing={2}>
            <Box>
              <Typography variant="caption" color="text.secondary">
                {t("users.defaultFormat")}
              </Typography>
              <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
                <MonoBlock value={subscriptionUrl} maxHeight={80} />
                <CopyButton value={subscriptionUrl} size="medium" />
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
              {t("users.targetHint")}
            </Typography>
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setProvisioned(null)}>{t("users.done")}</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={Boolean(confirmDelete)} onClose={() => setConfirmDelete(null)}>
        <DialogTitle>{t("users.confirmDelete")}</DialogTitle>
        <DialogContent>
          <Typography>
            {t("users.confirmDeleteMessage", { name: confirmDelete?.remark ?? "" })}
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

