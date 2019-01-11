// .vuepress/components/mermaid.vue
// Taken from https://vuepress-examples.netlify.com/demos/diagrams/

<template>
  <div class="mermaid">
  <slot></slot>
  </div>
</template>

<script>
 export default {
   beforeMount() {
     import("mermaid/dist/mermaid").then(m => {
       m.initialize({
         startOnLoad: true
       });
       m.init();
     });
   }
 };
</script>
