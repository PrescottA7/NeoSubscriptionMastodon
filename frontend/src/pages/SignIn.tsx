/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

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
        display: "flex",
        flexDirection: "column",
        backgroundColor: "background.default",
      }}
    >
      <Stack
        direction="row"
        sx={{ alignItems: "center", justifyContent: "space-between", p: 2 }}
      >
        <Wordmark />
        <ThemeToggle />
      </Stack>

      <Box sx={{ flexGrow: 1, display: "grid", placeItems: "center", px: 3, pb: 12 }}>
        <Box sx={{ width: "100%", maxWidth: 320 }}>
          <Typography sx={{ mb: 3, fontSize: "0.875rem", color: "text.secondary" }}>
            {mode === "setup" ? "请设置管理员账户。" : "请登录以继续。"}
          </Typography>

          <form onSubmit={handleSubmit}>
            <Stack spacing={2}>
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
                disabled={busy}
                startIcon={busy ? <CircularProgress size={14} color="inherit" /> : undefined}
                sx={{ mt: 0.5 }}
              >
                {mode === "setup" ? "创建并进入" : "登录"}
              </Button>
            </Stack>
          </form>
        </Box>
      </Box>
    </Box>
  );
}
