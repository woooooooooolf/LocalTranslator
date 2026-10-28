export default {
  server: {
    watch: {
      ignored: [
        "**/src-tauri/**",
        "**/runtime/**",
        "**/release/**",
        "**/data/**",
        "**/.cache/**",
      ],
    },
  },
};
