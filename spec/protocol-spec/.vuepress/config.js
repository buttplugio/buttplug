// .vuepress/config.js
module.exports = {
  themeConfig: {
    sidebar: [
      "/",
      "/architecture.md",
      "/messages.md",
      "/status.md",
      "/enumeration.md",
      "/generic.md",
      "/specific.md",
      "/deprecated.md",
    ]
  },
  plugins: [
    [
      "vuepress-plugin-matomo",
      {
        'siteId': 7,
        'trackerUrl': "https://matomo.nonpolynomial.com/"
      }
    ],
    "@vuepress/plugin-back-to-top"
  ]
};
