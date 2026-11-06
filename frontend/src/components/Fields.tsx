/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { useState } from "react";
import { Box, Button, Dialog, DialogActions, DialogContent, DialogTitle, Stack, TextField } from "@mui/material";

export type LeafFieldProps<T> = {
  label: string;
  value: T | null | undefined;
  onChange: (value: T) => void;
  helperText?: string;
  disabled?: boolean;
  placeholder?: string;
};

/** Text input that stores `null` for an empty value. */
export function NullableTextField({
  label,
  value,
  onChange,
  helperText,
  disabled,
  placeholder,
}: LeafFieldProps<string>) {
  return (
    <TextField
      fullWidth
      size="small"
      label={label}
      value={value ?? ""}
      placeholder={placeholder}
      helperText={helperText}
      disabled={disabled}
      onChange={(event) => onChange(event.target.value === "" ? "" : event.target.value)}
    />
  );
}

/** Numeric input that stores `null` when cleared. */
export function NullableNumberField({
  label,
  value,
  onChange,
  helperText,
  disabled,
  min = 0,
  max,
}: LeafFieldProps<number> & { min?: number; max?: number }) {
  return (
    <TextField
      fullWidth
      size="small"
      type="number"
      label={label}
      value={value ?? ""}
      helperText={helperText}
      disabled={disabled}
      slotProps={{ htmlInput: { min, max } }}
      onChange={(event) => {
        const raw = event.target.value;
        onChange(raw === "" ? 0 : Number(raw));
      }}
    />
  );
}

/** Multi-line editor opened from a summary row. */
export function StringListEditor({
  label,
  value,
  onChange,
  helperText,
  placeholder,
  separator = ",",
}: LeafFieldProps<string[]> & { separator?: string }) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState("");

  return (
    <>
      <Button
        variant="outlined"
        size="small"
        fullWidth
        onClick={() => {
          setDraft((value ?? []).join(separator));
          setOpen(true);
        }}
      >
        {label}（{value?.length ?? 0}）
      </Button>
      <Dialog open={open} onClose={() => setOpen(false)} fullWidth maxWidth="sm">
        <DialogTitle>{label}</DialogTitle>
        <DialogContent dividers>
          <Stack spacing={1}>
            <TextField
              fullWidth
              multiline
              minRows={4}
              value={draft}
              placeholder={placeholder}
              helperText={helperText ?? `使用 “${separator}” 分隔多个值`}
              onChange={(event) => setDraft(event.target.value)}
            />
          </Stack>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setOpen(false)}>取消</Button>
          <Button
            variant="contained"
            onClick={() => {
              onChange(
                draft
                  .split(separator)
                  .map((item) => item.trim())
                  .filter((item) => item.length > 0),
              );
              setOpen(false);
            }}
          >
            保存
          </Button>
        </DialogActions>
      </Dialog>
    </>
  );
}

/** Small layout helper so nested sections stay visually grouped. */
export function FieldRow({ children }: { children: React.ReactNode }) {
  return (
    <Box sx={{ display: "grid", gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr" }, gap: 2 }}>
      {children}
    </Box>
  );
}
