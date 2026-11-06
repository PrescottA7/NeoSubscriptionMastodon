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

import {
  changePassword,
  getApiErrorMessage,
  getApiFieldErrors,
  getSettings,
  updateDisguisePage,
  updateSettings,
} from "../api";
import AccentPicker from "../components/AccentPicker";
import PageHeader from "../components/PageHeader";
import { noticeAdded } from "../store/noticesSlice";
import { useAppDispatch } from "../store/hooks";
import { isEmpty, validateSettingsForm, type FieldErrors } from "../validation";
import type { Settings } from "../types";

export default function SettingsPage({ onLogout }: { onLogout: () => void }) {
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
        dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "加载失败") })),
      );
  }, [dispatch]);

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
      dispatch(noticeAdded({ severity: "error", message: Object.values(errors)[0] ?? "请检查表单" }));
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
      dispatch(noticeAdded({ severity: "success", message: "设置已保存" }));
    } catch (err) {
      const serverErrors = getApiFieldErrors(err);
      if (!isEmpty(serverErrors)) {
        setFieldErrors(serverErrors);
      }
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "保存失败") }));
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
      dispatch(noticeAdded({ severity: "success", message: "伪装页面已更新" }));
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "保存失败") }));
    } finally {
      setSaving(false);
    }
  }

  async function handlePasswordChange() {
    setChanging(true);
    try {
      await changePassword(currentPassword, newPassword);
      dispatch(noticeAdded({ severity: "success", message: "密码已更新，请重新登录" }));
      onLogout();
    } catch (err) {
      dispatch(noticeAdded({ severity: "error", message: getApiErrorMessage(err, "修改失败") }));
    } finally {
      setChanging(false);
    }
  }

  return (
    <Box>
      <PageHeader
        crumbs={[{ label: "设置" }]}
        title="设置"
        description="伪装策略对所有公开路径生效，包括无效订阅链接与常见探测路径。"
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
                外观
              </Typography>
              <Chip size="small" variant="outlined" label="仅本浏览器生效" />
            </Stack>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              主题色用于主按钮、输入框聚焦环和选中高亮；深色模式下会自动提亮以保持可读性。
              此设置保存在本机浏览器，不影响其他设备。
            </Typography>
            <AccentPicker />
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              订阅
            </Typography>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                label="订阅路径前缀"
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
                    ? "已在 config.yml 中设置 subscription_prefix，链接使用该值；修改需编辑配置文件并重启服务"
                    : fieldErrors.subscription_prefix ?? "例如 /s，最终链接为 <前缀>/<令牌>"
                }
              />
              <TextField
                size="small"
                label="站点名称"
                value={siteName}
                onChange={(event) => setSiteName(event.target.value)}
                error={Boolean(fieldErrors.site_name)}
                helperText={fieldErrors.site_name}
              />
              <TextField
                size="small"
                select
                label="默认格式"
                value={target}
                onChange={(event) => setTarget(event.target.value)}
                error={Boolean(fieldErrors.default_target)}
                helperText={fieldErrors.default_target ?? "无法从 User-Agent 判断时使用"}
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
                label="失败响应延迟 (ms)"
                value={delay}
                onChange={(event) => setDelay(event.target.value)}
                error={Boolean(fieldErrors.failure_delay_ms)}
                helperText={fieldErrors.failure_delay_ms ?? "给 404 增加延迟，降低枚举速度"}
                slotProps={{ htmlInput: { min: 0, max: 10000 } }}
              />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              伪装页面
            </Typography>
            <TextField
              fullWidth
              multiline
              minRows={8}
              label="404 页面 HTML"
              value={disguise}
              onChange={(event) => setDisguise(event.target.value)}
              helperText="留空保存可恢复内置页面。内容会以 ISO-8859-1 与 404 状态返回。"
            />
            <Box sx={{ mt: 2 }}>
              <Button variant="contained" disabled={saving} onClick={() => void handleSaveDisguise()}>
                保存伪装页面
              </Button>
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              404 透传
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
              开启后，伪装请求会转发到样板站并原样返回它的响应，上面的静态页面不再生效；
              样板站不可达时自动回退到静态页面。
            </Typography>
            <FormControlLabel
              control={
                <Switch
                  checked={passthrough}
                  onChange={(event) => setPassthrough(event.target.checked)}
                />
              }
              label="开启 404 透传"
              sx={{ mb: 2 }}
            />
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                label="样板页地址"
                placeholder="https://sample.example.com/"
                value={templateUrl}
                onChange={(event) => setTemplateUrl(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_url)}
                helperText={fieldErrors.disguise_template_url ?? "必须是 https:// 地址"}
                sx={{ gridColumn: { sm: "1 / -1" } }}
              />
              <TextField
                size="small"
                label="服务器地址（可选）"
                placeholder="host.docker.internal"
                value={templateIp}
                onChange={(event) => setTemplateIp(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_ip)}
                helperText={fieldErrors.disguise_template_ip ?? "IP 或主机名，填写后跳过 DNS，直连该地址"}
              />
              <TextField
                size="small"
                type="number"
                label="服务器端口（可选）"
                value={templatePort}
                onChange={(event) => setTemplatePort(event.target.value)}
                error={Boolean(fieldErrors.disguise_template_port)}
                helperText={fieldErrors.disguise_template_port ?? "0 表示使用地址中的端口"}
                slotProps={{ htmlInput: { min: 0, max: 65535 } }}
              />
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Typography variant="subtitle2" color="text.secondary" sx={{ mb: 2 }}>
              管理员密码
            </Typography>
            <Alert severity="info" sx={{ mb: 2 }}>
              修改密码会立即注销所有登录会话。
            </Alert>
            <Box
              sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}
            >
              <TextField
                size="small"
                type="password"
                label="当前密码"
                value={currentPassword}
                onChange={(event) => setCurrentPassword(event.target.value)}
              />
              <TextField
                size="small"
                type={showPassword ? "text" : "password"}
                label="新密码"
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
                helperText="至少 12 个字符"
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <IconButton size="small" onClick={() => setShowPassword((value) => !value)}>
                        {showPassword ? (
                          <VisibilityOff fontSize="small" />
                        ) : (
                          <Visibility fontSize="small" />
                        )}
                      </IconButton>
                    </InputAdornment>
                  ),
                }}
              />
            </Box>
            <Box sx={{ mt: 2 }}>
              <Button
                variant="outlined"
                disabled={changing || !currentPassword || newPassword.length < 12}
                onClick={() => void handlePasswordChange()}
              >
                修改密码
              </Button>
            </Box>
          </CardContent>
        </Card>

        <Card>
          <CardContent>
            <Stack direction="row" sx={{ alignItems: "center", justifyContent: "space-between" }}>
              <Box>
                <Typography variant="subtitle2">退出登录</Typography>
                <Typography variant="body2" color="text.secondary">
                  当前设备的管理会话将立即失效。
                </Typography>
              </Box>
              <Button color="error" startIcon={<Logout />} onClick={onLogout}>
                退出
              </Button>
            </Stack>
            <Divider sx={{ my: 2 }} />
            <Typography variant="caption" color="text.secondary">
              NSM {settings ? `· 前缀 ${settings.subscription_prefix}` : ""}
            </Typography>
          </CardContent>
        </Card>

        <Box sx={{ display: "flex", justifyContent: "flex-end", pt: 1 }}>
          <Button variant="contained" disabled={saving} onClick={() => void handleSave()}>
            保存设置
          </Button>
        </Box>
      </Stack>
    </Box>
  );
}
