import { useEffect } from "react";
import { Navigate, Outlet, Route, Routes, useLocation } from "react-router-dom";
import { Box, CircularProgress } from "@mui/material";

import { setUnauthorizedHandler } from "./api";
import PageContainer from "./components/PageContainer";
import { noticeAdded } from "./store/noticesSlice";
import { bootstrapAuth, logoutAction, sessionExpired } from "./store/authSlice";
import { useAppDispatch, useAppSelector } from "./store/hooks";
import NodeEditorPage from "./pages/NodeEditorPage";
import NodesPage from "./pages/NodesPage";
import SignIn from "./pages/SignIn";
import SettingsPage from "./pages/SettingsPage";
import UsersPage from "./pages/UsersPage";
import { pageList } from "./pages/pages";

function ProtectedLayout() {
  const status = useAppSelector((state) => state.auth.status);
  const initialized = useAppSelector((state) => state.auth.initialized);
  const username = useAppSelector((state) => state.auth.admin?.username ?? "");
  const dispatch = useAppDispatch();
  const location = useLocation();

  if (!initialized || status === "loading") {
    return (
      <Box sx={{ minHeight: "100dvh", display: "grid", placeItems: "center" }}>
        <CircularProgress />
      </Box>
    );
  }

  if (status !== "authenticated") {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return (
    <PageContainer
      pages={pageList}
      username={username}
      onLogout={() => {
        void dispatch(logoutAction());
      }}
    >
      <Outlet />
    </PageContainer>
  );
}

export default function App() {
  const dispatch = useAppDispatch();
  const status = useAppSelector((state) => state.auth.status);
  const initialized = useAppSelector((state) => state.auth.initialized);
  const needsSetup = useAppSelector((state) => state.auth.needsSetup);

  useEffect(() => {
    void dispatch(bootstrapAuth());
  }, [dispatch]);

  useEffect(() => {
    setUnauthorizedHandler(() => {
      if (status === "authenticated") {
        dispatch(sessionExpired());
        dispatch(noticeAdded({ severity: "warning", message: "登录状态已失效，请重新登录" }));
      }
    });
    return () => setUnauthorizedHandler(null);
  }, [status, dispatch]);

  if (!initialized) {
    return (
      <Box sx={{ minHeight: "100dvh", display: "grid", placeItems: "center" }}>
        <CircularProgress />
      </Box>
    );
  }

  return (
    <Routes>
      <Route
        path="/login"
        element={
          needsSetup ? (
            <SignIn mode="setup" />
          ) : status === "authenticated" ? (
            <Navigate to="/" replace />
          ) : (
            <SignIn mode="login" />
          )
        }
      />
      <Route element={<ProtectedLayout />}>
        <Route path="/" element={<NodesPage />} />
        <Route path="/nodes" element={<NodesPage />} />
        <Route path="/nodes/new" element={<NodeEditorPage />} />
        <Route path="/nodes/:id" element={<NodeEditorPage />} />
        <Route path="/users" element={<UsersPage />} />
        <Route
          path="/settings"
          element={
            <SettingsPage
              onLogout={() => {
                void dispatch(logoutAction());
              }}
            />
          }
        />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
