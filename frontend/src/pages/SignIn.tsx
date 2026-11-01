import { useState } from "react";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  IconButton,
  InputAdornment,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import { Visibility, VisibilityOff } from "@mui/icons-material";
import { useNavigate } from "react-router-dom";

import { getApiErrorMessage } from "../api";
import ThemeToggle from "../components/ThemeToggle";
import { loginAction, setupAction } from "../store/authSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { Wordmark } from "../components/Wordmark";

export default function SignIn({ mode }: { mode: "login" | "setup" }) {
  const dispatch = useAppDispatch();
  const navigate = useNavigate();
  const error = useAppSelector((state) => state.auth.error);

  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) {
      return;
    }

    setBusy(true);
    setLocalError(null);
    try {
      const action = mode === "setup" ? setupAction : loginAction;
      await dispatch(action({ username: username.trim(), password })).unwrap();
      navigate("/", { replace: true });
    } catch (err) {
      setLocalError(getApiErrorMessage(err, mode === "setup" ? "初始化失败" : "登录失败"));
    } finally {
      setBusy(false);
    }
  }

  const message = localError ?? error;

  return (
    <Box
      sx={{
        minHeight: "100dvh",
        display: "grid",
        gridTemplateColumns: { xs: "1fr", md: "1fr 1fr" },
      }}
    >
      {/* Editorial half: hidden on small screens so the form stays the focus. */}
      <Box
        sx={{
          display: { xs: "none", md: "flex" },
          flexDirection: "column",
          justifyContent: "space-between",
          p: 6,
          borderRight: "1px solid",
          borderColor: "divider",
          backgroundColor: "background.paper",
        }}
      >
        <Wordmark />
        <Box>
          <Typography variant="h1" sx={{ mb: 2 }}>
            订阅分发，
            <br />
            只管节点。
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ maxWidth: "36ch" }}>
            结构化保存每个节点的服务端与客户端配置，按用户下发独立凭据，并为 v2rayN、
            sing-box、Mihomo、Xray、Hysteria2 生成可直接使用的配置。
          </Typography>
        </Box>
        <Typography variant="caption" color="text.disabled">
          NSM · NeoSubscriptionMastodon
        </Typography>
      </Box>

      <Box sx={{ display: "flex", flexDirection: "column", minHeight: "100dvh" }}>
        <Stack direction="row" sx={{ justifyContent: "flex-end", p: 2 }}>
          <ThemeToggle />
        </Stack>
        <Box sx={{ flexGrow: 1, display: "grid", placeItems: "center", px: 3, pb: 8 }}>
          <Box sx={{ width: "100%", maxWidth: 340 }}>
            <Typography variant="h3" component="h1" sx={{ mb: 1 }}>
              {mode === "setup" ? "初始化管理员" : "登录"}
            </Typography>
            <Typography variant="body2" color="text.secondary" sx={{ mb: 4 }}>
              {mode === "setup"
                ? "首次启动，创建唯一的超级管理员账户。"
                : "使用管理员账户继续。"}
            </Typography>

            <form onSubmit={handleSubmit}>
              <Stack spacing={2.5}>
                <TextField
                  label="用户名"
                  value={username}
                  onChange={(event) => setUsername(event.target.value)}
                  autoComplete="username"
                  required
                  fullWidth
                  size="small"
                />
                <TextField
                  label="密码"
                  type={show ? "text" : "password"}
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  autoComplete={mode === "setup" ? "new-password" : "current-password"}
                  required
                  fullWidth
                  size="small"
                  helperText={mode === "setup" ? "至少 12 个字符" : undefined}
                  slotProps={{
                    input: {
                      endAdornment: (
                        <InputAdornment position="end">
                          <IconButton
                            size="small"
                            onClick={() => setShow((value) => !value)}
                            aria-label={show ? "隐藏密码" : "显示密码"}
                          >
                            {show ? (
                              <VisibilityOff sx={{ fontSize: 18 }} />
                            ) : (
                              <Visibility sx={{ fontSize: 18 }} />
                            )}
                          </IconButton>
                        </InputAdornment>
                      ),
                    },
                  }}
                />
                {message && <Alert severity="error">{message}</Alert>}
                <Button
                  type="submit"
                  variant="contained"
                  size="large"
                  disabled={busy}
                  startIcon={busy ? <CircularProgress size={14} color="inherit" /> : undefined}
                  sx={{ mt: 1 }}
                >
                  {mode === "setup" ? "创建并进入" : "登录"}
                </Button>
              </Stack>
            </form>
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
