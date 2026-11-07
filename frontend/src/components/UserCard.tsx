/*
 * Copyright (C) 2026 Prescott Anderson
 * SPDX-License-Identifier: AGPL-3.0-or-later
 *
 * This file is part of NSM (NeoSubscriptionMastodon), licensed under the
 * GNU Affero General Public License v3.0 or later. See the LICENSE file
 * in the project root for the full text.
 */

import { Box, Card, Chip, Divider, IconButton, Stack, Switch, TextField, Tooltip, Typography } from '@mui/material';
import { Check, Close, Delete, Edit, Link as LinkIcon } from '@mui/icons-material';
import { useTranslation } from 'react-i18next';

import { CopyButton } from './CopyButton';
import { MONO_FONT } from '../theme';
import type { User } from '../types';

interface UserCardProps {
  user: User;
  label: string;
  link: string;
  /** Everything before the slug, shown next to the inline editor. */
  prefix: string;
  /** Slug draft while this card is being renamed, otherwise null. */
  draft: string | null;
  onEdit: () => void;
  onDraftChange: (value: string) => void;
  onSave: () => void;
  onCancel: () => void;
  onToggle: () => void;
  onReset: () => void;
  onDelete: () => void;
}

export default function UserCard({
  user,
  label,
  link,
  prefix,
  draft,
  onEdit,
  onDraftChange,
  onSave,
  onCancel,
  onToggle,
  onReset,
  onDelete,
}: UserCardProps) {
  const { t } = useTranslation();
  const editing = draft !== null;

  return (
    <Card
      sx={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        opacity: user.enabled ? 1 : 0.7,
        transition: 'border-color 160ms ease, opacity 160ms ease',
        '&:hover': { borderColor: 'primary.main', opacity: 1 },
      }}
    >
      <Stack direction='row' spacing={1.5} sx={{ p: 2, alignItems: 'center' }}>
        <Box
          aria-hidden
          sx={{
            width: 40,
            height: 40,
            flexShrink: 0,
            display: 'grid',
            placeItems: 'center',
            borderRadius: '50%',
            fontWeight: 600,
            bgcolor: 'primary.main',
            color: 'primary.contrastText',
          }}
        >
          {(label.replace(/^#/, '').charAt(0) || '?').toUpperCase()}
        </Box>
        <Box sx={{ minWidth: 0, flexGrow: 1 }}>
          <Typography noWrap sx={{ fontWeight: 600 }}>
            {label}
          </Typography>
          <Typography variant='caption' color='text.secondary'>
            {t('users.tableAccess')}: {user.access_count}
          </Typography>
        </Box>
        <Chip
          size='small'
          color={user.enabled ? 'success' : 'default'}
          variant='outlined'
          label={t(user.enabled ? 'nodes.enabled' : 'nodes.disabled')}
        />
      </Stack>

      <Box sx={{ px: 2, pb: 2, flexGrow: 1 }}>
        <Typography variant='caption' color='text.secondary' sx={{ display: 'block', mb: 0.5 }}>
          {t('users.tableLink')}
        </Typography>
        {editing ? (
          <Stack direction='row' spacing={0.5} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
            <Typography sx={{ fontFamily: MONO_FONT, fontSize: '0.75rem', color: 'text.secondary' }}>
              {prefix}
            </Typography>
            <TextField
              size='small'
              value={draft}
              autoFocus
              onChange={(event) => onDraftChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  onSave();
                }
                if (event.key === 'Escape') {
                  onCancel();
                }
              }}
              sx={{ flexGrow: 1, minWidth: 120 }}
            />
            <IconButton size='small' aria-label={t('users.savePath')} onClick={onSave}>
              <Check fontSize='small' />
            </IconButton>
            <IconButton size='small' aria-label={t('common.cancel')} onClick={onCancel}>
              <Close fontSize='small' />
            </IconButton>
          </Stack>
        ) : (
          <Stack
            direction='row'
            spacing={0.5}
            sx={{ alignItems: 'center', p: 0.5, pl: 1, borderRadius: 1, bgcolor: 'action.hover' }}
          >
            <Typography
              noWrap
              title={user.subscription_path}
              sx={{ fontFamily: MONO_FONT, fontSize: '0.75rem', flexGrow: 1, minWidth: 0 }}
            >
              {user.subscription_path}
            </Typography>
            <Tooltip title={t('users.editPath')}>
              <IconButton size='small' aria-label={t('users.editPath')} onClick={onEdit}>
                <Edit fontSize='small' />
              </IconButton>
            </Tooltip>
            <CopyButton value={link} />
          </Stack>
        )}
      </Box>

      <Divider />
      <Stack direction='row' sx={{ px: 1.5, py: 0.75, alignItems: 'center' }}>
        <Switch
          size='small'
          checked={user.enabled}
          onChange={onToggle}
          inputProps={{ 'aria-label': t('users.tableStatus') }}
        />
        <Box sx={{ flexGrow: 1 }} />
        <Tooltip title={t('users.resetLink')}>
          <IconButton size='small' onClick={onReset} aria-label={t('users.resetLinkAria')}>
            <LinkIcon fontSize='small' />
          </IconButton>
        </Tooltip>
        <Tooltip title={t('common.delete')}>
          <IconButton size='small' color='error' onClick={onDelete} aria-label={t('users.deleteAria')}>
            <Delete fontSize='small' />
          </IconButton>
        </Tooltip>
      </Stack>
    </Card>
  );
}
