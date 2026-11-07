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
import { useTranslation } from "react-i18next";

import { getApiErrorMessage } from "../api";
import ThemeToggle from "../components/ThemeToggle";
import LanguageSelect from "../components/LanguageSelect";
import { loginAction, setupAction } from "../store/authSlice";
import { useAppDispatch, useAppSelector } from "../store/hooks";
import { Wordmark } from "../components/Wordmark";

export default function SignIn({ mode }: { mode: "login" | "setup" }) {
  const { t } = useTranslation();
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
      setLocalError(
        getApiErrorMessage(err, t(mode === "setup" ? "auth.setupFailed" : "auth.loginFailed")),
      );
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
      <Stack direction="row" sx={{ alignItems: "center", justifyContent: "space-between", p: 2 }}>
        <Wordmark />
        <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
          <LanguageSelect />
          <ThemeToggle />
        </Stack>
      </Stack>

      <Box sx={{ flexGrow: 1, display: "grid", placeItems: "center", px: 3, pb: 12 }}>
        <Box
          sx={{
            width: "100%",
            maxWidth: 360,
            p: 3.5,
            border: "1px solid",
            borderColor: "divider",
            borderRadius: "8px",
            backgroundColor: "background.paper",
          }}
        >
          <Typography sx={{ mb: 3, fontSize: "0.875rem", color: "text.secondary" }}>
            {t(mode === "setup" ? "auth.setupMessage" : "auth.signInMessage")}
          </Typography>

          <form onSubmit={handleSubmit}>
            <Stack spacing={2}>
              <TextField
                label={t("auth.username")}
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                autoComplete="username"
                required
                fullWidth
                size="small"
              />
              <TextField
                label={t("auth.password")}
                type={show ? "text" : "password"}
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                autoComplete={mode === "setup" ? "new-password" : "current-password"}
                required
                fullWidth
                size="small"
                helperText={mode === "setup" ? t("auth.passwordHint") : undefined}
                slotProps={{
                  input: {
                    endAdornment: (
                      <InputAdornment position="end">
                        <IconButton
                          size="small"
                          onClick={() => setShow((value) => !value)}
                          aria-label={t(show ? "auth.hidePassword" : "auth.showPassword")}
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
                {t(mode === "setup" ? "auth.setupSubmit" : "auth.signIn")}
              </Button>
            </Stack>
          </form>
        </Box>
      </Box>
    </Box>
  );
}
