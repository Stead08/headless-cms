// https://nuxt.com/docs/api/configuration/nuxt-config
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineNuxtConfig({
  devtools: { enabled: true },
  vite: {
    plugins: [svelte()],
  },
});
