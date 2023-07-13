<template>
  <h1>Nuxt3</h1>
  <div ref="container"></div>
</template>

<script>
import { ref, onMounted, onBeforeUnmount } from "vue";
import Button from "../../common-ui/dist/Button.svelte";

export default {
  setup() {
    const container = ref(null);
    let buttonInstance = null;

    onMounted(async () => {
      await nextTick();
      if (container.value) {
        buttonInstance = new Button({
          target: container.value,
          props: {
            label: "svelte button",
          },
        });
      }
    });

    onBeforeUnmount(() => {
      if (buttonInstance) {
        buttonInstance.$destroy();
      }
    });

    return {
      container,
    };
  },
};
</script>
