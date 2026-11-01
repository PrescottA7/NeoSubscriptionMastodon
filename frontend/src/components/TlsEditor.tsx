import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  FormControlLabel,
  MenuItem,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { ExpandMore } from "@mui/icons-material";

import { FieldRow } from "./Fields";
import type { Quic, Tls } from "../types";
import type { FieldErrors } from "../validation";

const FINGERPRINTS = [
  "chrome",
  "firefox",
  "safari",
  "ios",
  "android",
  "edge",
  "360",
  "qq",
  "random",
  "randomized",
];

const TLS_VERSIONS = ["1.0", "1.1", "1.2", "1.3"];

export function TlsSection({
  tls,
  onChange,
  requireTls = false,
  allowReality = true,
  disabledReality,
  errors = {},
}: {
  tls: Tls;
  onChange: (next: Tls) => void;
  requireTls?: boolean;
  allowReality?: boolean;
  disabledReality?: boolean;
  errors?: FieldErrors;
}) {
  const reality = tls.reality ?? {
    enabled: false,
    public_key: "",
    short_id: "",
    spider_x: "",
    mldsa65_verify: "",
  };

  function patch(next: Partial<Tls>) {
    onChange({ ...tls, ...next });
  }

  const field = (name: string) => ({
    error: Boolean(errors[name]),
    helperText: errors[name],
  });

  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Stack direction="row" spacing={2} sx={{ alignItems: "center", width: "100%" }}>
          <Typography sx={{ flexGrow: 1 }}>TLS / REALITY</Typography>
          <Typography variant="caption" color="text.secondary">
            {tls.reality?.enabled ? "REALITY" : tls.enabled ? "TLS" : "关闭"}
          </Typography>
        </Stack>
      </AccordionSummary>
      <AccordionDetails>
        <Stack spacing={2}>
          <Stack direction="row" spacing={3} sx={{ flexWrap: "wrap" }}>
            <FormControlLabel
              control={
                <Switch
                  checked={tls.enabled}
                  disabled={requireTls}
                  onChange={(event) => patch({ enabled: event.target.checked })}
                />
              }
              label="启用 TLS"
            />
            {allowReality && (
              <FormControlLabel
                control={
                  <Switch
                    checked={Boolean(reality.enabled)}
                    disabled={disabledReality}
                    onChange={(event) =>
                      patch({
                        reality: { ...reality, enabled: event.target.checked },
                        enabled: event.target.checked ? false : tls.enabled,
                      })
                    }
                  />
                }
                label="使用 REALITY"
              />
            )}
            <FormControlLabel
              control={
                <Switch
                  checked={tls.insecure}
                  onChange={(event) => patch({ insecure: event.target.checked })}
                />
              }
              label="跳过证书校验"
            />
          </Stack>

          <FieldRow>
            <TextField
              size="small"
              label="SNI / Server Name"
              value={tls.server_name ?? ""}
              onChange={(event) => patch({ server_name: event.target.value })}
              helperText="留空则使用节点地址"
            />
            <TextField
              size="small"
              select
              label="uTLS 指纹"
              value={tls.fingerprint ?? "chrome"}
              onChange={(event) => patch({ fingerprint: event.target.value })}
            >
              {FINGERPRINTS.map((value) => (
                <MenuItem key={value} value={value}>
                  {value}
                </MenuItem>
              ))}
            </TextField>
            <TextField
              size="small"
              label="ALPN"
              value={tls.alpn.join(",")}
              onChange={(event) =>
                patch({
                  alpn: event.target.value
                    .split(",")
                    .map((item) => item.trim())
                    .filter((item) => item.length > 0),
                })
              }
              helperText="逗号分隔，例如 h2,http/1.1"
            />
            <TextField
              size="small"
              label="证书 SHA256 锁定"
              value={tls.certificate_sha256 ?? ""}
              onChange={(event) => patch({ certificate_sha256: event.target.value })}
              helperText="对应 pcs / pinSHA256，可留空"
            />
            <TextField
              size="small"
              label="证书名称校验"
              value={tls.verify_peer_cert_by_name ?? ""}
              onChange={(event) => patch({ verify_peer_cert_by_name: event.target.value })}
              helperText="对应 vcn / verifyPeerCertByName"
            />
            <Stack direction="row" spacing={2}>
              <TextField
                size="small"
                select
                label="最低版本"
                value={tls.min_version ?? ""}
                onChange={(event) => patch({ min_version: event.target.value })}
                sx={{ flex: 1 }}
              >
                <MenuItem value="">默认</MenuItem>
                {TLS_VERSIONS.map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
              <TextField
                size="small"
                select
                label="最高版本"
                value={tls.max_version ?? ""}
                onChange={(event) => patch({ max_version: event.target.value })}
                sx={{ flex: 1 }}
              >
                <MenuItem value="">默认</MenuItem>
                {TLS_VERSIONS.map((value) => (
                  <MenuItem key={value} value={value}>
                    {value}
                  </MenuItem>
                ))}
              </TextField>
            </Stack>
          </FieldRow>

          {allowReality && reality.enabled && (
            <Stack spacing={2}>
              <Typography variant="subtitle2" color="text.secondary">
                REALITY 参数
              </Typography>
              <FieldRow>
                <TextField
                  size="small"
                  label="Public Key (pbk)"
                  value={reality.public_key}
                  onChange={(event) => patch({ reality: { ...reality, public_key: event.target.value } })}
                  required
                  {...field("params.tls.reality.public_key")}
                  helperText={
                    field("params.tls.reality.public_key").helperText ??
                    "服务端 xray x25519 生成的 Public key"
                  }
                />
                <TextField
                  size="small"
                  label="Short ID (sid)"
                  value={reality.short_id ?? ""}
                  onChange={(event) => patch({ reality: { ...reality, short_id: event.target.value } })}
                  helperText="最长 16 位十六进制"
                />
                <TextField
                  size="small"
                  label="Spider X (spx)"
                  value={reality.spider_x ?? ""}
                  onChange={(event) => patch({ reality: { ...reality, spider_x: event.target.value } })}
                  helperText="通常为 /"
                />
                <TextField
                  size="small"
                  label="ML-DSA-65 公钥 (pqv)"
                  value={reality.mldsa65_verify ?? ""}
                  onChange={(event) =>
                    patch({ reality: { ...reality, mldsa65_verify: event.target.value } })
                  }
                  helperText="可选，抗量子校验"
                />
              </FieldRow>
            </Stack>
          )}
        </Stack>
      </AccordionDetails>
    </Accordion>
  );
}

export function QuicSection({
  quic,
  onChange,
}: {
  quic: Quic;
  onChange: (next: Quic) => void;
}) {
  return (
    <Accordion disableGutters>
      <AccordionSummary expandIcon={<ExpandMore />}>
        <Typography sx={{ flexGrow: 1 }}>QUIC 参数</Typography>
      </AccordionSummary>
      <AccordionDetails>
        <FieldRow>
          <TextField
            size="small"
            type="number"
            label="初始流接收窗口"
            value={quic.initial_stream_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                initial_stream_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label="最大流接收窗口"
            value={quic.max_stream_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                max_stream_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label="初始连接接收窗口"
            value={quic.initial_connection_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                initial_connection_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            type="number"
            label="最大连接接收窗口"
            value={quic.max_connection_receive_window ?? ""}
            onChange={(event) =>
              onChange({
                ...quic,
                max_connection_receive_window:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          />
          <TextField
            size="small"
            label="Keep Alive"
            placeholder="10s"
            value={quic.keep_alive_period ?? ""}
            onChange={(event) => onChange({ ...quic, keep_alive_period: event.target.value })}
          />
          <FormControlLabel
            control={
              <Switch
                checked={quic.disable_path_mtu_discovery}
                onChange={(event) =>
                  onChange({ ...quic, disable_path_mtu_discovery: event.target.checked })
                }
              />
            }
            label="禁用路径 MTU 探测"
          />
        </FieldRow>
      </AccordionDetails>
    </Accordion>
  );
}
