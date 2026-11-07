/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Box, Button, Card, Chip, Divider, IconButton, Stack, Switch, Tooltip, Typography } from '@mui/material';
import { ContentCopy, Delete, Edit } from '@mui/icons-material';
import { Link as RouterLink } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

import { MONO_FONT } from '../theme';
import type { Node } from '../types';

interface NodeCardProps {
  node: Node;
  toggling: boolean;
  onToggle: (enabled: boolean) => void;
  onPreview: () => void;
  onDelete: () => void;
}

export default function NodeCard({ node, toggling, onToggle, onPreview, onDelete }: NodeCardProps) {
  const { t } = useTranslation();

  return (
    <Card
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        opacity: node.enabled ? 1 : 0.7,
        transition: 'border-color 160ms ease, opacity 160ms ease',
        '&:hover': { borderColor: 'primary.main', opacity: 1 },
      }}
    >
      <Stack direction='row' spacing={1.5} sx={{ p: 2, alignItems: 'center' }}>
        <Box
          aria-hidden
          sx={{
            width: 10,
            height: 10,
            flexShrink: 0,
            borderRadius: '50%',
            bgcolor: node.enabled ? 'success.main' : 'text.disabled',
          }}
        />
        <Typography noWrap title={node.name} sx={{ fontWeight: 600, flexGrow: 1, minWidth: 0 }}>
          {node.name}
        </Typography>
        <Chip
          size='small'
          variant='outlined'
          label={t(`protocol.${node.protocol}`, node.protocol)}
        />
      </Stack>

      <Box sx={{ px: 2, pb: 2, flexGrow: 1 }}>
        <Typography variant='caption' color='text.secondary' sx={{ display: 'block', mb: 0.5 }}>
          {t('nodes.address')}
        </Typography>
        <Typography
          noWrap
          title={`${node.server}:${node.port}`}
          sx={{
            fontFamily: MONO_FONT,
            fontSize: '0.75rem',
            px: 1,
            py: 0.75,
            borderRadius: 1,
            bgcolor: 'action.hover',
          }}
        >
          {node.server}:{node.port}
        </Typography>
      </Box>

      <Divider />
      <Stack direction='row' spacing={0.5} sx={{ px: 1.5, py: 0.75, alignItems: 'center' }}>
        <Switch
          size='small'
          checked={node.enabled}
          disabled={toggling}
          onChange={(event) => onToggle(event.target.checked)}
          inputProps={{ 'aria-label': t('nodes.enableToggle', { name: node.name }) }}
        />
        <Button
          size='small'
          variant='text'
          onClick={onPreview}
          sx={{ minWidth: 0, px: 1, fontSize: '0.75rem', color: 'text.secondary', '&:hover': { color: 'text.primary' } }}
        >
          {t('nodes.serverConfig')}
        </Button>
        <Box sx={{ flexGrow: 1 }} />
        <Tooltip title={t('nodes.editNode')}>
          <IconButton size='small' component={RouterLink} to={`/nodes/${node.id}`} aria-label={t('nodes.editAria')}>
            <Edit sx={{ fontSize: 16 }} />
          </IconButton>
        </Tooltip>
        <Tooltip title={t('nodes.duplicateNode')}>
          <IconButton
            size='small'
            component={RouterLink}
            to={`/nodes/new?from=${node.id}`}
            aria-label={t('nodes.duplicateAria')}
          >
            <ContentCopy sx={{ fontSize: 16 }} />
          </IconButton>
        </Tooltip>
        <Tooltip title={t('nodes.deleteNode')}>
          <IconButton size='small' onClick={onDelete} aria-label={t('nodes.deleteAria')}>
            <Delete sx={{ fontSize: 16 }} />
          </IconButton>
        </Tooltip>
      </Stack>
    </Card>
  );
}
