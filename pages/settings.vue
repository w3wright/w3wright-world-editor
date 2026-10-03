<script setup lang="ts">
/**
 * The settings screen.
 *
 * It needs no map, so unlike the view screens it renders its panel unconditionally — the panel
 * itself is what asks the backend for the settings and decides whether it has anything to show.
 */
import SettingsPanel from '~/components/SettingsPanel.vue'
</script>

<template>
  <SettingsPanel />
</template>
