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
  Chip,
  Divider,
  FormControlLabel,
  IconButton,
  InputAdornment,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { Logout, LockOutlined, Visibility, VisibilityOff } from "@mui/icons-material";
import { useTranslation } from "react-i18next";

import {
  changePassword,
  getApiErrorMessage,
  getApiFieldErrors,
  getSettings,
  updateDisguisePage,
  updateSettings,
} from "../api";
import AccentPicker from "../components/AccentPicker";
import LanguageSelect from "../components/LanguageSelect";
import PageHeader from "../components/PageHeader";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch } from "../store/hooks";
import { isEmpty, validateSettingsForm, type FieldErrors } from "../validation";
import type { Settings } from "../types";

export default function SettingsPage({ onLogout }: { onLogout: () => void }) {
  const { t } = useTranslation();
  const dispatch = useAppDispatch();

  const [settings, setSettings] = useState<Settings | null>(null);
  const [prefix, setPrefix] = useState("/s");
  const [siteName, setSiteName] = useState("NSM");
  const [target, setTarget] = useState("uri");
  // Set when `config.yml` pins the prefix: the database value is ignored, so the
  // field shows the pinned value read-only instead of pretending to be editable.
  const [prefixPinned, setPrefixPinned] = useState(false);
  // Kept as a string so a cleared input does not turn into NaN on submit.
  const [delay, setDelay] = useState("0");
  const [disguise, setDisguise] = useState("");
  const [passthrough, setPassthrough] = useState(false);
  const [templateUrl, setTemplateUrl] = useState("");
  const [templateIp, setTemplateIp] = useState("");
  // Kept as a string for the same reason as `delay`.
  const [templatePort, setTemplatePort] = useState("0");
  const [saving, setSaving] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});

  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [changing, setChanging] = useState(false);

  useEffect(() => {
    void getSettings()
      .then((value) => {
        setSettings(value);
        setPrefix(value.subscription_prefix);
        setPrefixPinned(value.subscription_prefix_pinned);
        setSiteName(value.site_name);
        setTarget(value.default_target);
        setDelay(String(value.failure_delay_ms));
        setDisguise(value.disguise_page);
        setPassthrough(value.disguise_passthrough);
        setTemplateUrl(value.disguise_template_url);
        setTemplateIp(value.disguise_template_ip);
        setTemplatePort(String(value.disguise_template_port));
      })
      .catch((err) =>
        dispatch(
          noticeAdded({
            severity: "error",
            message: getApiErrorMessage(err, t("settings.loadFailed")),
          }),
        ),
      );
  }, [dispatch, t]);

  async function handleSave() {
    const errors = validateSettingsForm({
      subscription_prefix: prefix,
      default_target: target,
      site_name: siteName,
      failure_delay_ms: delay,
      disguise_passthrough: passthrough,
      disguise_template_url: templateUrl,
      disguise_template_ip: templateIp,
      disguise_template_port: templatePort,
    });
    if (!isEmpty(errors)) {
      setFieldErrors(errors);
      dispatch(
        noticeAdded({
          severity: "error",
          message: Object.values(errors)[0] ?? t("common.checkForm"),
        }),
      );
      return;
    }

    setFieldErrors({});
    setSaving(true);
    try {
      const updated = await updateSettings({
        // A pinned prefix cannot be changed from the UI; omitting it leaves the
        // config file authoritative and avoids a pointless rejected write.
        subscription_prefix: prefixPinned ? undefined : prefix.trim(),
        site_name: siteName.trim(),
        default_target: target,
        failure_delay_ms: Number(delay || "0"),
        disguise_passthrough: passthrough,
        disguise_template_url: templateUrl.trim(),
        disguise_template_ip: templateIp.trim(),
        disguise_template_port: Number(templatePort || "0"),
      });
      setSettings(updated);
      setPrefix(updated.subscription_prefix);
      setPrefixPinned(updated.subscription_prefix_pinned);
      setSiteName(updated.site_name);
      setDelay(String(updated.failure_delay_ms));
      setPassthrough(updated.disguise_passthrough);
      setTemplateUrl(updated.disguise_template_url);
      setTemplateIp(updated.disguise_template_ip);
      setTemplatePort(String(updated.disguise_template_port));
      dispatch(noticeAdded({ severity: "success", message: t("settings.saveSuccess") }));
    } catch (err) {
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setFieldErrors(serverErrors);
      }
      dispatch(
        noticeAdded({
          severity: "error",
          message: getApiErrorMessage(err, t("settings.saveFailed")),
        }),
      );
    } finally {
      setSaving(false);
    }
  }

  async function handleSaveDisguise() {
    setSaving(true);
    try {
      const updated = await updateDisguisePage(disguise);
      setSettings(updated);
      setDisguise(updated.disguise_page);
      dispatch(noticeAdded({ severity: "success", message: t("settings.disguiseSaved") }));
    } catch (err) {
      dispatch(
        noticeAdded({
          severity: "error",
          message: getApiErrorMessage(err, t("settings.saveFailed")),
        }),
      );
    } finally {
      setSaving(false);
    }
  }

  async function handlePasswordChange() {
    setChanging(true);
    try {
      await changePassword(currentPassword, newPassword);
      dispatch(
        noticeAdded({ severity: "success", message: t("settings.passwordChangeSuccess") }),
      );
      onLogout();
    } catch (err) {
      dispatch(
        noticeAdded({
          severity: "error",
          message: getApiErrorMessage(err, t("settings.passwordChangeFailed")),
        }),
      );
    } finally {
      setChanging(false);
    }
  }

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: t("settings.breadcrumb") }]}
        title={t("settings.title")}
        description={t("settings.description")}
      />

      <Stack spacing={2}>
        <Card>
          <CardContent>
            <Stack
              direction="row"
              spacing={1}
              sx={{ alignItems: "center", justifyContent: "space-between", mb: 2 }}
            >
              <Typography variant="subtitle2" color="text.secondary">
                {t("settings.appearance")}
              </Typography>
              <Chip size="small" variant="outlined" label={t("settings.appearanceLocal")} />
            </Stack>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t("theme.accentHelper")}
            </Typography>
            <AccentPicker />
            <Divider sx={{ my: 2 }} />
            <Stack direction="row" spacing={2} sx={{ alignItems: "center", flexWrap: "wrap" }}>
              <LanguageSelect size="medium" />
              <Typography variant="caption" color="text.disabled">
                {t("settings.languageHelper")}
              </Typography>
            </Stack>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              {t("settings.subscription")}
            </Typography>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                label={t("settings.subscriptionPrefix")}
                value={prefix}
                onChange={(event) => setPrefix(event.target.value)}
                disabled={prefixPinned}
                error={Boolean(fieldErrors.subscription_prefix)}
                slotProps={{
                  input: prefixPinned
                    ? {
                        endAdornment: (
                          <InputAdornment position="end">
                            <LockOutlined sx={{ fontSize: 16 }} />
                          </InputAdornment>
                        ),
                      }
                    : undefined,
                }}
                helperText={
                  prefixPinned
                    ? t("settings.subscriptionPrefixPinned")
                    : (fieldErrors.subscription_prefix ?? t("settings.subscriptionPrefixHelper"))
                }
              />
              <TextField
                size="small"
                label={t("settings.siteName")}
                value={siteName}
                onChange={(event) => setSiteName(event.target.value)}
                error={Boolean(fieldErrors.site_name)}
                helperText={fieldErrors.site_name}
              />
              <TextField
                size="small"
                select
                label={t("settings.defaultTarget")}
                value={target}
                onChange={(event) => setTarget(event.target.value)}
                error={Boolean(fieldErrors.default_target)}
                helperText={fieldErrors.default_target ?? t("settings.defaultTargetHelper")}
                SelectProps={{ native: true }}
              >
                {(settings?.available_cores ?? ["uri", "sing-box", "mihomo", "xray", "hysteria2"]).map(
                  (core) => (
                    <option key={core} value={core}>
                      {core}
                    </option>
                  ),
                )}
              </TextField>
              <TextField
                size="small"
                type="number"
                label={t("settings.failureDelay")}
                value={delay}
                onChange={(event) => setDelay(event.target.value)}
                error={Boolean(fieldErrors.failure_delay_ms)}
                helperText={fieldErrors.failure_delay_ms ?? t("settings.failureDelayHelper")}
                slotProps={{ htmlInput: { min: 0, max: 10000 } }}
              />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              {t("settings.disguisePage")}
            </Typography>
            <TextField
              fullWidth
              multiline
              minRows={8}
              label={t("settings.disguisePageField")}
              value={disguise}
              onChange={(event) => setDisguise(event.target.value)}
              helperText={t("settings.disguisePageHelper")}
            />
            <Box sx={{ mt: 2 }}>
              <Button variant="contained" disabled={saving} onClick={() => void handleSaveDisguise()}>
                {t("settings.saveDisguisePage")}
              </Button>
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              {t("settings.disguisePassthrough")}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              {t("settings.disguisePassthroughHelper")}
            </Typography>
            <FormControlLabel
              control={
                <Switch
                  checked={passthrough}
                  onChange={(event) => setPassthrough(event.target.checked)}
                />
              }
              label={t("settings.enablePassthrough")}
              sx={{ mb: 2 }}
            />
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                label={t("settings.disguiseTemplateUrl")}
                placeholder="https://sample.example.com/"
                value={templateUrl}
                onChange={(event) => setTemplateUrl(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_url)}
                helperText={
                  fieldErrors.disguise_template_url ?? t("settings.disguiseTemplateUrlHelper")
                }
                sx={{ gridColumn: { sm: "1 / -1" } }}
              />
              <TextField
                size="small"
                label={t("settings.disguiseTemplateIp")}
                placeholder="host.docker.internal"
                value={templateIp}
                onChange={(event) => setTemplateIp(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_ip)}
                helperText={fieldErrors.disguise_template_ip ?? t("settings.disguiseTemplateIpHelper")}
              />
              <TextField
                size="small"
                type="number"
                label={t("settings.disguiseTemplatePort")}
                value={templatePort}
                onChange={(event) => setTemplatePort(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_port)}
                helperText={
                  fieldErrors.disguise_template_port ?? t("settings.disguiseTemplatePortHelper")
                }
                slotProps={{ htmlInput: { min: 0, max: 65535 } }}
              />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              {t("settings.adminPassword")}
            </Typography>
            <Alert severity="info" sx={{ mb: 2 }}>
              {t("settings.adminPasswordNotice")}
            </Alert>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                type="password"
                label={t("settings.currentPassword")}
                value={currentPassword}
                onChange={(event) => setCurrentPassword(event.target.value)}
              />
              <TextField
                size="small"
                type={showPassword ? "text" : "password"}
                label={t("settings.newPassword")}
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
                helperText={t("settings.passwordHint")}
                slotProps={{
                  input: {
                    endAdornment: (
                      <InputAdornment position="end">
                        <IconButton
                          size="small"
                          onClick={() => setShowPassword((value) => !value)}
                        >
                          {showPassword ? (
                            <VisibilityOff fontSize="small" />
                          ) : (
                            <Visibility fontSize="small" />
                          )}
                        </IconButton>
                      </InputAdornment>
                    ),
                  },
                }}
              />
            </Box>
            <Box sx={{ mt: 2 }}>
              <Button
                variant="outlined"
                disabled={changing || !currentPassword || newPassword.length < 12}
                onClick={() => void handlePasswordChange()}
              >
                {t("settings.changePassword")}
              </Button>
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Stack direction="row" sx={{ alignItems: "center", justifyContent: "space-between" }}>
              <Box>
                <Typography variant="subtitle2">{t("settings.signOutTitle")}</Typography>
                <Typography variant="body2" color="text.secondary">
                  {t("settings.signOutDescription")}
                </Typography>
              </Box>
              <Button color="error" startIcon={<Logout />} onClick={onLogout}>
                {t("auth.signOut")}
              </Button>
            </Stack>
            <Divider sx={{ my: 2 }} />
            <Typography variant="caption" color="text.secondary">
              {settings
                ? t("settings.prefixFooter", { prefix: settings.subscription_prefix })
                : t("app.shortName")}
            </Typography>
          </CardContent>
        </Card>

        <Box sx={{ display: "flex", justifyContent: "flex-end", pt: 1 }}>
          <Button variant="contained" disabled={saving} onClick={() => void handleSave()}>
            {t("settings.saveSettings")}
          </Button>
        </Box>
      </Stack>
    </Box>
  );
}
