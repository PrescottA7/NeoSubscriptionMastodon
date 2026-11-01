import { Box, Typography } from "@mui/material";

/** Square mark with a filled core: the only graphic the UI needs. */
export function Logomark({ size = 18 }: { size?: number }) {
  return (
    <Box
      aria-hidden
      sx={{
        width: size,
        height: size,
        flexShrink: 0,
        border: "1.5px solid",
        borderColor: "text.primary",
        borderRadius: "3px",
        position: "relative",
        "&::after": {
          content: '""',
          position: "absolute",
          inset: size / 4.5,
          backgroundColor: "text.primary",
          borderRadius: "1px",
        },
      }}
    />
  );
}

/** Logomark plus wordmark, used in the rail and on the sign-in screen. */
export function Wordmark({ subtitle }: { subtitle?: string }) {
  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
      <Logomark />
      <Box>
        <Typography sx={{ fontWeight: 600, fontSize: "0.9375rem", letterSpacing: "-0.01em" }}>
          NSM
        </Typography>
        {subtitle && (
          <Typography
            variant="caption"
            sx={{ display: "block", color: "text.disabled", fontSize: "0.625rem", mt: -0.25 }}
          >
            {subtitle}
          </Typography>
        )}
      </Box>
    </Box>
  );
}
