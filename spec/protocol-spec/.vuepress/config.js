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
  ],
  evergreen: true,
  title: "Buttplug Protocol Specification",
  description: "Specification for the Buttplug Intimate Hardware Control Protocol",
  head: [
    ['link', { rel: 'icon', href: '/buttplug.svg' }],
    ["meta", {property: "og:type", content:"website"}],
    ["meta", {property: "og:title", content:"Buttplug Protocol Specification"}],
    ["meta", {property: "og:url", content:"https://buttplug-spec.docs.buttplug.io"}],
    ["meta", {property: "og:site_name", content:"Buttplug Protocol Specification"}],
    ["meta", {property: "og:description", content:"Specification for the Buttplug Intimate Hardware Control Protocol."}],
    ["meta", {property: "og:locale", content:"default"}],
    ["meta", {property: "og:image", content:"https://buttplug-spec.docs.buttplug.io/buttplug-logo-opengraph.png"}],
    ["meta", {property: "og:updated_time", content:date}],
    ["meta", {name:"twitter:card", content:"summary"}],
    ["meta", {name:"twitter:title", content:"Buttplug Protocol Specification"}],
    ["meta", {name:"twitter:description", content:"Specification for the Buttplug Intimate Hardware Control Protocol."}],
    ["meta", {name:"twitter:image", content:"https://buttplug-spec.docs.buttplug.io/buttplug-logo-opengraph.png"}],
    ["meta", {name:"twitter:creator", content:"@buttplugio"}],
  ]
};
