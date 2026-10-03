<script setup lang="ts">
/**
 * The welcome screen: where the app opens.
 *
 * It is a route rather than a state of the overview so that "nothing is open" has an address,
 * and so the first screen a user sees is not a panel explaining why it is empty.
 */
import WelcomeCard from '~/components/WelcomeCard.vue'

defineEmits<{ open: [] }>()
</script>

<template>
  <WelcomeCard @open="$emit('open')" />
</template>
