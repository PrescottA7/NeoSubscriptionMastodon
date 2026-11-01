import { Box, Breadcrumbs, Link as MuiLink, Typography } from "@mui/material";
import { Link as RouterLink } from "react-router-dom";
import { ChevronRight } from "@mui/icons-material";

export interface Crumb {
  label: string;
  /** Omit on the last crumb: the current page is not a link. */
  to?: string;
}

/**
 * Page heading with a breadcrumb trail above it.
 *
 * The trail is treated as real UI rather than a footnote: it is the fastest way
 * back up a level, so it sits directly above the title at a readable size.
 */
export default function PageHeader({
  crumbs,
  title,
  description,
  actions,
}: {
  crumbs: Crumb[];
  title: string;
  description?: string;
  actions?: React.ReactNode;
}) {
  return (
    <Box sx={{ mb: 3 }}>
      {crumbs.length > 0 && (
        <Breadcrumbs
          separator={<ChevronRight sx={{ fontSize: 14, color: "text.disabled" }} />}
          aria-label="面包屑导航"
          sx={{ mb: 1.25, "& .MuiBreadcrumbs-ol": { flexWrap: "nowrap" } }}
        >
          {crumbs.map((crumb, index) => {
            const last = index === crumbs.length - 1;
            if (last || !crumb.to) {
              return (
                <Typography
                  key={`${crumb.label}-${index}`}
                  variant="body2"
                  sx={{ color: "text.primary", fontWeight: 500, whiteSpace: "nowrap" }}
                  aria-current="page"
                >
                  {crumb.label}
                </Typography>
              );
            }
            return (
              <MuiLink
                key={`${crumb.label}-${index}`}
                component={RouterLink}
                to={crumb.to}
                underline="hover"
                variant="body2"
                sx={{ color: "text.secondary", whiteSpace: "nowrap" }}
              >
                {crumb.label}
              </MuiLink>
            );
          })}
        </Breadcrumbs>
      )}

      <Box
        sx={{
          display: "flex",
          gap: 2,
          alignItems: { xs: "flex-start", sm: "flex-end" },
          flexDirection: { xs: "column", sm: "row" },
          justifyContent: "space-between",
          borderBottom: "1px solid",
          borderColor: "divider",
          pb: 2,
        }}
      >
        <Box sx={{ minWidth: 0 }}>
          <Typography variant="h2" component="h1" sx={{ mb: description ? 0.75 : 0 }}>
            {title}
          </Typography>
          {description && (
            <Typography variant="body2" color="text.secondary" sx={{ maxWidth: "62ch" }}>
              {description}
            </Typography>
          )}
        </Box>
        {actions && (
          <Box sx={{ display: "flex", gap: 1, flexShrink: 0, flexWrap: "wrap" }}>{actions}</Box>
        )}
      </Box>
    </Box>
  );
}
